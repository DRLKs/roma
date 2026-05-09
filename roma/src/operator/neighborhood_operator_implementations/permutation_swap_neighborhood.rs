use std::fmt::Display;

use crate::operator::traits::{NeighborhoodOperator, Operator};
use crate::solution::Solution;
use crate::utils::random::Random;

/// Permutation neighborhood that applies one or more random swaps.
#[derive(Clone)]
pub struct PermutationSwapNeighborhood {
    swap_count: usize,
}

impl PermutationSwapNeighborhood {
    pub fn new(swap_count: usize) -> Self {
        assert!(swap_count > 0, "swap_count must be > 0");
        Self { swap_count }
    }

    pub fn swap_count(&self) -> usize {
        self.swap_count
    }
}

impl Operator for PermutationSwapNeighborhood {
    fn name(&self) -> &str {
        "PermutationSwapNeighborhood"
    }
}

impl<Q> NeighborhoodOperator<usize, Q> for PermutationSwapNeighborhood
where
    Q: Clone + Display,
{
    fn generate_neighbor(&self, solution: &Solution<usize, Q>, rng: &mut Random) -> Solution<usize, Q> {
        let mut neighbor = solution.copy();
        let n = neighbor.num_variables();
        if n < 2 {
            return neighbor;
        }

        for _ in 0..self.swap_count {
            let i = rng.range(n as u64) as usize;
            let mut j = rng.range(n as u64) as usize;
            if i == j {
                j = (j + 1) % n;
            }
            let _ = neighbor.swap_variables(i, j);
        }

        neighbor
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn keeps_permutation_membership() {
        let operator = PermutationSwapNeighborhood::new(2);
        let solution: Solution<usize> = Solution::new(vec![0, 1, 2, 3, 4]);
        let mut rng = Random::new(42);

        let neighbor = operator.generate_neighbor(&solution, &mut rng);
        let mut values = neighbor.variables().to_vec();
        values.sort_unstable();

        assert_eq!(values, vec![0, 1, 2, 3, 4]);
    }

    #[test]
    fn handles_short_permutations() {
        let operator = PermutationSwapNeighborhood::new(1);
        let solution: Solution<usize> = Solution::new(vec![0]);
        let mut rng = Random::new(9);

        let neighbor = operator.generate_neighbor(&solution, &mut rng);

        assert_eq!(neighbor.variables(), &[0]);
    }
}