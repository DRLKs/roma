use crate::problem::traits::Problem;
use crate::solution_set::traits::SolutionSet;
use crate::solution::traits::Dominance;

/// Trait that defines the basic interface for all optimization algorithms.
/// 
/// # Type Parameters
/// * `T` - Type of the solution variables
pub trait Algorithm<T, Q = f64>
where
    T: Clone,
    Q: Clone + Default + Dominance,
{
    type SolutionSet: SolutionSet<T, Q>;

    type Parameters;

    /// Runs the optimization algorithm on the given problem.
    ///
    /// # Arguments
    /// 
    /// * `problem` - The optimization problem to solve
    /// 
    fn run(&mut self, problem: &(impl Problem<T, Q> + Sync)) -> Self::SolutionSet;

    fn validate_parameters(&self) -> bool{
        true  // default
    }

    fn get_solution_set(&self) -> Option<&Self::SolutionSet>;
}