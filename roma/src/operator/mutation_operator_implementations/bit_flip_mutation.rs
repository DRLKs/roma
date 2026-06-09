use crate::solution::RealBounds;
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
    fn execute(
        &self,
        solution: &mut Solution<bool>,
        probability: f64,
        bounds: Option<&RealBounds>,
        rng: &mut Random,
    ) {
        let _ = bounds;
        if probability <= 0.0 {
            return;
        }

        let variables = solution.variables_mut();
        for value in variables.iter_mut() {
            if rng.next_f64() < probability {
                *value = !*value;
            }
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::solution::BinarySolutionBuilder;

    #[test]
    fn name_is_exposed() {
        let mutation = BitFlipMutation::new();

        assert_eq!(mutation.name(), "BitFlipMutation");
    }

    #[test]
    fn probability_one_flips_every_bit() {
        let mutation = BitFlipMutation::new();
        let mut solution = BinarySolutionBuilder::zeros(10).build();
        let mut rng = Random::new(42);

        mutation.execute(&mut solution, 1.0, None, &mut rng);

        assert!(solution.variables().iter().all(|&value| value));
    }

    #[test]
    fn zero_probability_leaves_solution_unchanged() {
        let mutation = BitFlipMutation::new();
        let mut solution = BinarySolutionBuilder::zeros(10).build();
        let mut rng = Random::new(42);

        mutation.execute(&mut solution, 0.0, None, &mut rng);

        let number_ones = solution.variables().iter().filter(|&&x| x).count();
        assert_eq!(number_ones, 0, "No one should be flipped");
    }

    #[test]
    fn negative_probability_leaves_solution_unchanged() {
        let mutation = BitFlipMutation::new();
        let mut solution = BinarySolutionBuilder::zeros(15).build();
        let mut rng = Random::new(42);

        mutation.execute(&mut solution, -2.0, None, &mut rng);

        let number_ones = solution.variables().iter().filter(|&&x| x).count();
        assert_eq!(number_ones, 0, "No one should be flipped");
    }

    #[test]
    fn probability_greater_than_one_flips_every_bit() {
        let mutation = BitFlipMutation::new();
        let size = 15;
        let mut solution = BinarySolutionBuilder::zeros(size).build();
        let mut rng = Random::new(42);

        mutation.execute(&mut solution, 2.0, None, &mut rng);

        assert_eq!(solution.variables(), vec![true; size].as_slice());
    }
}
