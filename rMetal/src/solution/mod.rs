//! Core solution abstractions and builders.
//!
//! This module provides a generic `Solution<T, Q>` abstraction and
//! convenience builders for common variable types.

pub(crate) mod implementations;
pub(crate) mod traits;

pub use implementations::binary_solution::BinarySolutionBuilder;
pub use implementations::pareto_crowding_solution::MultiObjectiveRealSolutionBuilder;
pub use implementations::pareto_crowding_solution::MultiObjectiveVectorRealSolutionBuilder;
pub use implementations::permutation_solution::PermutationSolutionBuilder;
pub use implementations::real_solution::RealSolutionBuilder;
pub use implementations::string_solution::StringSolutionBuilder;
pub use traits::Dominance;
pub use traits::ParetoCrowdingDistanceQuality;

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
    variables: Vec<T>,
    /// Optional cached quality payload.
    ///
    /// This value is expected to be updated by problem evaluation and invalidated
    /// whenever variables change.
    /// For scalar optimization this is usually `f64`.
    /// For vector-based multi-objective optimization this can be `Vec<f64>`.
    /// For metadata-rich workflows this can be a custom type.
    quality: Option<Q>,
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
    ///
    /// # Cache invalidation
    /// Calling this method invalidates cached quality immediately.
    /// Any subsequent mutation through the returned slice means this solution
    /// is no longer considered evaluated and must be re-evaluated.
    pub fn variables_mut(&mut self) -> &mut [T] {
        self.invalidate();
        &mut self.variables
    }

    /// Replaces all decision variables and invalidates quality cache.
    ///
    /// After this call, quality is cleared because decision data changed.
    /// The solution must be re-evaluated.
    pub fn set_variables(&mut self, variables: Vec<T>) {
        self.variables = variables;
        self.invalidate();
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

    /// Returns a mutable reference to one decision variable by index.
    ///
    /// # Cache invalidation
    /// Calling this method invalidates cached quality immediately,
    /// even if `index` is out of bounds and the returned value is `None`.
    /// This keeps a strict invariant: mutable variable access implies stale
    /// quality cache and requires re-evaluation.
    pub fn get_variable_mut(&mut self, index: usize) -> Option<&mut T> {
        self.invalidate();
        self.variables.get_mut(index)
    }

    /// Replaces one variable and invalidates quality cache.
    ///
    /// Returns `true` when index is valid.
    ///
    /// When this returns `true`, decision data changed and quality is cleared.
    /// The solution must be re-evaluated.
    pub fn set_variable(&mut self, index: usize, value: T) -> bool {
        if let Some(variable) = self.variables.get_mut(index) {
            *variable = value;
            self.invalidate();
            true
        } else {
            false
        }
    }

    /// Swaps two variables and invalidates quality cache.
    ///
    /// Returns `true` when both indexes are valid.
    ///
    /// When this returns `true`, decision data changed and quality is cleared.
    /// The solution must be re-evaluated.
    pub fn swap_variables(&mut self, i: usize, j: usize) -> bool {
        if i < self.variables.len() && j < self.variables.len() {
            self.variables.swap(i, j);
            self.invalidate();
            true
        } else {
            false
        }
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
    ///
    /// Use this when decision variables are changed through external logic.
    /// After invalidation, the solution has no valid quality and must be
    /// re-evaluated by the problem.
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
    /// Returns the scalar quality value if present.
    pub fn try_quality_value(&self) -> Option<f64> {
        self.quality
    }

    /// Returns the scalar quality value.
    ///
    /// # Panics
    /// Panics when the quality cache is missing.
    ///
    /// In optimization hot paths, silently defaulting quality can hide invalid
    /// states (non-evaluated solutions participating in selection/ranking).
    /// Use [`try_quality_value`](Self::try_quality_value) when absence is expected.
    pub fn quality_value(&self) -> f64 {
        self.quality
            .expect("quality_value() called on a solution without evaluated quality")
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
        debug_assert_eq!(
            lower.len(),
            variables.len(),
            "lower_bounds length should match variables length"
        );
        debug_assert_eq!(
            upper.len(),
            variables.len(),
            "upper_bounds length should match variables length"
        );

        for ((value, &lo), &up) in variables.iter_mut().zip(lower.iter()).zip(upper.iter()) {
            *value = value.clamp(lo, up);
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

    #[test]
    fn try_quality_value_reflects_presence() {
        let mut s: Solution<i32> = Solution::new(vec![1, 2, 3]);
        assert_eq!(s.try_quality_value(), None);

        s.set_quality(1.25);
        assert_eq!(s.try_quality_value(), Some(1.25));
    }

    #[test]
    #[should_panic(expected = "quality_value() called on a solution without evaluated quality")]
    fn quality_value_panics_when_missing() {
        let s: Solution<i32> = Solution::new(vec![1, 2, 3]);
        let _ = s.quality_value();
    }
}
