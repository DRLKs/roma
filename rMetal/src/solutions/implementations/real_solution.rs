use crate::quality_indicator::implementations::multi_objective_quality_indicator::MultiObjectiveQualityIndicator;
use crate::quality_indicator::traits::QualityIndicator;
use crate::solutions::traits::{Solution, SolutionInfo};
use std::cmp::Ordering;

/// Real-valued solution for multi-objective optimization problems
#[derive(Clone)]
pub struct RealSolution {
    solution_info: SolutionInfo<f64>,
    quality: MultiObjectiveQualityIndicator,
}

impl RealSolution {
    /// Create a new solution with specified number of variables
    /// Variables are initialized to 0.0
    pub fn new_with_size(size: usize) -> Self {
        Self::new(SolutionInfo::new(vec![0.0; size]))
    }

    /// Create a new solution with specified bounds for each variable
    pub fn new_with_bounds(lower_bounds: Vec<f64>, upper_bounds: Vec<f64>) -> Self {
        assert_eq!(
            lower_bounds.len(),
            upper_bounds.len(),
            "Bounds must have same length"
        );
        Self::new(SolutionInfo::new(lower_bounds))
    }

    /// Get all objective values
    pub fn get_objectives(&self) -> Option<&Vec<f64>> {
        self.quality.get_objectives()
    }

    /// Get specific objective value
    pub fn get_objective(&self, index: usize) -> Option<f64> {
        self.quality.get_objective(index)
    }

    /// Set objective values
    pub fn set_objectives(&mut self, objectives: Vec<f64>) {
        self.quality.set_fitness_indicator(Some(objectives));
    }

    /// Get rank (for NSGA-II)
    pub fn get_rank(&self) -> Option<usize> {
        self.quality.get_rank()
    }

    /// Set rank (for NSGA-II)
    pub fn set_rank(&mut self, rank: usize) {
        self.quality.set_rank(rank);
    }

    /// Get crowding distance (for NSGA-II)
    pub fn get_crowding_distance(&self) -> Option<f64> {
        self.quality.get_crowding_distance()
    }

    /// Set crowding distance (for NSGA-II)
    pub fn set_crowding_distance(&mut self, distance: f64) {
        self.quality.set_crowding_distance(distance);
    }

    /// Check if this solution dominates another (Pareto dominance)
    pub fn dominates(&self, other: &Self) -> bool {
        self.quality.dominates(&other.quality)
    }
}

impl Solution<f64> for RealSolution {
    type Quality = MultiObjectiveQualityIndicator;

    fn new(solution_info: SolutionInfo<f64>) -> Self {
        Self {
            solution_info,
            quality: MultiObjectiveQualityIndicator::new(None),
        }
    }

    fn get_solution_info(&self) -> &SolutionInfo<f64> {
        &self.solution_info
    }

    fn get_solution_info_mut(&mut self) -> &mut SolutionInfo<f64> {
        &mut self.solution_info
    }

    fn get_quality(&self) -> Option<&Self::Quality> {
        Some(&self.quality)
    }

    fn set_quality(&mut self, quality: Self::Quality) {
        self.quality = quality;
    }

    /// For multi-objective, return first objective as primary value
    /// (mainly for compatibility with single-objective interfaces)
    fn value(&self) -> f64 {
        self.quality
            .get_objective(0)
            .unwrap_or(f64::NEG_INFINITY)
    }

    fn compare(&self, other: &Self) -> Option<Ordering> {
        self.quality.compare(&other.quality)
    }
}

impl PartialEq for RealSolution {
    fn eq(&self, other: &Self) -> bool {
        self.solution_info == other.solution_info
    }
}

impl Eq for RealSolution {}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_new_solution() {
        let solution = RealSolution::new_with_size(5);
        assert_eq!(solution.get_number_of_variables(), 5);
        assert_eq!(solution.get_variable(0), Some(&0.0));
    }

    #[test]
    fn test_set_objectives() {
        let mut solution = RealSolution::new_with_size(3);
        solution.set_objectives(vec![1.0, 2.0]);

        assert_eq!(solution.get_objective(0), Some(1.0));
        assert_eq!(solution.get_objective(1), Some(2.0));
        assert_eq!(solution.value(), 1.0); // First objective
    }

    #[test]
    fn test_dominance() {
        let mut sol1 = RealSolution::new_with_size(2);
        let mut sol2 = RealSolution::new_with_size(2);

        sol1.set_objectives(vec![1.0, 2.0]);
        sol2.set_objectives(vec![2.0, 3.0]);

        assert!(sol1.dominates(&sol2));
        assert!(!sol2.dominates(&sol1));
    }

    #[test]
    fn test_rank_crowding() {
        let mut solution = RealSolution::new_with_size(2);
        
        solution.set_rank(5);
        assert_eq!(solution.get_rank(), Some(5));

        solution.set_crowding_distance(3.14);
        assert_eq!(solution.get_crowding_distance(), Some(3.14));
    }
}
