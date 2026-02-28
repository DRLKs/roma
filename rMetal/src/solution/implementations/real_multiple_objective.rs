use crate::solution::{apply_bounds, Solution};
use crate::solution::traits::MultiObjectiveInfo;

/// Builder for multiple-objective solutions (`Solution<T, MultiObjectiveInfo>`).
impl<T> Solution<T, MultiObjectiveInfo> {
    /// Returns the objective vector.
    pub fn objectives(&self) -> &[f64] {
        if let Some(info) = &self.quality {
            &info.objectives
        } else {
            &[]
        }
    }

    /// Sets the objective vector.
    pub fn set_objectives(&mut self, objectives: Vec<f64>) {
        match &mut self.quality {
            Some(info) => info.objectives = objectives,
            None => {
                self.quality = Some(MultiObjectiveInfo {
                    objectives,
                    rank: None,
                    crowding_distance: None,
                })
            }
        }
    }

    /// Returns the Pareto rank.
    pub fn rank(&self) -> Option<usize> {
        self.quality.as_ref().and_then(|info| info.rank)
    }

    /// Sets the Pareto rank.
    pub fn set_rank(&mut self, rank: usize) {
        match &mut self.quality {
            Some(info) => info.rank = Some(rank),
            None => {
                self.quality = Some(MultiObjectiveInfo {
                    objectives: vec![],
                    rank: Some(rank),
                    crowding_distance: None,
                })
            }
        }
    }

    /// Returns the crowding distance.
    pub fn crowding_distance(&self) -> Option<f64> {
        self.quality
            .as_ref()
            .and_then(|info| info.crowding_distance)
    }

    /// Sets the crowding distance.
    pub fn set_crowding_distance(&mut self, distance: f64) {
        match &mut self.quality {
            Some(info) => info.crowding_distance = Some(distance),
            None => {
                self.quality = Some(MultiObjectiveInfo {
                    objectives: vec![],
                    rank: None,
                    crowding_distance: Some(distance),
                })
            }
        }
    }

    /// Returns objective value by index.
    pub fn get_objective(&self, index: usize) -> Option<f64> {
        self.quality
            .as_ref()
            .and_then(|info| info.objectives.get(index).copied())
    }

    /// Returns all objectives if present.
    pub fn get_objectives(&self) -> Option<&[f64]> {
        self.quality.as_ref().and_then(|info| {
            if info.objectives.is_empty() {
                None
            } else {
                Some(info.objectives.as_slice())
            }
        })
    }

    /// Returns true if this solution Pareto-dominates `other` (minimization).
    pub fn dominates(&self, other: &Self) -> bool {
        let my_objectives = match &self.quality {
            Some(info) => &info.objectives,
            None => return false,
        };
        let other_objectives = match &other.quality {
            Some(info) => &info.objectives,
            None => return false,
        };

        if my_objectives.len() != other_objectives.len() {
            return false;
        }

        let mut at_least_one_better = false;
        for (a, b) in my_objectives.iter().zip(other_objectives.iter()) {
            if a > b {
                return false;
            }
            if a < b {
                at_least_one_better = true;
            }
        }
        at_least_one_better
    }

}


/// Builder for multi-objective real solutions (`Solution<f64, MultiObjectiveInfo>`).
pub struct MultiObjectiveRealSolutionBuilder {
    variables: Vec<f64>,
    objectives: Vec<f64>,
    rank: Option<usize>,
    crowding_distance: Option<f64>,
    lower_bounds: Option<Vec<f64>>,
    upper_bounds: Option<Vec<f64>>,
}

impl MultiObjectiveRealSolutionBuilder {
    /// Creates a builder from an existing variable vector.
    pub fn from_variables(variables: Vec<f64>) -> Self {
        Self {
            variables,
            objectives: vec![],
            rank: None,
            crowding_distance: None,
            lower_bounds: None,
            upper_bounds: None,
        }
    }

    /// Sets objective values.
    pub fn with_objectives(mut self, objectives: Vec<f64>) -> Self {
        self.objectives = objectives;
        self
    }

    /// Sets Pareto rank.
    pub fn with_rank(mut self, rank: usize) -> Self {
        self.rank = Some(rank);
        self
    }

    /// Sets crowding distance.
    pub fn with_crowding_distance(mut self, distance: f64) -> Self {
        self.crowding_distance = Some(distance);
        self
    }

    /// Sets per-variable lower bounds.
    pub fn with_lower_bounds(mut self, bounds: Vec<f64>) -> Self {
        self.lower_bounds = Some(bounds);
        self
    }

    /// Sets per-variable upper bounds.
    pub fn with_upper_bounds(mut self, bounds: Vec<f64>) -> Self {
        self.upper_bounds = Some(bounds);
        self
    }

    /// Sets a uniform lower/upper bound for all variables.
    pub fn with_bounds(mut self, lower: f64, upper: f64) -> Self {
        let size = self.variables.len();
        self.lower_bounds = Some(vec![lower; size]);
        self.upper_bounds = Some(vec![upper; size]);
        self
    }

    /// Builds the final multi-objective real solution.
    pub fn build(self) -> Solution<f64, MultiObjectiveInfo> {
        let variables = apply_bounds(self.variables, &self.lower_bounds, &self.upper_bounds);

        let mut solution: Solution<f64, MultiObjectiveInfo> = Solution::new(variables);
        solution.set_objectives(self.objectives);
        if let Some(rank) = self.rank {
            solution.set_rank(rank);
        }
        if let Some(distance) = self.crowding_distance {
            solution.set_crowding_distance(distance);
        }
        solution
    }
}


#[cfg(test)]
mod tests {
    use crate::solution::implementations::real_multiple_objective::MultiObjectiveRealSolutionBuilder;
    use crate::solution::Solution;
    use crate::solution::MultiObjectiveInfo;

    #[test]
    fn test_multiobjective_builder_rank_and_crowding() {
        let solution = MultiObjectiveRealSolutionBuilder::from_variables(vec![1.0, 0.0, 1.0])
            .with_objectives(vec![0.2, 0.8])
            .with_rank(0)
            .with_crowding_distance(1.5)
            .build();

        assert_eq!(solution.objectives(), &[0.2, 0.8]);
        assert_eq!(solution.rank(), Some(0));
        assert_eq!(solution.crowding_distance(), Some(1.5));
    }

    #[test]
    fn dominates_returns_true_for_strictly_better_solution() {
        let a = MultiObjectiveRealSolutionBuilder::from_variables(vec![0.0, 0.0])
            .with_objectives(vec![0.2, 0.3])
            .build();
        let b = MultiObjectiveRealSolutionBuilder::from_variables(vec![0.0, 0.0])
            .with_objectives(vec![0.3, 0.4])
            .build();

        assert!(a.dominates(&b));
        assert!(!b.dominates(&a));
    }

    #[test]
    fn dominates_returns_false_for_equal_or_incomparable_solutions() {
        let equal_1 = MultiObjectiveRealSolutionBuilder::from_variables(vec![0.0, 0.0])
            .with_objectives(vec![0.5, 0.5])
            .build();
        let equal_2 = MultiObjectiveRealSolutionBuilder::from_variables(vec![0.0, 0.0])
            .with_objectives(vec![0.5, 0.5])
            .build();
        assert!(!equal_1.dominates(&equal_2));

        let x = MultiObjectiveRealSolutionBuilder::from_variables(vec![0.0, 0.0])
            .with_objectives(vec![0.2, 0.8])
            .build();
        let y = MultiObjectiveRealSolutionBuilder::from_variables(vec![0.0, 0.0])
            .with_objectives(vec![0.3, 0.6])
            .build();
        assert!(!x.dominates(&y));
        assert!(!y.dominates(&x));
    }

    #[test]
    fn dominates_returns_false_when_objective_lengths_differ() {
        let mut a: Solution<f64, MultiObjectiveInfo> = Solution::new(vec![0.0, 0.0]);
        a.set_objectives(vec![0.1, 0.2]);
        let mut b: Solution<f64, MultiObjectiveInfo> = Solution::new(vec![0.0, 0.0]);
        b.set_objectives(vec![0.1]);

        assert!(!a.dominates(&b));
    }

}