use std::cmp::Ordering;
use std::fmt::{Debug};
use crate::quality_indicator::quality_indicator_trait::QualityIndicator;

/// Trait that defines the basic interface for all the solutions.
pub trait Solution<T: Clone> : Eq {
    type Quality: QualityIndicator;
    
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

    fn get_quality(&self) -> Option<&Self::Quality>;

    fn set_quality(&mut self, quality: Self::Quality);

    fn is_valid(&self) -> bool {
        true 
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