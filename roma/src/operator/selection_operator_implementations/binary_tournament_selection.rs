use crate::operator::traits::{Operator, SelectionOperator};
use crate::solution::Solution;
use crate::utils::random::Random;

/// Binary Tournament Selection operator.
/// Randomly selects two solutions and returns the better one.
#[derive(Clone)]
pub struct BinaryTournamentSelection {
    name: String,
}

impl BinaryTournamentSelection {
    pub fn new() -> Self {
        BinaryTournamentSelection {
            name: "BinaryTournamentSelection".to_string(),
        }
    }
}

impl Default for BinaryTournamentSelection {
    fn default() -> Self {
        Self::new()
    }
}

impl Operator for BinaryTournamentSelection {
    fn name(&self) -> &str {
        &self.name
    }
}

impl<T> SelectionOperator<T> for BinaryTournamentSelection
where
    T: Clone,
{
    fn execute<'a>(
        &self,
        population: &'a [Solution<T>],
        rng: &mut Random,
        dominates: &dyn Fn(&Solution<T, f64>, &Solution<T, f64>) -> bool,
    ) -> &'a Solution<T> {
        if population.is_empty() {
            panic!("Cannot select from empty population");
        }

        if population.len() == 1 {
            return &population[0];
        }

        let index1 = rng.range(population.len() as u64) as usize;
        let mut index2 = rng.range(population.len() as u64) as usize;

        // Ensure we select two different individuals
        while index2 == index1 && population.len() > 1 {
            index2 = rng.range(population.len() as u64) as usize;
        }

        let individual1 = &population[index1];
        let individual2 = &population[index2];

        if dominates(individual2, individual1) {
            individual2
        } else {
            individual1
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::problem::traits::Problem;
    use crate::solution::BinarySolutionBuilder;

    struct MaxProblem;
    struct MinProblem;

    impl Problem<bool> for MaxProblem {
        fn new() -> Self {
            Self
        }

        fn evaluate(&self, _solution: &mut Solution<bool>) {}

        fn create_solution(&self, _rng: &mut Random) -> Solution<bool> {
            panic!("not needed in tests")
        }

        fn set_problem_description(&mut self, _description: String) {}

        fn get_problem_description(&self) -> String {
            "max".to_string()
        }

        fn dominates(&self, solution_a: &Solution<bool>, solution_b: &Solution<bool>) -> bool {
            solution_a.quality_value() > solution_b.quality_value()
        }

        fn better_fitness_fn(&self) -> fn(f64, f64) -> bool {
            use crate::solution::traits::evaluator::maximizing_fitness;
            maximizing_fitness
        }
    }

    impl Problem<bool> for MinProblem {
        fn new() -> Self {
            Self
        }

        fn evaluate(&self, _solution: &mut Solution<bool>) {}

        fn create_solution(&self, _rng: &mut Random) -> Solution<bool> {
            panic!("not needed in tests")
        }

        fn set_problem_description(&mut self, _description: String) {}

        fn get_problem_description(&self) -> String {
            "min".to_string()
        }

        fn dominates(&self, solution_a: &Solution<bool>, solution_b: &Solution<bool>) -> bool {
            solution_a.quality_value() < solution_b.quality_value()
        }

        fn better_fitness_fn(&self) -> fn(f64, f64) -> bool {
            use crate::solution::traits::evaluator::minimizing_fitness;
            minimizing_fitness
        }
    }

    #[test]
    fn test_binary_tournament_name() {
        let selection = BinaryTournamentSelection::new();

        assert_eq!(selection.name(), "BinaryTournamentSelection");
    }

    #[test]
    fn test_binary_tournament_selection() {
        let selection = BinaryTournamentSelection::new();
        let mut rng = Random::new(42);

        let solution1 = BinarySolutionBuilder::zeros(5).with_quality(10.0).build();

        let solution2 = BinarySolutionBuilder::ones(5).with_quality(5.0).build();

        let population = vec![solution1, solution2];

        let selected = selection.execute(&population, &mut rng, &|a, b| MaxProblem.dominates(a, b));

        // Should consistently select the better solution
        assert_eq!(selected.quality_value(), 10.0);
    }

    #[test]
    #[should_panic]
    fn test_binary_tournament_selection_with_empty_population() {
        let selection = BinaryTournamentSelection::new();
        let mut rng = Random::new(42);

        let population: Vec<Solution<bool>> = vec![];

        let _selected = selection.execute(&population, &mut rng, &|a, b| MaxProblem.dominates(a, b));
    }

    #[test]
    fn test_binary_tournament_selection_with_only_one() {
        let selection = BinaryTournamentSelection::new();
        let mut rng = Random::new(42);

        let fitness = 10.0;
        let solution = BinarySolutionBuilder::zeros(5)
            .with_quality(fitness)
            .build();

        let population = vec![solution];

        let selected = selection.execute(&population, &mut rng, &|a, b| MaxProblem.dominates(a, b));

        assert_eq!(selected.quality_value(), fitness);
    }

    #[test]
    fn test_binary_tournament_selection_minimization() {
        let selection = BinaryTournamentSelection::new();
        let mut rng = Random::new(42);

        let solution1 = BinarySolutionBuilder::zeros(5).with_quality(10.0).build();
        let solution2 = BinarySolutionBuilder::ones(5).with_quality(5.0).build();

        let population = vec![solution1, solution2];

        let selected = selection.execute(&population, &mut rng, &|a, b| MinProblem.dominates(a, b));
        assert_eq!(selected.quality_value(), 5.0);
    }
}
