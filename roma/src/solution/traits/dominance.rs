use super::ParetoCrowdingDistanceQuality;

/// Defines pairwise dominance between quality cache values.
///
/// The concrete semantics depend on each quality type.
pub trait Dominance {
    /// Returns `true` when `self` dominates `other`.
    fn dominates(&self, other: &Self) -> bool;
}

impl Dominance for f64 {
    /// Scalar dominance defaults to maximization semantics.
    ///
    /// Direction-aware scalar comparisons are handled explicitly in
    /// algorithm/objective code paths.
    fn dominates(&self, other: &Self) -> bool {
        self > other
    }
}

impl Dominance for ParetoCrowdingDistanceQuality {
    /// Pareto dominance based on objective vectors (minimization semantics).
    ///
    /// Rules:
    /// - `self` dominates `other` iff all objectives are <= and at least one
    ///   objective is strictly <,
    /// - rank and crowding-distance are **not** part of Pareto dominance;
    ///   they are selection metadata used as tie-breakers in algorithms
    ///   (e.g., NSGA-II tournament/replacement).
    /// - if objective vectors are missing, empty, or mismatched in length,
    ///   dominance is undefined and this returns `false`.
    ///
    /// In Pareto optimization, it is common that two solutions are
    /// non-dominated with respect to each other. In that case this method must
    /// return `false` in both directions.
    fn dominates(&self, other: &Self) -> bool {
        if self.objectives.is_empty()
            || other.objectives.is_empty()
            || self.objectives.len() != other.objectives.len()
        {
            return false;
        }

        let mut strictly_better_in_any = false;
        for (&a, &b) in self.objectives.iter().zip(other.objectives.iter()) {
            if a > b {
                return false;
            }
            if a < b {
                strictly_better_in_any = true;
            }
        }

        strictly_better_in_any
    }
}
