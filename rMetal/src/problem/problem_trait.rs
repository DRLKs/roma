use crate::solutions::solution_trait::Solution;

pub trait ProblemTrait<T: Clone> {
    /// Tipo de fitness que produce este problema
    
    fn evaluate<S: Solution<T>>(&self, solution: &mut S);

    
}