//! Core solution abstractions and builders.
//!
//! This module provides a generic `Solution<T, Q>` abstraction and
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
/// - `Q`: quality payload type.
#[derive(Clone, Debug)]
pub struct Solution<T, Q = ScalarQuality> {
    pub variables: Vec<T>,
    /// Optional quality payload.
    ///
    /// For scalar optimization this stores quality (`f64`).
    /// For multi-objective optimization this stores `MultiObjectiveInfo`.
    pub quality: Option<Q>,
}

impl<T, Q> Solution<T, Q> {
    pub fn new(variables: Vec<T>) -> Self {
        Self {
            variables,
            quality: None,
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

    /// Clone helper kept for compatibility with legacy algorithm code.
    pub fn copy(&self) -> Self
    where
        T: Clone,
        Q: Clone,
    {
        self.clone()
    }

    pub fn get_variable(&self, index: usize) -> Option<&T> {
        self.variables.get(index)
    }

    pub fn get_variable_mut(&mut self, index: usize) -> Option<&mut T> {
        self.variables.get_mut(index)
    }

    /// Returns quality payload if present.
    pub fn quality(&self) -> Option<&Q> {
        self.quality.as_ref()
    }

    /// Returns mutable quality payload if present.
    pub fn quality_mut(&mut self) -> Option<&mut Q> {
        self.quality.as_mut()
    }

    /// Replaces quality payload.
    pub fn set_quality(&mut self, quality: Q) {
        self.quality = Some(quality);
    }

    /// Returns true when quality payload is present.
    pub fn has_quality(&self) -> bool {
        self.quality.is_some()
    }

    /// Clears quality payload.
    pub fn clear_quality(&mut self) {
        self.quality = None;
    }

    pub fn invalidate(&mut self) {
        self.clear_quality();
    }
}

impl<T, Q> Solution<T, Q>
where
    Q: QualityValue,
{
    /// Returns a scalar proxy value from the quality state.
    ///
    /// If the quality payload is `None`, this method returns `0.0`.
    pub fn quality_value(&self) -> f64 {
        self.quality
            .as_ref()
            .map(|o| o.quality_value())
            .unwrap_or(0.0)
    }
}

fn finalize_scalar_solution<T>(variables: Vec<T>, quality: Option<f64>) -> Solution<T> {
    let mut solution = Solution::new(variables);
    if let Some(quality) = quality {
        solution.set_quality(quality);
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

#[cfg(test)]
mod tests {
    use super::Solution;

    #[test]
    fn quality_helpers_work_as_expected() {
        let mut s: Solution<i32> = Solution::new(vec![1, 2, 3]);
        assert!(!s.has_quality());
        assert_eq!(s.quality(), None);

        s.set_quality(4.0);
        assert!(s.has_quality());
        assert_eq!(s.quality().copied(), Some(4.0));

        s.clear_quality();
        assert!(!s.has_quality());
        assert_eq!(s.quality(), None);
    }

    #[test]
    fn invalidate_clears_quality() {
        let mut s: Solution<bool> = Solution::new(vec![true, false]);
        s.set_quality(10.0);
        assert_eq!(s.quality().copied(), Some(10.0));
        s.invalidate();
        assert_eq!(s.quality(), None);
    }
}