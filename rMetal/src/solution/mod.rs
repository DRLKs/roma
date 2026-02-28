//! Core solution abstractions and builders.
//!
//! This module provides a generic `Solution<T, O>` abstraction and
//! convenience builders for common variable types.

pub(crate) mod traits;
pub(crate) mod implementations;

use crate::solution::traits::{QualityValue, ScalarQuality};

pub use traits::MultiObjectiveInfo;
pub use implementations::binary_solution::BinarySolutionBuilder;
pub use implementations::real_solution::RealSolutionBuilder;
pub use implementations::real_multiple_objective::MultiObjectiveRealSolutionBuilder;
pub use implementations::string_solution::StringSolutionBuilder;


/// Generic optimization solution.
///
/// # Type Parameters
/// - `T`: variable type.
/// - `O`: objective payload type.
#[derive(Clone, Debug)]
pub struct Solution<T, O = ScalarQuality> {
    pub variables: Vec<T>,
    pub objectives: Option<O>,
}

impl<T, O> Solution<T, O> {
    pub fn new(variables: Vec<T>) -> Self {
        Self {
            variables,
            objectives: None,
        }
    }

    pub fn variables(&self) -> &[T] {
        &self.variables
    }

    pub fn variables_mut(&mut self) -> &mut [T] {
        &mut self.variables
    }

    /// Returns the number of decision variables.
    pub fn num_variables(&self) -> usize {
        self.variables.len()
    }

    /// Compatibility alias used by legacy code.
    pub fn get_number_of_variables(&self) -> usize {
        self.num_variables()
    }

    /// Clone helper kept for compatibility with legacy algorithm code.
    pub fn copy(&self) -> Self
    where
        T: Clone,
        O: Clone,
    {
        self.clone()
    }

    pub fn get_variable(&self, index: usize) -> Option<&T> {
        self.variables.get(index)
    }

    pub fn get_variable_mut(&mut self, index: usize) -> Option<&mut T> {
        self.variables.get_mut(index)
    }

    pub fn invalidate(&mut self) {
        self.objectives = None;
    }
}

impl<T, O> Solution<T, O>
where
    O: QualityValue,
{
    /// Returns a scalar proxy value from the quality state.
    pub fn value(&self) -> f64 {
        self.objectives.as_ref().map(|o| o.value()).unwrap_or(0.0)
    }
}

impl<T> Solution<T, ScalarQuality> {
    /// Returns the scalar fitness value.
    pub fn fitness(&self) -> Option<f64> {
        self.objectives
    }
    
    /// Sets the scalar fitness value.
    pub fn set_fitness(&mut self, fitness: f64) {
        self.objectives = Some(fitness);
    }

}

fn finalize_scalar_solution<T>(variables: Vec<T>, fitness: Option<f64>) -> Solution<T> {
    let mut solution = Solution::new(variables);
    if let Some(fitness) = fitness {
        solution.set_fitness(fitness);
    }
    solution
}


fn apply_bounds(
    mut variables: Vec<f64>,
    lower_bounds: &Option<Vec<f64>>,
    upper_bounds: &Option<Vec<f64>>,
) -> Vec<f64> {
    if let (Some(lower), Some(upper)) = (lower_bounds, upper_bounds) {
        for i in 0..variables.len() {
            if i < lower.len() && i < upper.len() {
                variables[i] = variables[i].clamp(lower[i], upper[i]);
            }
        }
    }

    variables
}