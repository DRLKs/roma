use crate::ImprovementDirection;
use crate::experiment::traits::ExperimentalCase;
use crate::problem::traits::Problem;
use crate::solution::traits::Dominance;
use std::cmp::Ordering;
use std::fmt::Display;

use super::parallel::{ParallelConfig, parallel_collect_by_range};
use super::report::{ExperimentFailure, ExperimentReport, ExperimentRunResult, ExperimentSummary};
use super::utils::{best_and_worst, mean, variance};

/// Immutable metadata snapshot for each registered experiment case.
///
/// This avoids recomputing labels/parameters for every run execution.
#[derive(Clone)]
struct CaseMetadata {
    algorithm_name: String,
    case_name: String,
    parameters_text: String,
}

/// Aggregated output produced by one worker (or merged workers).
struct WorkerOutput {
    run_results: Vec<ExperimentRunResult>,
    failures: Vec<ExperimentFailure>,
}

/// Result classification for a single `case x run` job.
enum JobOutcome {
    Success(ExperimentRunResult),
    Failure(ExperimentFailure),
}

/// Experiment executor that runs multiple algorithm cases and summarizes results.
///
/// The executor supports parallel dispatch across `case x run` jobs while
/// preserving a simple builder-style API.
pub struct Experiment<T, Q, P>
where
    T: Clone + Send + 'static,
    Q: Clone + Default + Dominance + Send + 'static + Copy + Into<f64>,
    P: Problem<T, Q> + Sync,
{
    problem: P,
    runs: usize,
    parallel_threads: Option<usize>,
    objective: ImprovementDirection,
    cases: Vec<Box<dyn ExperimentalCase<T, Q, P>>>,
}

impl<T, Q, P> Experiment<T, Q, P>
where
    T: Clone + Send + 'static + Display,
    Q: Clone + Default + Dominance + Send + 'static + Copy + Into<f64> + Display,
    P: Problem<T, Q> + Sync,
{
    /// Builds and caches textual metadata for each case.
    fn collect_case_metadata(&self) -> Vec<CaseMetadata> {
        self.cases
            .iter()
            .map(|case| CaseMetadata {
                algorithm_name: case.algorithm_name().to_string(),
                case_name: case.case_name(),
                parameters_text: case.parameters_as_text(),
            })
            .collect()
    }

    /// Executes exactly one `(case_idx, run_index)` job.
    ///
    /// Returns either a successful scalar run result or a failure payload for
    /// report diagnostics.
    fn execute_single_job(
        &self,
        case_idx: usize,
        run_index: usize,
        metadata: &CaseMetadata,
    ) -> JobOutcome {
        let case = &self.cases[case_idx];

        match case.run(&self.problem) {
            Ok(solution_set) => {
                if let Some(best_value) = solution_set.best_solution_value() {
                    JobOutcome::Success(ExperimentRunResult {
                        algorithm_name: metadata.algorithm_name.clone(),
                        case_name: metadata.case_name.clone(),
                        run_index,
                        best_value,
                    })
                } else {
                    JobOutcome::Failure(ExperimentFailure {
                        algorithm_name: metadata.algorithm_name.clone(),
                        case_name: metadata.case_name.clone(),
                        run_index,
                        error: "algorithm returned an empty solution set".to_string(),
                    })
                }
            }
            Err(error) => JobOutcome::Failure(ExperimentFailure {
                algorithm_name: metadata.algorithm_name.clone(),
                case_name: metadata.case_name.clone(),
                run_index,
                error,
            }),
        }
    }

    /// Runs all jobs in parallel and merges worker-local buffers.
    fn execute_jobs_parallel(&self, case_metadata: &[CaseMetadata]) -> WorkerOutput {
        let total_jobs = self.cases.len().saturating_mul(self.runs);

        let worker_outputs = parallel_collect_by_range(
            total_jobs,
            ParallelConfig::new(self.parallel_threads).with_min_chunk_size(1),
            |_, range| {
                let mut run_results = Vec::<ExperimentRunResult>::new();
                let mut failures = Vec::<ExperimentFailure>::new();

                for flat_idx in range {
                    let case_idx = flat_idx / self.runs;
                    let run_index = flat_idx % self.runs;
                    let metadata = &case_metadata[case_idx];

                    match self.execute_single_job(case_idx, run_index, metadata) {
                        JobOutcome::Success(result) => run_results.push(result),
                        JobOutcome::Failure(failure) => failures.push(failure),
                    }
                }

                WorkerOutput {
                    run_results,
                    failures,
                }
            },
        );

        let mut run_results = Vec::<ExperimentRunResult>::new();
        let mut failures = Vec::<ExperimentFailure>::new();

        for mut worker in worker_outputs {
            run_results.append(&mut worker.run_results);
            failures.append(&mut worker.failures);
        }

        WorkerOutput {
            run_results,
            failures,
        }
    }

    /// Computes per-case statistics and ranking from raw run results.
    fn build_summaries(
        &self,
        case_metadata: &[CaseMetadata],
        run_results: &[ExperimentRunResult],
    ) -> Vec<ExperimentSummary> {
        let mut summaries = Vec::new();

        for metadata in case_metadata {
            let case_name = &metadata.case_name;
            let algorithm_name = &metadata.algorithm_name;

            let mut values: Vec<f64> = run_results
                .iter()
                .filter(|r| r.algorithm_name == *algorithm_name && r.case_name == *case_name)
                .map(|r| r.best_value)
                .collect();

            if values.is_empty() {
                continue;
            }

            values.sort_by(|a, b| a.partial_cmp(b).unwrap_or(std::cmp::Ordering::Equal));

            let runs_ok = values.len();
            let mean = mean(&values);
            let variance = variance(&values, mean);
            let (best, worst) = best_and_worst(&values, self.objective);

            summaries.push(ExperimentSummary {
                algorithm_name: algorithm_name.clone(),
                case_name: case_name.clone(),
                parameters_text: metadata.parameters_text.clone(),
                runs_ok,
                best,
                mean,
                worst,
                std_dev: variance.sqrt(),
            });
        }

        summaries.sort_by(|a, b| {
            let ord = match self.objective {
                ImprovementDirection::Maximize => {
                    b.best.partial_cmp(&a.best).unwrap_or(Ordering::Equal)
                }
                ImprovementDirection::Minimize => {
                    a.best.partial_cmp(&b.best).unwrap_or(Ordering::Equal)
                }
            };

            if ord == Ordering::Equal {
                a.case_name.cmp(&b.case_name)
            } else {
                ord
            }
        });

        summaries
    }

    /// Creates a new experiment with default settings.
    ///
    /// Defaults:
    /// - runs: `30`
    /// - objective: `Objective::Maximize`
    /// - threads: auto
    pub fn new(problem: P) -> Self {
        let objective = problem.get_improvement_direction().clone();
        Self {
            problem,
            runs: 30,
            parallel_threads: None,
            objective: objective,
            cases: Vec::new(),
        }
    }

    /// Sets number of runs per case (`runs >= 1`).
    pub fn with_runs(mut self, runs: usize) -> Self {
        self.runs = runs.max(1);
        self
    }

    /// Configures how many worker threads are used to execute case runs.
    ///
    /// `None` means auto (available parallelism).
    pub fn with_threads(mut self, threads: usize) -> Self {
        self.parallel_threads = Some(threads.max(1));
        self
    }

    /// Forces sequential execution of case runs.
    pub fn sequential(mut self) -> Self {
        self.parallel_threads = Some(1);
        self
    }

    /// Uses automatic parallelism based on available hardware threads.
    pub fn with_parallel(mut self) -> Self {
        self.parallel_threads = None;
        self
    }

    /// Registers one algorithm/configuration case in the experiment.
    pub fn add_case(mut self, case: impl ExperimentalCase<T, Q, P> + 'static) -> Self {
        self.cases.push(Box::new(case));
        self
    }

    /// Executes all registered cases and returns the consolidated report.
    ///
    /// Work scheduling is parallelized across all flattened jobs:
    /// `total_jobs = num_cases * runs_per_case`.
    pub fn execute(&self) -> Result<ExperimentReport, String> {
        if self.cases.is_empty() {
            return Err("experiment has no algorithms/configurations to execute".to_string());
        }

        let case_metadata = self.collect_case_metadata();
        let worker_output = self.execute_jobs_parallel(&case_metadata);
        let summaries = self.build_summaries(&case_metadata, &worker_output.run_results);

        Ok(ExperimentReport {
            objective: self.objective,
            runs_per_case: self.runs,
            run_results: worker_output.run_results,
            failures: worker_output.failures,
            summaries,
        })
    }
}
