use crate::operator::traits::{Operator, SelectionOperator};
use crate::solution::MultiObjectiveInfo;
use crate::utils::random::{Random, seed_from_time};
use std::cell::RefCell;
use crate::solution::Solution;

/// Binary Tournament Selection for Multi-Objective Optimization.
///
/// Selection priority:
/// 1) Lower rank is better.
/// 2) If rank ties, larger crowding distance is better.
/// 3) If both tie/missing, fallback to Pareto dominance.
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

impl SelectionOperator<f64, MultiObjectiveInfo> for MultiObjectiveTournamentSelection {
    fn execute<'a>(&self, population: &'a [Solution<f64, MultiObjectiveInfo>]) -> &'a Solution<f64, MultiObjectiveInfo> {
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

        let rank1 = solution1.rank().unwrap_or(usize::MAX);
        let rank2 = solution2.rank().unwrap_or(usize::MAX);

        if rank1 < rank2 {
            return solution1;
        }
        if rank2 < rank1 {
            return solution2;
        }

        let crowding1 = solution1.crowding_distance().unwrap_or(0.0);
        let crowding2 = solution2.crowding_distance().unwrap_or(0.0);

        if crowding1 > crowding2 {
            return solution1;
        }
        if crowding2 > crowding1 {
            return solution2;
        }

        if solution1.dominates(solution2) {
            solution1
        } else {
            solution2
        }
    }
}

#[cfg(test)]
mod tests {
    use crate::solution::{MultiObjectiveRealSolutionBuilder};
    use super::*;

    #[test]
    fn test_selection_from_single_solution() {
        let selection = MultiObjectiveTournamentSelection::new();
        let solution = MultiObjectiveRealSolutionBuilder::from_variables(vec![1.0])
            .with_objectives(vec![0.5, 0.5])
            .with_rank(0)
            .build();
        let population = vec![solution];

        let selected = selection.execute(&population);
        assert_eq!(selected.variables(), &[1.0]);
    }

    #[test]
    #[should_panic(expected = "Cannot select from empty population")]
    fn test_selection_from_empty_population() {
        let selection = MultiObjectiveTournamentSelection::new();
        let population: Vec<Solution<f64, MultiObjectiveInfo>> = vec![];
        selection.execute(&population);
    }

    #[test]
    fn test_selection_prefers_better_rank() {
        let selection = MultiObjectiveTournamentSelection::new();

        let solution1 = MultiObjectiveRealSolutionBuilder::from_variables(vec![1.0])
            .with_objectives(vec![0.1, 0.1])
            .with_rank(0)
            .build();

        let solution2 = MultiObjectiveRealSolutionBuilder::from_variables(vec![1.0])
            .with_objectives(vec![0.9, 0.9])
            .with_rank(1)
            .build();

        let population = vec![solution1, solution2];

        for _ in 0..10 {
            let selected = selection.execute(&population);
            assert_eq!(selected.rank(), Some(0));
        }
    }

    #[test]
    fn test_selection_uses_crowding_distance_when_rank_ties() {
        let selection = MultiObjectiveTournamentSelection::new();

        let solution1 = MultiObjectiveRealSolutionBuilder::from_variables(vec![1.0])
            .with_objectives(vec![0.4, 0.6])
            .with_rank(0)
            .with_crowding_distance(2.0)
            .build();

        let solution2 = MultiObjectiveRealSolutionBuilder::from_variables(vec![1.0])
            .with_objectives(vec![0.5, 0.5])
            .with_rank(0)
            .with_crowding_distance(1.0)
            .build();

        let population = vec![solution1, solution2];

        for _ in 0..10 {
            let selected = selection.execute(&population);
            assert_eq!(selected.crowding_distance(), Some(2.0));
        }
    }

    #[test]
    fn test_selection_name() {
        let selection = MultiObjectiveTournamentSelection::new();
        assert_eq!(selection.name(), "Multi-Objective Tournament Selection");
    }
}
