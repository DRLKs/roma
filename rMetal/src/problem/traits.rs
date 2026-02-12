use crate::solutions::traits::Solution;

/// Trait that defines the basic interface for optimization problems.
/// * `S` - Solution type
/// * `T` - Type of the solution variables
pub trait Problem<S, T>
where
    S: Solution<T>,
    T: Clone,
{
    fn new() -> Self;

    /// Evaluates a solution and updates its quality/fitness
    fn evaluate(&self, solution: &mut S);

    fn set_problem_description(&mut self, description: String);

    fn get_problem_description(&self) -> String;

    /// Creates a new random solution for this problem
    fn create_solution(&self) -> S;
}