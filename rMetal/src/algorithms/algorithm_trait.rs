use crate::solutions::solution_trait::Solution;
use crate::solution_set::solution_set_trait::SolutionSet;

/// Trait that defines the basic interface for all optimization algorithms.
pub trait Algorithm<T, S>
where
    S: Solution<T>,
    T: Clone,
{

    type SolutionSet: SolutionSet<T, S>;

    type Parameters;

    /// Runs the optimization algorithm.
    ///
    ///
    /// # Arguments
    /// 
    /// * `verbose` - Verbosity level of the output:
    ///   * `0` - No output
    ///   * `1` - Basic information (start, end)
    ///   * `>1` - Full debug information
    /// 
    fn run(&self, verbose: u8) -> impl SolutionSet<T,S>;

    fn validate_parameters(&self) -> bool;

    fn get_solution_set(&self) -> &Self::SolutionSet;

    fn get_parameters(&self) -> &Self::Parameters;

    fn set_parameters(&mut self, params: Self::Parameters);

    

}