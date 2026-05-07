use crate::problem::traits::Problem;
use crate::solution_set::traits::SolutionSet;
use std::fmt;

/// Generic key/value parameter descriptor used by experiment cases.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct CaseParameter {
    pub key: String,
    pub value: String,
}

impl CaseParameter {
    pub fn new(key: impl Into<String>, value: impl Into<String>) -> Self {
        Self {
            key: key.into(),
            value: value.into(),
        }
    }
}

impl fmt::Display for CaseParameter {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(f, "{}={}", self.key, self.value)
    }
}

/// One executable experiment case for a concrete algorithm/configuration.
///
/// The case is responsible for:
/// - exposing user-facing identifiers for reporting,
/// - creating/running the algorithm using its own configured parameters,
/// - returning one scalar best-value metric for aggregation.
pub trait ExperimentalCase<T, Q, P>: Send + Sync
where
    T: Clone + Send + 'static,
    Q: Clone + Default + Send + 'static + Copy + Into<f64>,
    P: Problem<T, Q> + Sync,
{
    fn algorithm_name(&self) -> &str;

    /// Human-readable identifier for this concrete configuration.
    fn case_name(&self) -> String {
        self.algorithm_name().to_string()
    }

    /// Returns generic parameter key/value pairs for reporting.
    fn parameters(&self) -> Vec<CaseParameter>;

    /// Helper to print all parameters in a single textual line.
    fn parameters_as_text(&self) -> String {
        self.parameters()
            .into_iter()
            .map(|p| p.to_string())
            .collect::<Vec<_>>()
            .join(", ")
    }

    /// Creates and executes the algorithm with its configured parameters.
    fn run(&self, problem: &P) -> Result<Box<dyn SolutionSet<T, Q>>, String>;
}
