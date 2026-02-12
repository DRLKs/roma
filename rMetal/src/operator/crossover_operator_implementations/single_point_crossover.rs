use crate::operator::traits::{CrossoverOperator, Operator};
use crate::solutions::implementations::binary_solution::BinarySolution;
use crate::solutions::traits::Solution;
use crate::utils::random::Random;

/// Single Point Crossover operator for binary solutions.
/// Selects a random crossover point and exchanges segments between parents.
pub struct SinglePointCrossover {
    name: String,
    offspring_count: usize,
}

impl SinglePointCrossover {
    pub fn new() -> Self {
        SinglePointCrossover {
            name: "SinglePointCrossover".to_string(),
            offspring_count: 2,
        }
    }
    
    /// Creates a crossover operator that produces only one offspring
    pub fn with_one_offspring() -> Self {
        SinglePointCrossover {
            name: "SinglePointCrossover".to_string(),
            offspring_count: 1,
        }
    }
}

impl Default for SinglePointCrossover {
    fn default() -> Self {
        Self::new()
    }
}

impl Operator for SinglePointCrossover {
    fn name(&self) -> &str {
        &self.name
    }
}

impl CrossoverOperator<bool, BinarySolution> for SinglePointCrossover {
    fn execute(&self, parent1: &BinarySolution, parent2: &BinarySolution) -> Vec<BinarySolution> {
        let length = parent1.get_number_of_variables().min(parent2.get_number_of_variables());
        
        if length <= 1 {
            // Cannot perform crossover, return copies of parents
            return vec![parent1.copy(), parent2.copy()];
        }
        
        let mut rng = Random::new(crate::utils::random::seed_from_time());
        let crossover_point = rng.range(length as u64 - 1) as usize + 1;
        
        let mut offspring1 = parent1.copy();
        let mut offspring2 = parent2.copy();
        
        // Exchange segments after crossover point
        for i in crossover_point..length {
            if let (Some(&val1), Some(&val2)) = (parent1.get_variable(i), parent2.get_variable(i)) {
                let _ = offspring1.set_variable(i, val2);
                let _ = offspring2.set_variable(i, val1);
            }
        }
        
        if self.offspring_count == 1 {
            vec![offspring1]
        } else {
            vec![offspring1, offspring2]
        }
    }
    
    fn number_of_offspring(&self) -> usize {
        self.offspring_count
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_single_point_crossover() {
        let crossover = SinglePointCrossover::new();
        let parent1 = BinarySolution::zeros(10);
        let parent2 = BinarySolution::ones(10);
        
        let offspring = crossover.execute(&parent1, &parent2);
        
        assert_eq!(offspring.len(), 2);
        assert_eq!(offspring[0].get_number_of_variables(), 10);
        assert_eq!(offspring[1].get_number_of_variables(), 10);
    }
}
