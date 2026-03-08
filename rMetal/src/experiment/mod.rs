use std::collections::HashMap;
use std::panic::{self, AssertUnwindSafe};
use std::sync::Arc;
use crate::observer::{ExperimentEvent, ExperimentObservable, ExperimentObserver};

pub mod traits;
mod utils;

pub use traits::ExperimentableAlgorithm;
use utils::derive_seed;

/// Optimization direction used to sort experiment comparisons.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Objective {
    Maximize,
    Minimize,
}

/// Named parameter set for one algorithm configuration to be evaluated.
#[derive(Debug, Clone)]
pub struct AlgorithmConfiguration<P> {
    pub name: String,
    pub parameters: P,
    pub attributes: HashMap<String, String>,
}

impl<P> AlgorithmConfiguration<P> {
    pub fn new(name: impl Into<String>, parameters: P) -> Self {
        Self {
            name: name.into(),
            parameters,
            attributes: HashMap::new(),
        }
    }

    pub fn with_attribute(mut self, key: impl Into<String>, value: impl Into<String>) -> Self {
        self.attributes.insert(key.into(), value.into());
        self
    }
}

/// One execution result of a specific algorithm/configuration on a problem instance.
#[derive(Debug, Clone)]
pub struct ExperimentRunResult {
    pub algorithm: String,
    pub configuration: String,
    pub problem: String,
    pub run_index: usize,
    pub seed: u64,
    pub best_value: f64,
}

/// Aggregated statistics for one (algorithm, configuration, problem) triplet.
#[derive(Debug, Clone)]
pub struct ExperimentSummary {
    pub algorithm: String,
    pub configuration: String,
    pub problem: String,
    pub runs: usize,
    pub best: f64,
    pub mean: f64,
    pub worst: f64,
    pub std_dev: f64,
}

/// Full report generated after executing an experiment.
#[derive(Debug, Clone)]
pub struct ExperimentReport {
    pub name: String,
    pub objective: Objective,
    pub runs_per_case: usize,
    pub run_results: Vec<ExperimentRunResult>,
    pub summaries: Vec<ExperimentSummary>,
}

impl ExperimentReport {
    /// Returns summaries sorted as a comparison table according to objective direction.
    pub fn comparison(&self) -> Vec<ExperimentSummary> {
        let mut sorted = self.summaries.clone();
        sorted.sort_by(|a, b| match self.objective {
            Objective::Maximize => b
                .mean
                .partial_cmp(&a.mean)
                .unwrap_or(std::cmp::Ordering::Equal),
            Objective::Minimize => a
                .mean
                .partial_cmp(&b.mean)
                .unwrap_or(std::cmp::Ordering::Equal),
        });
        sorted
    }
}

type CaseRunner = Box<dyn Fn(u64) -> f64 + Send + Sync>;

struct ExperimentCase {
    algorithm: String,
    configuration: String,
    problem: String,
    runner: CaseRunner,
}

/// Builder/executor for reproducible experiments.
///
/// Typical use:
/// - add one case per algorithm/configuration/problem combination,
/// - execute with `runs = N`,
/// - inspect summary/comparison metrics.
pub struct Experiment {
    name: String,
    runs: usize,
    base_seed: u64,
    objective: Objective,
    cases: Vec<ExperimentCase>,
    observers: Vec<Box<dyn ExperimentObserver>>,
}

impl Experiment {
    pub fn new(name: impl Into<String>) -> Self {
        Self {
            name: name.into(),
            runs: 30,
            base_seed: 42,
            objective: Objective::Maximize,
            cases: Vec::new(),
            observers: Vec::new(),
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

    pub fn add_case<F>(
        mut self,
        algorithm: impl Into<String>,
        configuration: impl Into<String>,
        problem: impl Into<String>,
        run_once: F,
    ) -> Self
    where
        F: Fn(u64) -> f64 + Send + Sync + 'static,
    {
        self.cases.push(ExperimentCase {
            algorithm: algorithm.into(),
            configuration: configuration.into(),
            problem: problem.into(),
            runner: Box::new(run_once),
        });
        self
    }

    /// Adds all configurations exposed by an experimentable algorithm.
    ///
    /// This method is intended for parameter/operator sweeps where the same
    /// algorithm is evaluated under multiple settings and deterministic seeds.
    pub fn add_experimentable_algorithm<A>(
        mut self,
        problem: impl Into<String>,
        algorithm: A,
    ) -> Self
    where
        A: ExperimentableAlgorithm + 'static,
    {
        let problem = problem.into();
        let algorithm_name = algorithm.algorithm_name().to_string();
        let algorithm = Arc::new(algorithm);

        for configuration in algorithm.configurations() {
            let configuration_name = format_configuration_label(&configuration);
            let parameters_for_runner = configuration.parameters.clone();
            let algorithm = Arc::clone(&algorithm);

            self.cases.push(ExperimentCase {
                algorithm: algorithm_name.clone(),
                configuration: configuration_name,
                problem: problem.clone(),
                runner: Box::new(move |seed| {
                    algorithm.run_with_parameters(&parameters_for_runner, seed)
                }),
            });
        }

        self
    }

    pub fn execute(&mut self) -> ExperimentReport {
        Self::notify_observers(&mut self.observers, ExperimentEvent::Start {
            name: self.name.clone(),
            objective: self.objective,
            runs_per_case: self.runs,
            total_cases: self.cases.len(),
        });

        let mut run_results = Vec::new();

        for case in &self.cases {
            let algorithm = case.algorithm.clone();
            let configuration = case.configuration.clone();
            let problem = case.problem.clone();

            Self::notify_observers(&mut self.observers, ExperimentEvent::CaseStarted {
                algorithm: algorithm.clone(),
                configuration: configuration.clone(),
                problem: problem.clone(),
            });

            for run_index in 0..self.runs {
                let seed = derive_seed(
                    self.base_seed,
                    &algorithm,
                    &configuration,
                    &problem,
                    run_index as u64,
                );
                let run_result = panic::catch_unwind(AssertUnwindSafe(|| (case.runner)(seed)));

                let best_value = match run_result {
                    Ok(value) => value,
                    Err(payload) => {
                        let message = panic_payload_to_string(payload);
                        Self::notify_observers(&mut self.observers, ExperimentEvent::Error {
                            algorithm: algorithm.clone(),
                            configuration: configuration.clone(),
                            problem: problem.clone(),
                            message,
                        });
                        break;
                    }
                };

                run_results.push(ExperimentRunResult {
                    algorithm: algorithm.clone(),
                    configuration: configuration.clone(),
                    problem: problem.clone(),
                    run_index,
                    seed,
                    best_value,
                });

                Self::notify_observers(&mut self.observers, ExperimentEvent::RunCompleted {
                    algorithm: algorithm.clone(),
                    configuration: configuration.clone(),
                    problem: problem.clone(),
                    run_index,
                    seed,
                    best_value,
                });
            }
        }

        let summaries = summarize(&run_results, self.objective);

        let report = ExperimentReport {
            name: self.name.clone(),
            objective: self.objective,
            runs_per_case: self.runs,
            run_results,
            summaries,
        };

        Self::notify_observers(&mut self.observers, ExperimentEvent::End {
            report: report.clone(),
        });

        for observer in self.observers.iter_mut() {
            observer.finalize();
        }

        report
    }

    fn notify_observers(observers: &mut Vec<Box<dyn ExperimentObserver>>, event: ExperimentEvent) {
        for observer in observers.iter_mut() {
            observer.update(&event);
        }
    }
}

fn panic_payload_to_string(payload: Box<dyn std::any::Any + Send>) -> String {
    match payload.downcast::<String>() {
        Ok(msg) => *msg,
        Err(payload) => match payload.downcast::<&'static str>() {
            Ok(msg) => (*msg).to_string(),
            Err(_) => "Unknown panic during experiment run".to_string(),
        },
    }
}

impl ExperimentObservable for Experiment {
    fn add_experiment_observer(&mut self, observer: Box<dyn ExperimentObserver>) {
        self.observers.push(observer);
    }

    fn clear_experiment_observers(&mut self) {
        self.observers.clear();
    }
}

fn summarize(results: &[ExperimentRunResult], objective: Objective) -> Vec<ExperimentSummary> {
    let mut groups: HashMap<(String, String, String), Vec<f64>> = HashMap::new();

    for r in results {
        groups
            .entry((
                r.algorithm.clone(),
                r.configuration.clone(),
                r.problem.clone(),
            ))
            .or_default()
            .push(r.best_value);
    }

    let mut summaries: Vec<ExperimentSummary> = groups
        .into_iter()
        .map(|((algorithm, configuration, problem), values)| {
            let runs = values.len();
            let mean = values.iter().sum::<f64>() / runs as f64;
            let variance = values
                .iter()
                .map(|v| {
                    let d = *v - mean;
                    d * d
                })
                .sum::<f64>()
                / runs as f64;
            let std_dev = variance.sqrt();

            let (best, worst) = match objective {
                Objective::Maximize => (
                    values
                        .iter()
                        .copied()
                        .fold(f64::NEG_INFINITY, f64::max),
                    values.iter().copied().fold(f64::INFINITY, f64::min),
                ),
                Objective::Minimize => (
                    values.iter().copied().fold(f64::INFINITY, f64::min),
                    values
                        .iter()
                        .copied()
                        .fold(f64::NEG_INFINITY, f64::max),
                ),
            };

            ExperimentSummary {
                algorithm,
                configuration,
                problem,
                runs,
                best,
                mean,
                worst,
                std_dev,
            }
        })
        .collect();

    summaries.sort_by(|a, b| match objective {
        Objective::Maximize => b
            .mean
            .partial_cmp(&a.mean)
            .unwrap_or(std::cmp::Ordering::Equal),
        Objective::Minimize => a
            .mean
            .partial_cmp(&b.mean)
            .unwrap_or(std::cmp::Ordering::Equal),
    });

    summaries
}

fn format_configuration_label<P>(configuration: &AlgorithmConfiguration<P>) -> String {
    if configuration.attributes.is_empty() {
        return configuration.name.clone();
    }

    let mut attributes: Vec<(&String, &String)> = configuration.attributes.iter().collect();
    attributes.sort_by(|(ka, _), (kb, _)| ka.cmp(kb));

    let details = attributes
        .into_iter()
        .map(|(k, v)| format!("{}={}", k, v))
        .collect::<Vec<_>>()
        .join(", ");

    format!("{} [{}]", configuration.name, details)
}

#[cfg(test)]
mod tests {
    use super::*;

    struct DummyExperimentable;

    impl ExperimentableAlgorithm for DummyExperimentable {
        type Parameters = f64;

        fn algorithm_name(&self) -> &str {
            "Dummy"
        }

        fn configurations(&self) -> Vec<AlgorithmConfiguration<Self::Parameters>> {
            vec![
                AlgorithmConfiguration::new("baseline", 1.0)
                    .with_attribute("mutation", "bit_flip")
                    .with_attribute("population", "20"),
                AlgorithmConfiguration::new("aggressive", 2.0)
                    .with_attribute("mutation", "bit_flip")
                    .with_attribute("population", "50"),
            ]
        }

        fn run_with_parameters(&self, parameters: &Self::Parameters, seed: u64) -> f64 {
            (seed % 10) as f64 * *parameters
        }
    }

    #[test]
    fn experiment_is_reproducible() {
        let build = || {
            Experiment::new("repro")
                .with_runs(5)
                .with_base_seed(123)
                .add_case("GA", "C1", "A", |seed| (seed % 1000) as f64)
                .add_case("GA", "C2", "A", |seed| ((seed % 1000) as f64) * 0.5)
                .execute()
        };

        let r1 = build();
        let r2 = build();

        assert_eq!(r1.run_results.len(), r2.run_results.len());
        for (a, b) in r1.run_results.iter().zip(r2.run_results.iter()) {
            assert_eq!(a.seed, b.seed);
            assert_eq!(a.best_value, b.best_value);
        }
    }

    #[test]
    fn comparison_orders_by_mean() {
        let report = Experiment::new("cmp")
            .with_runs(3)
            .with_base_seed(1)
            .with_objective(Objective::Maximize)
            .add_case("Alg", "C1", "A", |_| 10.0)
            .add_case("Alg", "C2", "A", |_| 20.0)
            .execute();

        let cmp = report.comparison();
        assert_eq!(cmp.first().map(|x| x.configuration.as_str()), Some("C2"));
    }

    #[test]
    fn add_experimentable_algorithm_expands_all_configurations() {
        let report = Experiment::new("exp")
            .with_runs(4)
            .with_base_seed(7)
            .add_experimentable_algorithm("P1", DummyExperimentable)
            .execute();

        assert_eq!(report.summaries.len(), 2);
        assert_eq!(report.run_results.len(), 8);

        let names: Vec<&str> = report
            .summaries
            .iter()
            .map(|s| s.configuration.as_str())
            .collect();

        assert!(names.iter().any(|n| n.starts_with("baseline")));
        assert!(names.iter().any(|n| n.starts_with("aggressive")));
    }
}
