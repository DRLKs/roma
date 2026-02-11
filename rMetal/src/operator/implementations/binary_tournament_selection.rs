use crate::operator::operator_trait::{Operator, SelectionOperator};
use crate::solutions::solution_trait::Solution;
use crate::utils::random::Random;

/// Binary Tournament Selection operator.
/// Randomly selects two solutions and returns the better one.
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

impl<T, S> SelectionOperator<T, S> for BinaryTournamentSelection
where
    S: Solution<T>,
    T: Clone,
{
    fn execute<'a>(&self, population: &'a [S]) -> &'a S {
        if population.is_empty() {
            panic!("Cannot select from empty population");
        }
        
        if population.len() == 1 {
            return &population[0];
        }
        
        let mut rng = Random::new(crate::utils::random::seed_from_time());
        
        let index1 = rng.range(population.len() as u64) as usize;
        let mut index2 = rng.range(population.len() as u64) as usize;
        
        // Ensure we select two different individuals
        while index2 == index1 && population.len() > 1 {
            index2 = rng.range(population.len() as u64) as usize;
        }
        
        let individual1 = &population[index1];
        let individual2 = &population[index2];
        
        // Return the better solution (higher fitness value)
        if individual1.value() >= individual2.value() {
            individual1
        } else {
            individual2
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::solutions::implementations::binary_solution::BinarySolution;
    use crate::quality_indicator::implementations::decimal_quality_indicator::DecimalQualityIndicator;

    #[test]
    fn test_binary_tournament_selection() {
        let selection = BinaryTournamentSelection::new();
        
        let mut solution1 = BinarySolution::zeros(5);
        solution1.set_quality(DecimalQualityIndicator::new(Some(10.0)));
        
        let mut solution2 = BinarySolution::ones(5);
        solution2.set_quality(DecimalQualityIndicator::new(Some(5.0)));
        
        let population = vec![solution1, solution2];
        
        let selected = selection.execute(&population);
        
        // Should consistently select the better solution
        assert_eq!(selected.value(), 10.0);
    }
}
