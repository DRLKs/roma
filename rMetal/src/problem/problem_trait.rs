use crate::solutions::solution_trait::Solution;

pub trait ProblemTrait<T: Clone> {
    /// Tipo de fitness que produce este problema
    type Fitness: PartialOrd + Clone + std::fmt::Display + std::fmt::Debug;
    
    fn evaluate<S: Solution<T, Fitness = Self::Fitness>>(&self, solution: &mut S);

    
}