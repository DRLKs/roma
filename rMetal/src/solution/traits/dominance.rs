use super::{scalar_dominance_direction, ParetoCrowdingDistanceQuality, ScalarDominanceDirection};

/// Defines pairwise dominance between quality cache values.
///
/// The concrete semantics depend on each quality type.
pub trait Dominance {
    /// Returns `true` when `self` dominates `other`.
    fn dominates(&self, other: &Self) -> bool;
}

impl Dominance for f64 {
    /// Scalar dominance semantics are configurable at runtime.
    fn dominates(&self, other: &Self) -> bool {
        match scalar_dominance_direction() {
            ScalarDominanceDirection::Maximize => self > other,
            ScalarDominanceDirection::Minimize => self < other,
        }
    }
}

impl Dominance for ParetoCrowdingDistanceQuality {
    /// Dominance based on rank/crowding-distance ordering.
    ///
    /// Rules:
    /// - lower rank dominates higher rank,
    /// - if ranks tie, larger crowding distance dominates,
    /// - if both tie (or missing), no dominance.
    fn dominates(&self, other: &Self) -> bool {
        let self_rank = self.rank.unwrap_or(usize::MAX);
        let other_rank = other.rank.unwrap_or(usize::MAX);

        if self_rank < other_rank {
            return true;
        }
        if self_rank > other_rank {
            return false;
        }

        let self_crowding = self.crowding_distance.unwrap_or(0.0);
        let other_crowding = other.crowding_distance.unwrap_or(0.0);
        self_crowding > other_crowding
    }
}
