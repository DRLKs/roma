use crate::operator::traits::{Operator, SelectionOperator};
use crate::algorithms::objective::{is_better, ImprovementDirection};
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
        direction: ImprovementDirection,
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
        
        if is_better(
            individual2.quality_value(),
            individual1.quality_value(),
            direction,
        ) {
            individual2
        } else {
            individual1
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::algorithms::objective::ImprovementDirection;
    use crate::solution::BinarySolutionBuilder;

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
        
        let selected = selection.execute(&population, &mut rng, ImprovementDirection::Maximize);
        
        // Should consistently select the better solution
        assert_eq!(selected.quality_value(), 10.0);
    }

    #[test]
    #[should_panic]
    fn test_binary_tournament_selection_with_empty_population() {
        let selection = BinaryTournamentSelection::new();
        let mut rng = Random::new(42);

        let population: Vec<Solution<bool>> = vec![];

        let _selected = selection.execute(&population, &mut rng, ImprovementDirection::Maximize);
    }

    #[test]
    fn test_binary_tournament_selection_with_only_one() {
        let selection = BinaryTournamentSelection::new();
        let mut rng = Random::new(42);

        let fitness = 10.0;
        let solution = BinarySolutionBuilder::zeros(5).with_quality(fitness).build();


        let population = vec![solution];

        let selected = selection.execute(&population, &mut rng, ImprovementDirection::Maximize);

        assert_eq!(selected.quality_value(), fitness);
    }

    #[test]
    fn test_binary_tournament_selection_minimization() {
        let selection = BinaryTournamentSelection::new();
        let mut rng = Random::new(42);

        let solution1 = BinarySolutionBuilder::zeros(5).with_quality(10.0).build();
        let solution2 = BinarySolutionBuilder::ones(5).with_quality(5.0).build();

        let population = vec![solution1, solution2];

        let selected = selection.execute(&population, &mut rng, ImprovementDirection::Minimize);
        assert_eq!(selected.quality_value(), 5.0);
    }
}
