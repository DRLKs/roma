use crate::solutions::solution_trait::Solution;

/// Trait
/// * `S` - Solution
/// * `T` - Type of the solution value
pub trait ProblemTrait<S, T>
where
    S: Solution<T>,
    T: Clone,
{
    fn new() -> Self;

    fn evaluate(&self, solution: &mut S);

    fn set_problem_description(&self, description :String);

    fn get_problem_description(&self) -> String;

    fn create_solution(&self) -> S;

    fn neighbor(&self, solution: &S) -> S;
}