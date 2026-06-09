use crate::solution::RealBounds;
use crate::operator::traits::{CrossoverOperator, Operator};
use crate::solution::Solution;
use crate::utils::random::Random;

/// Single Point Crossover operator for binary solutions.
/// Selects a random crossover point and exchanges segments between parents.
#[derive(Clone)]
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

impl CrossoverOperator<bool> for SinglePointCrossover {
    fn execute(
        &self,
        parent1: &Solution<bool>,
        parent2: &Solution<bool>,
        _bounds: Option<&RealBounds>,
        rng: &mut Random,
    ) -> Vec<Solution<bool>> {
        let length = parent1.num_variables().min(parent2.num_variables());

        if length <= 1 {
            // Cannot perform crossover, return copies of parents
            return vec![parent1.clone(), parent2.clone()];
        }

        let crossover_point = rng.range(length as u64 - 1) as usize + 1;

        let mut offspring1 = parent1.clone();
        let mut offspring2 = parent2.clone();
        let offspring1_variables = offspring1.variables_mut();
        let offspring2_variables = offspring2.variables_mut();

        // Exchange segments after crossover point
        for i in crossover_point..length {
            let val1 = parent1
                .get_variable(i)
                .copied()
                .expect("index must be valid within crossover length");
            let val2 = parent2
                .get_variable(i)
                .copied()
                .expect("index must be valid within crossover length");
            offspring1_variables[i] = val2;
            offspring2_variables[i] = val1;
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
    use crate::solution::BinarySolutionBuilder;

    #[test]
    fn name_is_exposed() {
        let crossover = SinglePointCrossover::new();

        assert_eq!(crossover.name(), "SinglePointCrossover");
    }

    #[test]
    fn offspring_exchange_a_suffix_after_a_single_cut_point() {
        let crossover = SinglePointCrossover::new();
        let parent1 = BinarySolutionBuilder::zeros(10).build();
        let parent2 = BinarySolutionBuilder::ones(10).build();
        let mut rng = Random::new(42);

        let offspring = crossover.execute(&parent1, &parent2, None, &mut rng);

        assert_eq!(offspring.len(), 2);

        for child in &offspring {
            assert_eq!(child.num_variables(), 10);
        }

        let first_true = offspring[0]
            .variables()
            .iter()
            .position(|&value| value)
            .expect("offspring should inherit a suffix from the second parent");
        assert!(first_true > 0);
        assert!(first_true < offspring[0].num_variables());
        assert!(offspring[0].variables()[..first_true].iter().all(|&value| !value));
        assert!(offspring[0].variables()[first_true..].iter().all(|&value| value));

        let first_false = offspring[1]
            .variables()
            .iter()
            .position(|&value| !value)
            .expect("offspring should inherit a suffix from the first parent");
        assert!(first_false > 0);
        assert!(first_false < offspring[1].num_variables());
        assert!(offspring[1].variables()[..first_false].iter().all(|&value| value));
        assert!(offspring[1].variables()[first_false..].iter().all(|&value| !value));
    }
}
