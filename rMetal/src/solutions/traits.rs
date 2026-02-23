use std::cmp::Ordering;
use std::fmt::{Debug};
use crate::quality_indicator::traits::QualityIndicator;

/// Trait that defines the basic interface for all the solutions.
/// BasicSolution is generic and can return any type via ReturnType.
pub trait BasicSolution<T: Clone> : Eq {
    type ReturnType;

    fn new(variables: Vec<T>) -> Self;

    fn get_variables(&self) -> &Vec<T>;

    fn get_variables_mut(&mut self) -> &mut Vec<T>;

    fn get_number_of_variables(&self) -> usize {
        self.get_variables().len()
    }

    fn get_variable(&self, index: usize) -> Option<&T> {
        self.get_variables().get(index)
    }

    fn set_variable(&mut self, index: usize, value: T) -> Result<(), String> {
        if let Some(var) = self.get_variables_mut().get_mut(index) {
            *var = value;
            Ok(())
        } else {
            Err(format!("Index {} out of bounds", index))
        }
    }

    fn copy(&self) -> Self where Self: Sized {
        Self::new(self.get_variables().clone())
    }

    /// Get the return value for this solution
    fn get_return_value(&self) -> Option<&Self::ReturnType>;

    /// Set the return value for this solution
    fn set_return_value(&mut self, value: Self::ReturnType);

    fn is_valid(&self) -> bool {
        true
    }
}

/// Trait that defines the interface for solutions with quality indicators.
/// Solution extends the concept of BasicSolution by implementing a specific
/// return type that is a QualityIndicator.
pub trait Solution<T: Clone> : Eq {
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

    /// Get the quality indicator for this solution
    fn get_quality(&self) -> Option<&Self::Quality>;

    /// Set the quality indicator for this solution
    fn set_quality(&mut self, quality: Self::Quality);

    /// Get the fitness value as f64
    fn value(&self) -> f64;

    fn is_valid(&self) -> bool {
        true
    }

    /// Shows if the solution has a better quality indicator than the other
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

    /// Compares the quality of the solution with another
    fn compare(&self, other: &Self) -> Option<Ordering> {
        if let (Some(q1), Some(q2)) = (self.get_quality(), other.get_quality()) {
            q1.compare(q2)
        } else {
            None
        }
    }
}


#[derive(Clone, Debug, PartialEq)]
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