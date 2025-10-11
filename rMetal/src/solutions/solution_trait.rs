use std::cmp::Ordering;
use std::fmt::{Debug};
use crate::quality_indicator::quality_indicator_trait::QualityIndicator;

pub trait Solution<T: Clone> {
    /// Tipo para representar la calidad de la solución
    type Quality: QualityIndicator;
    
    fn new(solution_info: SolutionInfo<T>) -> Self;
    
    fn get_solution_info(&self) -> &SolutionInfo<T>;
    
    fn get_solution_info_mut(&mut self) -> &mut SolutionInfo<T>;
    
    fn get_number_of_variables(&self) -> usize {
        self.get_solution_info().get_variables().len()
    }
    
    fn get_variable(&self, index: usize) -> Option<&T> {
        self.get_solution_info().get_variables().get(index)
    }
    
    fn set_variable(&mut self, index: usize, value: T) -> Result<(), String> {
        if let Some(var) = self.get_solution_info_mut().get_variables_mut().get_mut(index) {
            *var = value;
            Ok(())
        } else {
            Err(format!("Index {} out of bounds", index))
        }
    }
    
    fn copy(&self) -> Self where Self: Sized {
        Self::new(self.get_solution_info().clone())
    }

    fn get_quality(&self) -> Option<&Self::Quality>;

    fn set_quality(&mut self, quality: Self::Quality);

    fn is_valid(&self) -> bool {
        true // Por defecto, todas las soluciones son válidas
    }
    
    fn dominates(&self, other: &Self) -> bool 
    where 
        <Self::Quality as QualityIndicator>::Fitness: PartialOrd 
    {
        let result: Option<Ordering>  = self.compare(other);
        
        if result.is_some() && result.unwrap() == Ordering::Greater {
            true
        }else{
            false
        }
        
    }

    fn compare(&self, other: &Self) -> Option<Ordering> {
        if let (Some(q1), Some(q2)) = (self.get_quality(), other.get_quality()) {
            q1.compare(q2)
        } else {
            None
        }
    }
}

pub trait SolutionBuilder<T: Clone> {
    type Solution: Solution<T>;
    
    fn build(solution_info: SolutionInfo<T>) -> Self::Solution;
    
    fn build_from_variables(variables: Vec<T>) -> Self::Solution {
        Self::build(SolutionInfo::new(variables))
    }
}

#[derive(Clone, Debug)]
pub struct SolutionInfo<T>{
    variables: Vec<T>
}

impl<T> SolutionInfo<T> {
    pub fn new(variables: Vec<T>) -> Self {
        SolutionInfo { variables }
    }
    
    pub fn get_variables(&self) -> &Vec<T> {
        &self.variables
    }
    
    pub fn get_variables_mut(&mut self) -> &mut Vec<T> {
        &mut self.variables
    }
}