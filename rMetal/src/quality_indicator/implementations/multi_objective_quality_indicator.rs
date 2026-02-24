use crate::quality_indicator::traits::QualityIndicator;
use std::cmp::Ordering;

/// Multi-objective quality indicator that stores a vector of objective values. 
/// To decide who solution is the best uses:
/// - Pareto Rank 
/// - Crowding distance 
#[derive(Clone, Debug)]
pub struct MultiObjectiveQualityIndicator {
    objectives: Option<Vec<f64>>,
    rank: Option<usize>,              // Pareto rank
    crowding_distance: Option<f64>,   // Crowding distance for NSGA-II
}

impl MultiObjectiveQualityIndicator {
    pub fn new(objectives: Option<Vec<f64>>) -> Self {
        Self {
            objectives,
            rank: None,
            crowding_distance: None,
        }
    }

    pub fn with_rank(mut self, rank: usize) -> Self {
        self.rank = Some(rank);
        self
    }

    pub fn with_crowding_distance(mut self, distance: f64) -> Self {
        self.crowding_distance = Some(distance);
        self
    }

    pub fn get_objectives(&self) -> Option<&Vec<f64>> {
        self.objectives.as_ref()
    }

    pub fn get_objective(&self, index: usize) -> Option<f64> {
        self.objectives.as_ref()?.get(index).copied()
    }

    pub fn get_rank(&self) -> Option<usize> {
        self.rank
    }

    pub fn set_rank(&mut self, rank: usize) {
        self.rank = Some(rank);
    }

    pub fn get_crowding_distance(&self) -> Option<f64> {
        self.crowding_distance
    }

    pub fn set_crowding_distance(&mut self, distance: f64) {
        self.crowding_distance = Some(distance);
    }

    pub fn number_of_objectives(&self) -> usize {
        self.objectives.as_ref().map(|v| v.len()).unwrap_or(0)
    }

    /// Checks if this solution dominates another (Pareto dominance)
    /// Returns true if this solution is better or equal in all objectives
    /// and strictly better in at least one
    pub fn dominates(&self, other: &Self) -> bool {
        let self_objs = match &self.objectives {
            Some(objs) => objs,
            None => return false,
        };

        let other_objs = match &other.objectives {
            Some(objs) => objs,
            None => return true,
        };

        if self_objs.len() != other_objs.len() {
            return false;
        }

        let mut at_least_one_better = false;
        for i in 0..self_objs.len() {
            if self_objs[i] > other_objs[i] {
                // Worse in at least one objective
                return false;
            }
            if self_objs[i] < other_objs[i] {
                at_least_one_better = true;
            }
        }

        at_least_one_better
    }
}

impl QualityIndicator for MultiObjectiveQualityIndicator {
    type Fitness = Option<Vec<f64>>;

    fn invalidate(&mut self) {
        self.objectives = None;
        self.rank = None;
        self.crowding_distance = None;
    }

    fn get_fitness_indicator(&self) -> &Self::Fitness {
        &self.objectives
    }

    fn get_fitness_indicator_mut(&mut self) -> &mut Self::Fitness {
        &mut self.objectives
    }

    fn set_fitness_indicator(&mut self, fitness: Self::Fitness) {
        self.objectives = fitness;
    }

    /// For multi-objective, comparison is based on rank and crowding distance
    fn compare(&self, other: &Self) -> Option<Ordering> {
        // First compare by rank (lower is better)
        match (self.rank, other.rank) {
            (Some(r1), Some(r2)) => {
                match r1.cmp(&r2) {
                    Ordering::Equal => {
                        // Same rank, compare by crowding distance (higher is better)
                        match (self.crowding_distance, other.crowding_distance) {
                            (Some(d1), Some(d2)) => d2.partial_cmp(&d1),
                            (Some(_), None) => Some(Ordering::Less),
                            (None, Some(_)) => Some(Ordering::Greater),
                            (None, None) => Some(Ordering::Equal),
                        }
                    }
                    other_ordering => Some(other_ordering),
                }
            }
            (Some(_), None) => Some(Ordering::Less),
            (None, Some(_)) => Some(Ordering::Greater),
            (None, None) => Some(Ordering::Equal),
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_dominance() {
        let qi1 = MultiObjectiveQualityIndicator::new(Some(vec![1.0, 2.0]));
        let qi2 = MultiObjectiveQualityIndicator::new(Some(vec![2.0, 3.0]));
        let qi3 = MultiObjectiveQualityIndicator::new(Some(vec![1.0, 2.0]));

        assert!(qi1.dominates(&qi2)); // qi1 is better in both
        assert!(!qi2.dominates(&qi1)); // qi2 is worse
        assert!(!qi1.dominates(&qi3)); // equal, no dominance
    }

    #[test]
    fn test_partial_dominance() {
        let qi1 = MultiObjectiveQualityIndicator::new(Some(vec![1.0, 3.0]));
        let qi2 = MultiObjectiveQualityIndicator::new(Some(vec![2.0, 2.0]));

        assert!(!qi1.dominates(&qi2)); // qi1 better in obj2, worse in obj1
        assert!(!qi2.dominates(&qi1)); // qi2 better in obj1, worse in obj2
    }

    #[test]
    fn test_rank_comparison() {
        let mut qi1 = MultiObjectiveQualityIndicator::new(Some(vec![1.0, 2.0]));
        let mut qi2 = MultiObjectiveQualityIndicator::new(Some(vec![1.0, 2.0]));

        qi1.set_rank(0);
        qi2.set_rank(1);

        assert_eq!(qi1.compare(&qi2), Some(Ordering::Less));
        assert_eq!(qi2.compare(&qi1), Some(Ordering::Greater));
    }

    #[test]
    fn test_crowding_distance_comparison() {
        let mut qi1 = MultiObjectiveQualityIndicator::new(Some(vec![1.0, 2.0]));
        let mut qi2 = MultiObjectiveQualityIndicator::new(Some(vec![1.0, 2.0]));

        qi1.set_rank(0);
        qi2.set_rank(0);
        qi1.set_crowding_distance(1.0);
        qi2.set_crowding_distance(2.0);

        // Higher crowding distance is better
        assert_eq!(qi1.compare(&qi2), Some(Ordering::Greater));
        assert_eq!(qi2.compare(&qi1), Some(Ordering::Less));
    }
}
