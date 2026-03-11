use crate::experiment::traits::ExperimentalCase;
use crate::problem::traits::Problem;
use crate::solution::traits::Dominance;
use std::cmp::Ordering;

use super::report::{
    ExperimentFailure,
    ExperimentReport,
    ExperimentRunResult,
    ExperimentSummary,
    Objective
};
use super::utils::{derive_seed, mean, variance, best_and_worst};

pub struct Experiment<T, Q, P>
where
    T: Clone + Send + 'static,
    Q: Clone + Default + Dominance + Send + 'static + Copy + Into<f64>,
    P: Problem<T, Q> + Sync,
{
    problem: P,
    runs: usize,
    base_seed: u64,
    objective: Objective,
    cases: Vec<Box<dyn ExperimentalCase<T, Q, P>>>,
}

impl<T, Q, P> Experiment<T, Q, P>
where
    T: Clone + Send + 'static,
    Q: Clone + Default + Dominance + Send + 'static + Copy + Into<f64>,
    P: Problem<T, Q> + Sync,
{
    pub fn new(problem: P) -> Self {
        Self {
            problem,
            runs: 30,
            base_seed: 42,
            objective: Objective::Maximize,
            cases: Vec::new(),
        }
    }

    pub fn with_runs(mut self, runs: usize) -> Self {
        self.runs = runs.max(1);
        self
    }

    pub fn with_base_seed(mut self, seed: u64) -> Self {
        self.base_seed = seed;
        self
    }

    pub fn with_objective(mut self, objective: Objective) -> Self {
        self.objective = objective;
        self
    }

    pub fn add_case(mut self, case: impl ExperimentalCase<T, Q, P> + 'static) -> Self {
        self.cases.push(Box::new(case));
        self
    }

    pub fn execute(&self) -> Result<ExperimentReport, String> {
        if self.cases.is_empty() {
            return Err("experiment has no algorithms/configurations to execute".to_string());
        }

        let mut run_results = Vec::<ExperimentRunResult>::new();
        let mut failures = Vec::<ExperimentFailure>::new();
        let problem_name = self.problem.get_problem_description();

        for case in &self.cases {
            let case_name = case.case_name();
            let algorithm_name = case.algorithm_name().to_string();

            for run_index in 0..self.runs {
                let seed = derive_seed(
                    self.base_seed,
                    &algorithm_name,
                    &case_name,
                    &problem_name,
                    run_index as u64,
                );

                match case.run(&self.problem, seed) {
                    Ok(solution_set) => {
                        if let Some(best_value) = solution_set.best_solution_value() {
                            run_results.push(ExperimentRunResult {
                                algorithm_name: algorithm_name.clone(),
                                case_name: case_name.clone(),
                                run_index,
                                seed,
                                best_value,
                            });
                        } else {
                            failures.push(ExperimentFailure {
                                algorithm_name: algorithm_name.clone(),
                                case_name: case_name.clone(),
                                run_index,
                                seed,
                                error: "algorithm returned an empty solution set".to_string(),
                            });
                        }
                    }
                    Err(error) => {
                        failures.push(ExperimentFailure {
                            algorithm_name: algorithm_name.clone(),
                            case_name: case_name.clone(),
                            run_index,
                            seed,
                            error,
                        });
                    }
                }
            }
        }

        let mut summaries = Vec::new();
        for case in &self.cases {
            let case_name = case.case_name();
            let algorithm_name = case.algorithm_name().to_string();

            let mut values: Vec<f64> = run_results
                .iter()
                .filter(|r| r.algorithm_name == algorithm_name && r.case_name == case_name)
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
                algorithm_name,
                case_name,
                parameters_text: case.parameters_as_text(),
                runs_ok,
                best,
                mean,
                worst,
                std_dev: variance.sqrt(),
            });

        }

        summaries.sort_by(|a, b| {
            let ord = match self.objective {
                Objective::Maximize => b.best.partial_cmp(&a.best).unwrap_or(Ordering::Equal),
                Objective::Minimize => a.best.partial_cmp(&b.best).unwrap_or(Ordering::Equal),
            };

            if ord == Ordering::Equal {
                a.case_name.cmp(&b.case_name)
            } else {
                ord
            }
        });

        Ok(ExperimentReport {
            objective: self.objective,
            runs_per_case: self.runs,
            run_results,
            failures,
            summaries,
        })
    }
}
