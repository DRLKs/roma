use crate::solutions::solution_trait::Solution;

pub trait ProblemTrait<T: Clone> {
    /// Tipo de fitness que produce este problema

    fn new() -> Self;

    fn evaluate<S: Solution<T>>(&self, solution: &mut S);

    fn get_problem_description(&self) -> String;
    
}