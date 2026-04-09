use crate::operator::traits::{MutationOperator, Operator};
use crate::solution::Solution;
use crate::utils::random::Random;

/// Bit Flip Mutation operator for binary solutions.
/// Each bit has a probability of being flipped (0->1 or 1->0).
#[derive(Clone)]
pub struct BitFlipMutation {
    name: String,
}

impl BitFlipMutation {
    pub fn new() -> Self {
        BitFlipMutation {
            name: "BitFlipMutation".to_string(),
        }
    }
}

impl Default for BitFlipMutation {
    fn default() -> Self {
        Self::new()
    }
}

impl Operator for BitFlipMutation {
    fn name(&self) -> &str {
        &self.name
    }
}

impl MutationOperator<bool> for BitFlipMutation {
    fn execute(&self, solution: &mut Solution<bool>, probability: f64, rng: &mut Random) {
        for i in 0..solution.num_variables() {
            if rng.next_f64() < probability {
                let value = solution
                    .get_variable(i)
                    .copied()
                    .expect("index must be valid within num_variables loop");
                solution.set_variable(i, !value);
            }
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::solution::BinarySolutionBuilder;

    #[test]
    fn test_bit_flip_mutation_name() {
        let mutation = BitFlipMutation::new();

        assert_eq!(mutation.name(), "BitFlipMutation");
    }

    #[test]
    fn test_bit_flip_mutation() {
        let mutation = BitFlipMutation::new();
        let mut solution = BinarySolutionBuilder::zeros(10).build();
        let mut rng = Random::new(42);

        // With probability 1.0, all bits should be flipped
        mutation.execute(&mut solution, 1.0, &mut rng);

        let number_ones = solution.variables().iter().filter(|&&x| x).count();
        assert!(number_ones > 0, "At least some bits should be flipped");
    }

    #[test]
    fn test_bit_flip_mutation_zero_probability() {
        let mutation = BitFlipMutation::new();
        let mut solution = BinarySolutionBuilder::zeros(10).build();
        let mut rng = Random::new(42);

        // With probability 0.0, no bits should be flipped
        mutation.execute(&mut solution, 0.0, &mut rng);

        let number_ones = solution.variables().iter().filter(|&&x| x).count();
        assert_eq!(number_ones, 0, "No one should be flipped");
    }

    #[test]
    fn test_bit_flip_mutation_zero_probability_negative_case() {
        let mutation = BitFlipMutation::new();
        let mut solution = BinarySolutionBuilder::zeros(15).build();
        let mut rng = Random::new(42);

        // With probability 0.0, no bits should be flipped
        mutation.execute(&mut solution, -2.0, &mut rng);

        let number_ones = solution.variables().iter().filter(|&&x| x).count();
        assert_eq!(number_ones, 0, "No one should be flipped");
    }

    #[test]
    fn test_bit_flip_mutation_zero_probability_greater_one_case() {
        let mutation = BitFlipMutation::new();
        let size = 15;
        let mut solution = BinarySolutionBuilder::zeros(size).build();
        let mut rng = Random::new(42);

        // With probability 2.0, all bits should be flipped
        mutation.execute(&mut solution, 2.0, &mut rng);

        let number_ones = solution.variables().iter().filter(|&&x| x).count();
        assert_eq!(number_ones, size, "All bits should be flipped");
    }
}
