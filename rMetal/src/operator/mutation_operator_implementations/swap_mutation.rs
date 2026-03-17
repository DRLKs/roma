use crate::operator::traits::{MutationOperator, Operator};
use crate::solution::Solution;
use crate::utils::random::Random;

/// Swap mutation for permutation solutions.
///
/// Preserves permutation validity by swapping indexes.
#[derive(Clone)]
pub struct SwapMutation {
    name: String,
}

impl SwapMutation {
    pub fn new() -> Self {
        Self {
            name: "SwapMutation".to_string(),
        }
    }
}

impl Default for SwapMutation {
    fn default() -> Self {
        Self::new()
    }
}

impl Operator for SwapMutation {
    fn name(&self) -> &str {
        &self.name
    }
}

impl MutationOperator<usize> for SwapMutation {
    fn execute(&self, solution: &mut Solution<usize>, probability: f64, rng: &mut Random) {
        let n = solution.num_variables();
        if n < 2 {
            return;
        }

        let p = probability.clamp(0.0, 1.0);
        for i in 0..n {
            if rng.next_f64() < p {
                let mut j = rng.range(n as u64) as usize;
                if j == i {
                    j = (j + 1) % n;
                }
                solution.variables.swap(i, j);
            }
        }

        solution.invalidate();
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn name_is_exposed() {
        let mutation = SwapMutation::new();
        assert_eq!(mutation.name(), "SwapMutation");
    }

    #[test]
    fn keeps_same_length() {
        let mutation = SwapMutation::new();
        let mut solution = Solution::new(vec![0, 1, 2, 3, 4]);
        let mut rng = Random::new(42);

        mutation.execute(&mut solution, 1.0, &mut rng);
        assert_eq!(solution.num_variables(), 5);
    }
}
