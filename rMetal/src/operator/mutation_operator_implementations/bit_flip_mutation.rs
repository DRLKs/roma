use crate::operator::traits::{MutationOperator, Operator};
use crate::solutions::implementations::binary_solution::BinarySolution;
use crate::solutions::traits::Solution;
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

impl MutationOperator<bool, BinarySolution> for BitFlipMutation {
    fn execute(&self, solution: &mut BinarySolution, probability: f64) {
        let mut rng = Random::new(crate::utils::random::seed_from_time());

        for i in 0..solution.get_number_of_variables() {
            if rng.next_f64() < probability {
                if let Some(value) = solution.get_variable(i) {
                    let _ = solution.set_variable(i, !value);
                }
            }
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_bit_flip_mutation_name() {
        let mutation = BitFlipMutation::new();

        assert_eq!(mutation.name(), "BitFlipMutation");
    }

    #[test]
    fn test_bit_flip_mutation() {
        let mutation = BitFlipMutation::new();
        let mut solution = BinarySolution::zeros(10);

        // With probability 1.0, all bits should be flipped
        mutation.execute(&mut solution, 1.0);

        assert!(solution.count_ones() > 0, "At least some bits should be flipped");
    }

    #[test]
    fn test_bit_flip_mutation_zero_probability() {
        let mutation = BitFlipMutation::new();
        let mut solution = BinarySolution::zeros(10);

        // With probability 0.0, no bits should be flipped
        mutation.execute(&mut solution, 0.0);

        assert_eq!(solution.count_ones(), 0, "No one should be flipped");
    }

    #[test]
    fn test_bit_flip_mutation_zero_probability_negative_case() {
        let mutation = BitFlipMutation::new();
        let mut solution = BinarySolution::zeros(15);

        // With probability 0.0, no bits should be flipped
        mutation.execute(&mut solution, -2.0);

        assert_eq!(solution.count_ones(), 0, "No one should be flipped");
    }

    #[test]
    fn test_bit_flip_mutation_zero_probability_greater_one_case() {
        let mutation = BitFlipMutation::new();
        let size = 15;
        let mut solution = BinarySolution::zeros(size);

        // With probability 2.0, all bits should be flipped
        mutation.execute(&mut solution, 2.0);

        assert_eq!(solution.count_ones(), size, "All bits should be flipped");
    }
}
