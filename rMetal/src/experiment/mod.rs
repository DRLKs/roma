use std::collections::HashMap;
use std::fs;
use std::path::Path;
use std::sync::Arc;
use std::time::{SystemTime, UNIX_EPOCH};
use crate::utils::json::{
    f64_to_json,
    json_array,
    json_field,
    json_object,
    json_string,
    u64_to_json,
    usize_to_json,
    JsonNodeSerializable
};

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

/// Failed execution entry for one specific run.
#[derive(Debug, Clone)]
pub struct ExperimentRunFailure {
    pub algorithm: String,
    pub configuration: String,
    pub problem: String,
    pub run_index: usize,
    pub seed: u64,
    pub message: String,
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
    pub failures: Vec<ExperimentRunFailure>,
    pub summaries: Vec<ExperimentSummary>,
}
impl JsonNodeSerializable for ExperimentRunResult {
    fn to_json_node(&self, indent_level: usize) -> String {
        json_object(
            &[
                json_field("algorithm", json_string(&self.algorithm)),
                json_field("configuration", json_string(&self.configuration)),
                json_field("problem", json_string(&self.problem)),
                json_field("run_index", usize_to_json(self.run_index)),
                json_field("seed", u64_to_json(self.seed)),
                json_field("best_value", f64_to_json(self.best_value)),
            ],
            indent_level,
        )
    }
}

impl JsonNodeSerializable for ExperimentRunFailure {
    fn to_json_node(&self, indent_level: usize) -> String {
        json_object(
            &[
                json_field("algorithm", json_string(&self.algorithm)),
                json_field("configuration", json_string(&self.configuration)),
                json_field("problem", json_string(&self.problem)),
                json_field("run_index", usize_to_json(self.run_index)),
                json_field("seed", u64_to_json(self.seed)),
                json_field("message", json_string(&self.message)),
            ],
            indent_level,
        )
    }
}

impl JsonNodeSerializable for ExperimentSummary {
    fn to_json_node(&self, indent_level: usize) -> String {
        json_object(
            &[
                json_field("algorithm", json_string(&self.algorithm)),
                json_field("configuration", json_string(&self.configuration)),
                json_field("problem", json_string(&self.problem)),
                json_field("runs", usize_to_json(self.runs)),
                json_field("best", f64_to_json(self.best)),
                json_field("mean", f64_to_json(self.mean)),
                json_field("worst", f64_to_json(self.worst)),
                json_field("std_dev", f64_to_json(self.std_dev)),
            ],
            indent_level,
        )
    }
}

impl JsonNodeSerializable for Objective {
    fn to_json_node(&self, _indent_level: usize) -> String {
        match self {
            Objective::Maximize => json_string("Maximize"),
            Objective::Minimize => json_string("Minimize"),
        }
    }
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

    /// Serializes the full report as a JSON string.
    pub fn to_json(&self) -> String {
        let generated_at_unix_ms = SystemTime::now()
            .duration_since(UNIX_EPOCH)
            .map(|d| d.as_millis())
            .unwrap_or(0);

        let successful_runs = self.run_results.len();
        let failed_runs = self.failures.len();
        let comparison = self.comparison();

        let run_results_json = self
            .run_results
            .iter()
            .map(|r| r.to_json_node(6))
            .collect::<Vec<_>>();

        let failures_json = self
            .failures
            .iter()
            .map(|f| f.to_json_node(6))
            .collect::<Vec<_>>();

        let summaries_json = self
            .summaries
            .iter()
            .map(|s| s.to_json_node(6))
            .collect::<Vec<_>>();

        let comparison_json = comparison
            .iter()
            .map(|s| s.to_json_node(6))
            .collect::<Vec<_>>();

        json_object(
            &[
                json_field(
                    "metadata",
                    json_object(
                        &[
                            json_field("schema_version", usize_to_json(1)),
                            json_field(
                                "generated_at_unix_ms",
                                generated_at_unix_ms.to_string(),
                            ),
                        ],
                        2,
                    ),
                ),
                json_field(
                    "report",
                    json_object(
                        &[
                            json_field("name", json_string(&self.name)),
                            json_field("objective", self.objective.to_json_node(0)),
                            json_field("runs_per_case", usize_to_json(self.runs_per_case)),
                            json_field(
                                "totals",
                                json_object(
                                    &[
                                        json_field(
                                            "successful_runs",
                                            usize_to_json(successful_runs),
                                        ),
                                        json_field("failed_runs", usize_to_json(failed_runs)),
                                        json_field(
                                            "summaries",
                                            usize_to_json(self.summaries.len()),
                                        ),
                                    ],
                                    4,
                                ),
                            ),
                            json_field("run_results", json_array(&run_results_json, 4)),
                            json_field("failures", json_array(&failures_json, 4)),
                            json_field("summaries", json_array(&summaries_json, 4)),
                            json_field("comparison", json_array(&comparison_json, 4)),
                        ],
                        2,
                    ),
                ),
            ],
            0,
        )
    }

    /// Writes the report JSON to disk.
    pub fn write_json<P: AsRef<Path>>(&self, output_path: P) -> Result<(), String> {
        let path = output_path.as_ref();
        if let Some(parent) = path.parent() {
            if !parent.as_os_str().is_empty() {
                fs::create_dir_all(parent)
                    .map_err(|e| format!("failed to create '{}': {}", parent.display(), e))?;
            }
        }

        fs::write(path, self.to_json())
            .map_err(|e| format!("failed to write '{}': {}", path.display(), e))
    }
}

type CaseRunner = Box<dyn Fn(u64) -> Result<f64, String> + Send + Sync>;

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
}

impl Experiment {
    pub fn new(name: impl Into<String>) -> Self {
        Self {
            name: name.into(),
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

    pub fn add_case<F>(
        self,
        algorithm: impl Into<String>,
        configuration: impl Into<String>,
        problem: impl Into<String>,
        run_once: F,
    ) -> Self
    where
        F: Fn(u64) -> f64 + Send + Sync + 'static,
    {
        self.add_case_result(
            algorithm,
            configuration,
            problem,
            move |seed| Ok(run_once(seed)),
        )
    }

    pub fn add_case_result<F>(
        mut self,
        algorithm: impl Into<String>,
        configuration: impl Into<String>,
        problem: impl Into<String>,
        run_once: F,
    ) -> Self
    where
        F: Fn(u64) -> Result<f64, String> + Send + Sync + 'static,
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
        let mut run_results = Vec::new();
        let mut failures = Vec::new();

        for case in &self.cases {
            let algorithm = case.algorithm.clone();
            let configuration = case.configuration.clone();
            let problem = case.problem.clone();

            for run_index in 0..self.runs {
                let seed = derive_seed(
                    self.base_seed,
                    &algorithm,
                    &configuration,
                    &problem,
                    run_index as u64,
                );
                let best_value = match (case.runner)(seed) {
                    Ok(value) => value,
                    Err(message) => {
                        failures.push(ExperimentRunFailure {
                            algorithm: algorithm.clone(),
                            configuration: configuration.clone(),
                            problem: problem.clone(),
                            run_index,
                            seed,
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
            }
        }

        let summaries = summarize(&run_results, self.objective);

        let report = ExperimentReport {
            name: self.name.clone(),
            objective: self.objective,
            runs_per_case: self.runs,
            run_results,
            failures,
            summaries,
        };

        report
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

        fn run_with_parameters(&self, parameters: &Self::Parameters, seed: u64) -> Result<f64, String> {
            Ok((seed % 10) as f64 * *parameters)
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
