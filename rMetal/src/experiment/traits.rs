use crate::experiment::AlgorithmConfiguration;

/// Contract for algorithms that can be automatically benchmarked in experiments.
///
/// Implementers expose a list of configurations (parameters/operators/attributes)
/// and define how to execute one run with a deterministic seed.
pub trait ExperimentableAlgorithm: Send + Sync {
    type Parameters: Clone + Send + Sync + 'static;

    /// Stable name used in experiment reports.
    fn algorithm_name(&self) -> &str;

    /// Parameter sweep to evaluate.
    fn configurations(&self) -> Vec<AlgorithmConfiguration<Self::Parameters>>;

    /// Executes one run for the given parameters and seed.
    ///
    /// Return value must be the scalar performance indicator used for ranking.
    fn run_with_parameters(&self, parameters: &Self::Parameters, seed: u64) -> f64;
}
