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
    use crate::solutions::traits::Solution;

    #[test]
    fn test_bit_flip_mutation() {
        let mutation = BitFlipMutation::new();
        let mut solution = BinarySolution::zeros(10);

        // With probability 1.0, all bits should be flipped
        mutation.execute(&mut solution, 1.0);

        // Check that at least some bits changed (probabilistic test)
        let ones_count = (0..10)
            .filter(|&i| *solution.get_variable(i).unwrap())
            .count();

        assert!(ones_count > 0, "At least some bits should be flipped");
    }
}
