use crate::operator::traits::{Operator, SelectionOperator};
use crate::solution::ParetoCrowdingDistanceQuality;
use crate::solution::Solution;
use crate::utils::random::Random;

/// Binary Tournament Selection for Multi-Objective Optimization.
///
/// Selection priority:
/// 1) Lower rank is better.
/// 2) If rank ties, larger crowding distance is better.
/// 3) If both tie/missing, fallback to Pareto dominance.
/// 4) If neither dominates (common in Pareto fronts), break tie randomly.
pub struct MultiObjectiveTournamentSelection {}

impl MultiObjectiveTournamentSelection {
    pub fn new() -> Self {
        MultiObjectiveTournamentSelection {}
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

impl SelectionOperator<f64, ParetoCrowdingDistanceQuality> for MultiObjectiveTournamentSelection {
    fn execute<'a>(
        &self,
        population: &'a [Solution<f64, ParetoCrowdingDistanceQuality>],
        rng: &mut Random,
        dominates: &dyn Fn(&Solution<f64, ParetoCrowdingDistanceQuality>, &Solution<f64, ParetoCrowdingDistanceQuality>) -> bool,
    ) -> &'a Solution<f64, ParetoCrowdingDistanceQuality> {
        if population.is_empty() {
            panic!("Cannot select from empty population");
        }

        if population.len() == 1 {
            return &population[0];
        }

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

        if dominates(solution1, solution2) {
            solution1
        } else if dominates(solution2, solution1) {
            solution2
        } else {
            // In multi-objective optimization, incomparability is common.
            // Avoid deterministic bias by using a random tie-break.
            if rng.next_f64() < 0.5 {
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
    use crate::solution::MultiObjectiveRealSolutionBuilder;
    use crate::problem::implementations::zdt1_problem::ZDT1Problem;
    use crate::problem::traits::Problem;

    #[test]
    fn returns_the_only_solution_when_population_has_one_member() {
        let selection = MultiObjectiveTournamentSelection::new();
        let mut rng = Random::new(42);
        let solution = MultiObjectiveRealSolutionBuilder::from_variables(vec![1.0])
            .with_objectives(vec![0.5, 0.5])
            .with_rank(0)
            .build();
        let population = vec![solution];
        let problem = ZDT1Problem::new(2);

        let selected = selection.execute(&population, &mut rng, &|a, b| problem.dominates(a, b));
        assert_eq!(selected.variables(), &[1.0]);
    }

    #[test]
    #[should_panic(expected = "Cannot select from empty population")]
    fn panics_on_empty_population() {
        let selection = MultiObjectiveTournamentSelection::new();
        let mut rng = Random::new(42);
        let population: Vec<Solution<f64, ParetoCrowdingDistanceQuality>> = vec![];
        let problem = ZDT1Problem::new(2);
        selection.execute(&population, &mut rng, &|a, b| problem.dominates(a, b));
    }

    #[test]
    fn prefers_lower_rank() {
        let selection = MultiObjectiveTournamentSelection::new();
        let mut rng = Random::new(42);

        let solution1 = MultiObjectiveRealSolutionBuilder::from_variables(vec![1.0])
            .with_objectives(vec![0.1, 0.1])
            .with_rank(0)
            .build();

        let solution2 = MultiObjectiveRealSolutionBuilder::from_variables(vec![1.0])
            .with_objectives(vec![0.9, 0.9])
            .with_rank(1)
            .build();

        let population = vec![solution1, solution2];
        let problem = ZDT1Problem::new(2);

        for _ in 0..10 {
            let selected = selection.execute(&population, &mut rng, &|a, b| problem.dominates(a, b));
            assert_eq!(selected.rank(), Some(0));
        }
    }

    #[test]
    fn uses_crowding_distance_when_rank_ties() {
        let selection = MultiObjectiveTournamentSelection::new();
        let mut rng = Random::new(42);

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
        let problem = ZDT1Problem::new(2);

        for _ in 0..10 {
            let selected = selection.execute(&population, &mut rng, &|a, b| problem.dominates(a, b));
            assert_eq!(selected.crowding_distance(), Some(2.0));
        }
    }

    #[test]
    fn name_is_exposed() {
        let selection = MultiObjectiveTournamentSelection::new();
        assert_eq!(selection.name(), "Multi-Objective Tournament Selection");
    }

    #[test]
    fn breaks_non_dominated_ties_without_bias_to_second() {
        let selection = MultiObjectiveTournamentSelection::new();
        let mut rng = Random::new(1234);

        // Same rank and crowding, and no dominance relation.
        let solution1 = MultiObjectiveRealSolutionBuilder::from_variables(vec![1.0])
            .with_objectives(vec![0.5, 0.5])
            .with_rank(0)
            .with_crowding_distance(1.0)
            .build();

        let solution2 = MultiObjectiveRealSolutionBuilder::from_variables(vec![2.0])
            .with_objectives(vec![0.5, 0.5])
            .with_rank(0)
            .with_crowding_distance(1.0)
            .build();

        let population = vec![solution1, solution2];
        let mut picked_first = 0usize;
        let mut picked_second = 0usize;
        let problem = ZDT1Problem::new(2);

        for _ in 0..100 {
            let selected = selection.execute(&population, &mut rng, &|a, b| problem.dominates(a, b));
            if selected.variables() == &[1.0] {
                picked_first += 1;
            } else {
                picked_second += 1;
            }
        }

        assert!(picked_first > 0);
        assert!(picked_second > 0);
    }

    #[test]
    fn falls_back_to_dominance_when_rank_and_crowding_are_missing() {
        let selection = MultiObjectiveTournamentSelection::new();
        let mut rng = Random::new(42);

        let dominating = MultiObjectiveRealSolutionBuilder::from_variables(vec![1.0])
            .with_objectives(vec![0.1, 0.1])
            .build();

        let dominated = MultiObjectiveRealSolutionBuilder::from_variables(vec![2.0])
            .with_objectives(vec![0.9, 0.9])
            .build();

        let population = vec![dominating, dominated];
        let problem = ZDT1Problem::new(2);

        for _ in 0..10 {
            let selected = selection.execute(&population, &mut rng, &|a, b| problem.dominates(a, b));
            assert_eq!(selected.variables(), &[1.0]);
        }
    }

    #[test]
    fn missing_crowding_distance_is_treated_as_lower_than_present_values() {
        let selection = MultiObjectiveTournamentSelection::new();
        let mut rng = Random::new(42);

        let missing_crowding = MultiObjectiveRealSolutionBuilder::from_variables(vec![1.0])
            .with_objectives(vec![0.4, 0.6])
            .with_rank(0)
            .build();

        let higher_crowding = MultiObjectiveRealSolutionBuilder::from_variables(vec![2.0])
            .with_objectives(vec![0.6, 0.4])
            .with_rank(0)
            .with_crowding_distance(2.0)
            .build();

        let population = vec![missing_crowding, higher_crowding];
        let problem = ZDT1Problem::new(2);

        for _ in 0..10 {
            let selected = selection.execute(&population, &mut rng, &|a, b| problem.dominates(a, b));
            assert_eq!(selected.variables(), &[2.0]);
        }
    }
}
