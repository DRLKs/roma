use std::collections::HashMap;

/// Optimization direction used to sort experiment comparisons.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Objective {
    Maximize,
    Minimize,
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

    pub fn execute(&self) -> ExperimentReport {
        let mut run_results = Vec::new();

        for case in &self.cases {
            for run_index in 0..self.runs {
                let seed = derive_seed(
                    self.base_seed,
                    &case.algorithm,
                    &case.configuration,
                    &case.problem,
                    run_index as u64,
                );
                let best_value = (case.runner)(seed);

                run_results.push(ExperimentRunResult {
                    algorithm: case.algorithm.clone(),
                    configuration: case.configuration.clone(),
                    problem: case.problem.clone(),
                    run_index,
                    seed,
                    best_value,
                });
            }
        }

        let summaries = summarize(&run_results, self.objective);

        ExperimentReport {
            name: self.name.clone(),
            objective: self.objective,
            runs_per_case: self.runs,
            run_results,
            summaries,
        }
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

fn derive_seed(base_seed: u64, a: &str, c: &str, p: &str, run: u64) -> u64 {
    let mut z = base_seed
        ^ hash64(a)
        ^ hash64(c).rotate_left(13)
        ^ hash64(p).rotate_left(27)
        ^ run.wrapping_mul(0x9E37_79B9_7F4A_7C15);

    z = (z ^ (z >> 30)).wrapping_mul(0xBF58_476D_1CE4_E5B9);
    z = (z ^ (z >> 27)).wrapping_mul(0x94D0_49BB_1331_11EB);
    z ^ (z >> 31)
}

fn hash64(s: &str) -> u64 {
    // FNV-1a 64-bit
    let mut hash: u64 = 0xcbf2_9ce4_8422_2325;
    for b in s.as_bytes() {
        hash ^= *b as u64;
        hash = hash.wrapping_mul(0x1000_0000_01b3);
    }
    hash
}

#[cfg(test)]
mod tests {
    use super::*;

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
}
