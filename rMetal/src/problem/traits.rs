use crate::algorithms::runtime::ImprovementDirection;
use crate::solution::Solution;
use crate::utils::random::Random;

/// Trait that defines the basic interface for optimization problems.
/// * `T` - Type of the solution variables
pub trait Problem<T, Q = f64>
where
    T: Clone,
    Q: Clone + Default,
{
    fn new() -> Self;

    /// Evaluates a solution and updates its quality/fitness
    fn evaluate(&self, solution: &mut Solution<T, Q>);
    
    /// Creates a new random solution for this problem that serves as a starting point for the algorithm
    fn create_solution(&self, _rng: &mut Random) -> Solution<T, Q>;

    fn set_problem_description(&mut self, description: String);

    fn get_problem_description(&self) -> String;

    /// Returns the objective improvement direction for this problem.
    ///
    /// This is the single source of truth for scalar optimization direction
    /// in the framework. Algorithms and runtime termination consume this value
    fn get_improvement_direction(&self) -> ImprovementDirection;
}