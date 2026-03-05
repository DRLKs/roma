//! Core solution abstractions and builders.
//!
//! This module provides a generic `Solution<T, Q>` abstraction and
//! convenience builders for common variable types.

pub(crate) mod traits;
pub(crate) mod implementations;


pub use traits::ParetoCrowdingDistanceQuality;
pub use traits::Dominance;
pub use traits::ScalarDominanceDirection;
pub use traits::scalar_dominance_direction;
pub use traits::set_scalar_dominance_direction;
pub use implementations::binary_solution::BinarySolutionBuilder;
pub use implementations::real_solution::RealSolutionBuilder;
pub use implementations::pareto_crowding_solution::MultiObjectiveRealSolutionBuilder;
pub use implementations::pareto_crowding_solution::MultiObjectiveVectorRealSolutionBuilder;
pub use implementations::string_solution::StringSolutionBuilder;

/// Generic optimization solution.
///
/// # Type Parameters
/// - `T`: variable type.
/// - `Q`: quality payload type (defaults to `f64`).
///
/// # Notes
/// This type is intentionally generic over `Q` so algorithms can decide how to
/// store fitness/quality information:
/// - single-objective: `f64` (default)
/// - multi-objective with rank/crowding metadata: `ParetoCrowdingDistanceQuality`
/// - custom metadata-rich payloads
#[derive(Clone, Debug)]
pub struct Solution<T, Q = f64> {
    pub variables: Vec<T>,
    /// Optional cached quality payload.
    ///
    /// This value is expected to be updated by problem evaluation and invalidated
    /// whenever variables change.
    /// For scalar optimization this is usually `f64`.
    /// For vector-based multi-objective optimization this can be `Vec<f64>`.
    /// For metadata-rich workflows this can be a custom type.
    pub quality: Option<Q>,
}

impl<T, Q> Solution<T, Q> {
    /// Creates a solution with variables and no quality assigned.
    pub fn new(variables: Vec<T>) -> Self {
        Self {
            variables,
            quality: None,
        }
    }

    /// Returns an immutable view of decision variables.
    pub fn variables(&self) -> &[T] {
        &self.variables
    }

    /// Returns a mutable view of decision variables.
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

    /// Invalidates the quality cache.
    pub fn invalidate(&mut self) {
        self.quality = None;
    }
}

impl<T, Q> Solution<T, Q>
where
    Q: traits::Dominance,
{
    /// Returns `true` when this solution dominates `other` according to
    /// the quality-cache dominance semantics.
    ///
    /// If any quality cache is missing, returns `false`.
    pub fn dominates(&self, other: &Self) -> bool {
        match (&self.quality, &other.quality) {
            (Some(a), Some(b)) => a.dominates(b),
            _ => false,
        }
    }
}

impl<T> Solution<T, f64> {
    /// Returns the scalar quality value.
    ///
    /// If the quality cache is missing, this method returns `0.0`.
    pub fn quality_value(&self) -> f64 {
        self.quality.unwrap_or(0.0)
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

        s.invalidate();
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