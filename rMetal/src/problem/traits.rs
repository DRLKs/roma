use crate::solution::Solution;
use crate::solution::traits::{QualityValue, ScalarQuality};
use crate::utils::random::Random;

/// Trait that defines the basic interface for optimization problems.
/// * `T` - Type of the solution variables
pub trait Problem<T, Q = ScalarQuality>
where
    T: Clone,
    Q: Clone + Default + QualityValue,
{
    fn new() -> Self;

    /// Evaluates a solution and updates its quality/fitness
    fn evaluate(&self, solution: &mut Solution<T, Q>);
    
    /// Creates a new random solution for this problem that serves as a starting point for the algorithm
    fn create_solution(&self, _rng: &mut Random) -> Solution<T, Q>;

    fn set_problem_description(&mut self, description: String);

    fn get_problem_description(&self) -> String;

    
}