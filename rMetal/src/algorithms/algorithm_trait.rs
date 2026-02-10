use crate::solutions::solution_trait::Solution;
use crate::solution_set::solution_set_trait::SolutionSet;
use crate::problem::problem_trait::Problem;

/// Trait that defines the basic interface for all optimization algorithms.
/// 
/// # Type Parameters
/// * `T` - Type of the solution variables
/// * `S` - Solution type
/// * `P` - Problem type
pub trait Algorithm<T, S, P>
where
    S: Solution<T>,
    T: Clone,
    P: Problem<S, T>,
{
    type SolutionSet: SolutionSet<T, S>;

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
    fn run(&mut self, problem: &P, verbose: u8) -> Self::SolutionSet;

    fn validate_parameters(&self) -> bool;

    fn get_solution_set(&self) -> Option<&Self::SolutionSet>;

    fn get_parameters(&self) -> &Self::Parameters;

    fn set_parameters(&mut self, params: Self::Parameters);
}