use crate::solution::traits::ParetoCrowdingDistanceQuality;
use crate::solution::{apply_bounds, Solution};

/// Convenience API for Pareto-and-crowding quality (`ParetoCrowdingDistanceQuality`).
impl<T> Solution<T, ParetoCrowdingDistanceQuality> {
    /// Returns the objective vector.
    pub fn objectives(&self) -> &[f64] {
        if let Some(info) = &self.value {
            &info.objectives
        } else {
            &[]
        }
    }

    /// Sets the objective vector.
    pub fn set_objectives(&mut self, objectives: Vec<f64>) {
        match &mut self.value {
            Some(info) => info.objectives = objectives,
            None => {
                self.value = Some(ParetoCrowdingDistanceQuality {
                    objectives,
                    rank: None,
                    crowding_distance: None,
                })
            }
        }
    }

    /// Returns the Pareto rank.
    pub fn rank(&self) -> Option<usize> {
        self.value.as_ref().and_then(|info| info.rank)
    }

    /// Sets the Pareto rank.
    pub fn set_rank(&mut self, rank: usize) {
        match &mut self.value {
            Some(info) => info.rank = Some(rank),
            None => {
                self.value = Some(ParetoCrowdingDistanceQuality {
                    objectives: vec![],
                    rank: Some(rank),
                    crowding_distance: None,
                })
            }
        }
    }

    /// Returns the crowding distance.
    pub fn crowding_distance(&self) -> Option<f64> {
        self.value
            .as_ref()
            .and_then(|info| info.crowding_distance)
    }

    /// Sets the crowding distance.
    pub fn set_crowding_distance(&mut self, distance: f64) {
        match &mut self.value {
            Some(info) => info.crowding_distance = Some(distance),
            None => {
                self.value = Some(ParetoCrowdingDistanceQuality {
                    objectives: vec![],
                    rank: None,
                    crowding_distance: Some(distance),
                })
            }
        }
    }

    /// Returns objective value by index.
    pub fn get_objective(&self, index: usize) -> Option<f64> {
        self.value
            .as_ref()
            .and_then(|info| info.objectives.get(index).copied())
    }

    /// Returns all objectives if present.
    pub fn get_objectives(&self) -> Option<&[f64]> {
        self.value.as_ref().and_then(|info| {
            if info.objectives.is_empty() {
                None
            } else {
                Some(info.objectives.as_slice())
            }
        })
    }
}

/// Builder for vector-based multi-objective real solutions (`Solution<f64, Vec<f64>>`).
pub struct MultiObjectiveVectorRealSolutionBuilder {
    variables: Vec<f64>,
    objectives: Vec<f64>,
    lower_bounds: Option<Vec<f64>>,
    upper_bounds: Option<Vec<f64>>,
}

impl MultiObjectiveVectorRealSolutionBuilder {
    /// Creates a builder from an existing variable vector.
    pub fn from_variables(variables: Vec<f64>) -> Self {
        Self {
            variables,
            objectives: vec![],
            lower_bounds: None,
            upper_bounds: None,
        }
    }

    /// Sets objective values.
    pub fn with_objectives(mut self, objectives: Vec<f64>) -> Self {
        self.objectives = objectives;
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

    /// Builds the final vector-based multi-objective real solution.
    pub fn build(self) -> Solution<f64, ParetoCrowdingDistanceQuality> {
        let lower_bounds = self.lower_bounds.clone();
        let upper_bounds = self.upper_bounds.clone();
        let variables = apply_bounds(self.variables, &self.lower_bounds, &self.upper_bounds);

        let mut solution: Solution<f64, ParetoCrowdingDistanceQuality> = Solution::new(variables);
        if let (Some(lower_bounds), Some(upper_bounds)) = (lower_bounds, upper_bounds) {
            solution.set_bounds(lower_bounds, upper_bounds);
        }
        if !self.objectives.is_empty() {
            solution.set_objectives(self.objectives);
        }
        solution
    }
}

/// Builder for Pareto-and-crowding multi-objective real solutions.
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

    /// Builds the final Pareto-and-crowding multi-objective real solution.
    pub fn build(self) -> Solution<f64, ParetoCrowdingDistanceQuality> {
        let lower_bounds = self.lower_bounds.clone();
        let upper_bounds = self.upper_bounds.clone();
        let variables = apply_bounds(self.variables, &self.lower_bounds, &self.upper_bounds);

        let mut solution: Solution<f64, ParetoCrowdingDistanceQuality> = Solution::new(variables);
        if let (Some(lower_bounds), Some(upper_bounds)) = (lower_bounds, upper_bounds) {
            solution.set_bounds(lower_bounds, upper_bounds);
        }
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
    use crate::solution::implementations::pareto_crowding_solution::{
        MultiObjectiveRealSolutionBuilder, MultiObjectiveVectorRealSolutionBuilder,
    };
    use crate::solution::ParetoCrowdingDistanceQuality;
    use crate::solution::Solution;

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
    fn pareto_dominates_when_all_objectives_no_worse_and_one_better() {
        let a = MultiObjectiveRealSolutionBuilder::from_variables(vec![0.0, 0.0])
            .with_objectives(vec![0.2, 0.3])
            .with_rank(5)
            .build();
        let b = MultiObjectiveRealSolutionBuilder::from_variables(vec![0.0, 0.0])
            .with_objectives(vec![0.3, 0.4])
            .with_rank(0)
            .build();

        assert!(a.dominates(&b));
        assert!(!b.dominates(&a));
    }

    #[test]
    fn pareto_non_dominated_when_trade_off_exists() {
        let a = MultiObjectiveRealSolutionBuilder::from_variables(vec![0.0, 0.0])
            .with_objectives(vec![0.2, 0.6])
            .with_rank(0)
            .with_crowding_distance(2.0)
            .build();
        let b = MultiObjectiveRealSolutionBuilder::from_variables(vec![0.0, 0.0])
            .with_objectives(vec![0.3, 0.5])
            .with_rank(0)
            .with_crowding_distance(1.0)
            .build();

        assert!(!a.dominates(&b));
        assert!(!b.dominates(&a));
    }

    #[test]
    fn dominates_returns_false_when_objectives_equal() {
        let a = MultiObjectiveRealSolutionBuilder::from_variables(vec![0.0, 0.0])
            .with_objectives(vec![0.4, 0.4])
            .with_rank(0)
            .with_crowding_distance(1.0)
            .build();
        let b = MultiObjectiveRealSolutionBuilder::from_variables(vec![0.0, 0.0])
            .with_objectives(vec![0.4, 0.4])
            .with_rank(0)
            .with_crowding_distance(1.0)
            .build();

        assert!(!a.dominates(&b));
        assert!(!b.dominates(&a));
    }

    #[test]
    fn vector_quality_builder_and_dominance_work() {
        let mut a = MultiObjectiveVectorRealSolutionBuilder::from_variables(vec![0.0, 0.0])
            .with_objectives(vec![0.2, 0.3])
            .build();
        let mut b = MultiObjectiveVectorRealSolutionBuilder::from_variables(vec![0.0, 0.0])
            .with_objectives(vec![0.3, 0.4])
            .build();

        a.set_rank(0); // metadata does not alter Pareto dominance semantics
        b.set_rank(1);

        assert!(a.dominates(&b));
        assert_eq!(a.get_objective(0), Some(0.2));

        let c: Solution<f64, ParetoCrowdingDistanceQuality> =
            MultiObjectiveVectorRealSolutionBuilder::from_variables(vec![0.0, 0.0]).build();
        assert_eq!(c.get_objectives(), None);
    }

    #[test]
    fn missing_objectives_do_not_dominate() {
        let mut a: Solution<f64, ParetoCrowdingDistanceQuality> = Solution::new(vec![0.0, 0.0]);
        a.set_crowding_distance(3.0);

        let b = MultiObjectiveRealSolutionBuilder::from_variables(vec![0.0, 0.0])
            .with_objectives(vec![0.1, 0.2])
            .with_rank(0)
            .with_crowding_distance(0.0)
            .build();

        assert!(!a.dominates(&b));
    }
}
