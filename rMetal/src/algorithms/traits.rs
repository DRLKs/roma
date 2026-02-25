use crate::problem::traits::Problem;
use crate::solution_set::traits::SolutionSet;
use crate::solution::{QualityState, QualityValue, ScalarQuality};

/// Trait that defines the basic interface for all optimization algorithms.
/// 
/// # Type Parameters
/// * `T` - Type of the solution variables
pub trait Algorithm<T, Q = ScalarQuality>
where
    T: Clone,
    Q: Clone + Default + QualityState + QualityValue,
{
    type SolutionSet: SolutionSet<T, Q>;

    type Parameters;

    /// Runs the optimization algorithm on the given problem.
    ///
    /// # Arguments
    /// 
    /// * `problem` - The optimization problem to solve
    /// * `verbose` - Verbosity level of the output:
    ///   * `0` - No output
    ///   * `1` - Basic information (start, end)
    ///   * `>1` - Full debug information
    /// 
    fn run(&mut self, problem: &impl Problem<T, Q>) -> Self::SolutionSet;

    fn validate_parameters(&self) -> bool{
        true  // default
    }

    fn get_solution_set(&self) -> Option<&Self::SolutionSet>;

    fn get_parameters(&self) -> &Self::Parameters;

    fn set_parameters(&mut self, params: Self::Parameters);
}

pub trait AlgorithmParameters<T, S>{

}