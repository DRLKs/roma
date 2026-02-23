use crate::operator::traits::{Operator, SelectionOperator};
use crate::solutions::implementations::real_solution::RealSolution;
use crate::utils::random::{Random, seed_from_time};
use std::cell::RefCell;

/// Binary Tournament Selection for Multi-Objective Optimization
///
/// Selects solutions based on Pareto rank and crowding distance:
/// 1. Prefer solution with better (lower) rank
/// 2. If ranks are equal, prefer solution with larger crowding distance
pub struct MultiObjectiveTournamentSelection {
    rng: RefCell<Random>,
}

impl MultiObjectiveTournamentSelection {
    pub fn new() -> Self {
        MultiObjectiveTournamentSelection {
            rng: RefCell::new(Random::new(seed_from_time())),
        }
    }
}

impl Default for MultiObjectiveTournamentSelection {
    fn default() -> Self {
        Self::new()
    }
}

impl Operator for MultiObjectiveTournamentSelection {
    fn name(&self) -> &str {
        "Multi-Objective Tournament Selection"
    }
}

impl SelectionOperator<f64, RealSolution> for MultiObjectiveTournamentSelection {
    fn execute<'a>(&self, population: &'a [RealSolution]) -> &'a RealSolution {
        if population.is_empty() {
            panic!("Cannot select from empty population");
        }

        if population.len() == 1 {
            return &population[0];
        }
        
        // Single borrow for efficiency
        let mut rng = self.rng.borrow_mut();

        // Select two random individuals
        let index1 = rng.range_between(0, population.len() as u64) as usize;
        let mut index2 = rng.range_between(0, population.len() as u64) as usize;

        // Ensure different individuals
        while index2 == index1 && population.len() > 1 {
            index2 = rng.range_between(0, population.len() as u64) as usize;
        }

        let solution1 = &population[index1];
        let solution2 = &population[index2];

        // Compare based on rank and crowding distance
        let rank1 = solution1.get_rank().unwrap_or(usize::MAX);
        let rank2 = solution2.get_rank().unwrap_or(usize::MAX);

        if rank1 < rank2 {
            solution1
        } else if rank2 < rank1 {
            solution2
        } else {
            // Same rank, compare crowding distance (prefer larger)
            let crowding1 = solution1.get_crowding_distance().unwrap_or(0.0);
            let crowding2 = solution2.get_crowding_distance().unwrap_or(0.0);

            if crowding1 > crowding2 {
                solution1
            } else {
                solution2
            }
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::solutions::traits::{Solution, SolutionInfo};

    #[test]
    fn test_selection_from_single_solution() {
        let mut selection = MultiObjectiveTournamentSelection::new();
        let solution = RealSolution::new(SolutionInfo::new(vec![0.5]));
        let population = vec![solution];

        let selected = selection.execute(&population);
        assert_eq!(
            selected.get_solution_info().get_variables(),
            &[0.5]
        );
    }

    #[test]
    #[should_panic(expected = "Cannot select from empty population")]
    fn test_selection_from_empty_population() {
        let mut selection = MultiObjectiveTournamentSelection::new();
        let population: Vec<RealSolution> = vec![];
        selection.execute(&population);
    }

    #[test]
    fn test_selection_prefers_better_rank() {
        let mut selection = MultiObjectiveTournamentSelection::new();

        let mut solution1 = RealSolution::new(SolutionInfo::new(vec![0.1]));
        solution1.set_rank(0);

        let mut solution2 = RealSolution::new(SolutionInfo::new(vec![0.9]));
        solution2.set_rank(1);

        let population = vec![solution1, solution2];

        // Run multiple times to ensure rank is preferred
        for _ in 0..10 {
            let selected = selection.execute(&population);
            assert_eq!(selected.get_rank(), Some(0));
        }
    }

    #[test]
    fn test_selection_uses_crowding_distance_when_same_rank() {
        let mut selection = MultiObjectiveTournamentSelection::new();

        let mut solution1 = RealSolution::new(SolutionInfo::new(vec![0.1]));
        solution1.set_rank(0);
        solution1.set_crowding_distance(100.0);

        let mut solution2 = RealSolution::new(SolutionInfo::new(vec![0.9]));
        solution2.set_rank(0);
        solution2.set_crowding_distance(1.0);

        let population = vec![solution1, solution2];

        // Run multiple times to check crowding distance preference
        let mut high_crowding_selected = 0;
        for _ in 0..20 {
            let selected = selection.execute(&population);
            if selected.get_crowding_distance() == Some(100.0) {
                high_crowding_selected += 1;
            }
        }

        // With higher crowding distance, should be selected more often
        assert!(high_crowding_selected > 10);
    }

    #[test]
    fn test_selection_name() {
        let selection = MultiObjectiveTournamentSelection::new();
        assert_eq!(selection.name(), "Multi-Objective Tournament Selection");
    }
}
