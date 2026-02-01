use crate::solutions::solution_trait::Solution;

/// Trait
pub trait ProblemTrait<S, T>
where
    S: Solution<T>,
    T: Clone,
{
    /// Tipo de fitness que produce este problema

    fn new() -> Self;

    fn evaluate(&self, solution: &mut S);

    fn get_problem_description(&self) -> String;

    fn create_solution(&self) -> S;

    fn neighbor(&self, solution: &S) -> S;
}