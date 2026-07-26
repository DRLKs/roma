use crate::operator::traits::{NeighborhoodOperator, Operator};
use crate::solution::{RealBounds, Solution};
use crate::utils::random::Random;

/// Bit-flip neighborhood operator for binary solutions.
///
/// Defines the Hamming distance-1 neighborhood: from any binary solution of
/// length n, there are exactly n neighbors, each obtained by flipping a single bit.
///
/// This is the standard neighborhood for binary optimization problems such as
/// Knapsack, Max-SAT, and similar combinatorial problems encoded with boolean
/// decision variables.
#[derive(Clone, Debug)]
pub struct BitFlipNeighborhood {
    name: String,
}

impl BitFlipNeighborhood {
    pub fn new() -> Self {
        Self {
            name: "BitFlipNeighborhood".to_string(),
        }
    }
}

impl Default for BitFlipNeighborhood {
    fn default() -> Self {
        Self::new()
    }
}

impl Operator for BitFlipNeighborhood {
    fn name(&self) -> &str {
        &self.name
    }
}

impl NeighborhoodOperator<bool> for BitFlipNeighborhood {
    fn neighborhood_size(&self, solution: &Solution<bool>) -> Option<usize> {
        Some(solution.num_variables())
    }

    fn random_neighbor(
        &self,
        solution: &Solution<bool>,
        _bounds: Option<&RealBounds>,
        rng: &mut Random,
    ) -> Solution<bool> {
        let n = solution.num_variables();
        if n == 0 {
            return solution.clone();
        }

        let flip_index = rng.range(n as u64) as usize;
        let mut variables = solution.variables().to_vec();
        variables[flip_index] = !variables[flip_index];
        Solution::new(variables)
    }

    fn all_neighbors(
        &self,
        solution: &Solution<bool>,
        _bounds: Option<&RealBounds>,
    ) -> Option<Vec<Solution<bool>>> {
        let n = solution.num_variables();
        let mut neighbors = Vec::with_capacity(n);

        for i in 0..n {
            let mut variables = solution.variables().to_vec();
            variables[i] = !variables[i];
            neighbors.push(Solution::new(variables));
        }

        Some(neighbors)
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::collections::HashSet;

    #[test]
    fn neighborhood_size_equals_number_of_variables() {
        let solution = Solution::new(vec![true, false, true, false, true]);
        let neighborhood = BitFlipNeighborhood::new();
        assert_eq!(neighborhood.neighborhood_size(&solution), Some(5));
    }

    #[test]
    fn random_neighbor_differs_by_exactly_one_bit() {
        let solution = Solution::new(vec![true, false, true, false]);
        let neighborhood = BitFlipNeighborhood::new();
        let mut rng = Random::new(42);

        let neighbor = neighborhood.random_neighbor(&solution, None, &mut rng);
        let differences: usize = solution
            .variables()
            .iter()
            .zip(neighbor.variables().iter())
            .filter(|(a, b)| a != b)
            .count();

        assert_eq!(differences, 1);
    }

    #[test]
    fn all_neighbors_flip_each_bit_once() {
        let solution = Solution::new(vec![true, false, true]);
        let neighborhood = BitFlipNeighborhood::new();

        let neighbors = neighborhood.all_neighbors(&solution, None).unwrap();
        assert_eq!(neighbors.len(), 3);

        let expected = [
            vec![false, false, true],
            vec![true, true, true],
            vec![true, false, false],
        ];

        let actual: Vec<Vec<bool>> = neighbors
            .iter()
            .map(|neighbor| neighbor.variables().to_vec())
            .collect();
        assert_eq!(actual, expected);

        let unique_neighbors: HashSet<Vec<bool>> = actual.iter().cloned().collect();
        assert_eq!(unique_neighbors.len(), 3);
    }

    #[test]
    fn empty_solution_returns_clone() {
        let solution: Solution<bool> = Solution::new(vec![]);
        let neighborhood = BitFlipNeighborhood::new();
        let mut rng = Random::new(1);

        let neighbor = neighborhood.random_neighbor(&solution, None, &mut rng);
        assert_eq!(neighbor.num_variables(), 0);
    }
}
