use crate::operator::traits::{NeighborhoodOperator, Operator};
use crate::solution::{RealBounds, Solution};
use crate::utils::random::Random;

/// 2-opt neighborhood operator for permutation solutions.
///
/// Performs a classic 2-opt move: selects two positions and reverses the
/// sub-path between them. This is the natural neighborhood for TSP and
/// other routing problems.
///
/// Unlike `SwapMutation`, which only exchanges two elements, 2-opt reverses
/// a contiguous segment, exploring a fundamentally different neighborhood
/// structure.
#[derive(Clone, Debug)]
pub struct TwoOptNeighborhood {
    name: String,
}

impl TwoOptNeighborhood {
    pub fn new() -> Self {
        Self {
            name: "TwoOptNeighborhood".to_string(),
        }
    }
}

impl Default for TwoOptNeighborhood {
    fn default() -> Self {
        Self::new()
    }
}

impl Operator for TwoOptNeighborhood {
    fn name(&self) -> &str {
        &self.name
    }
}

impl NeighborhoodOperator<usize> for TwoOptNeighborhood {
    fn neighborhood_size(&self, solution: &Solution<usize>) -> Option<usize> {
        let n = solution.num_variables();
        if n < 3 {
            return Some(0);
        }
        // Number of distinct 2-opt moves for a path representation:
        // for each pair (i, j) with j >= i+2, reverse segment [i+1..=j].
        Some((n - 1) * (n - 2) / 2)
    }

    fn random_neighbor(
        &self,
        solution: &Solution<usize>,
        _bounds: Option<&RealBounds>,
        rng: &mut Random,
    ) -> Solution<usize> {
        let n = solution.num_variables();
        if n < 3 {
            return solution.clone();
        }

        let mut i = rng.range(n as u64) as usize;
        let mut j = rng.range(n as u64) as usize;

        // Ensure i < j and they are not adjacent (which would be a no-op)
        if i > j {
            std::mem::swap(&mut i, &mut j);
        }
        if j == i || j - i < 2 {
            j = (i + 2).min(n - 1);
            if j - i < 2 {
                i = j.saturating_sub(2);
            }
        }

        let mut neighbor = solution.clone();
        let variables = neighbor.variables_mut();
        variables[i + 1..=j].reverse();
        neighbor
    }

    fn all_neighbors(
        &self,
        solution: &Solution<usize>,
        _bounds: Option<&RealBounds>,
    ) -> Option<Vec<Solution<usize>>> {
        let n = solution.num_variables();
        if n < 3 {
            return Some(Vec::new());
        }
        // Only enumerate for reasonably small solutions
        if n > 50 {
            return None;
        }

        let mut neighbors = Vec::new();
        for i in 0..n - 2 {
            for j in i + 2..n {
                let mut neighbor = solution.clone();
                let vars = neighbor.variables_mut();
                vars[i + 1..=j].reverse();
                neighbors.push(neighbor);
            }
        }
        Some(neighbors)
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn name_is_exposed() {
        let op = TwoOptNeighborhood::new();
        assert_eq!(op.name(), "TwoOptNeighborhood");
    }

    #[test]
    fn preserves_permutation_membership() {
        let op = TwoOptNeighborhood::new();
        let mut rng = Random::new(7);
        let solution = Solution::new(vec![0, 1, 2, 3, 4, 5, 6]);

        for _ in 0..50 {
            let neighbor = op.random_neighbor(&solution, None, &mut rng);
            let mut vars = neighbor.variables().to_vec();
            vars.sort();
            assert_eq!(vars, vec![0, 1, 2, 3, 4, 5, 6]);
        }
    }

    #[test]
    fn random_neighbor_does_not_modify_source() {
        let op = TwoOptNeighborhood::new();
        let mut rng = Random::new(42);
        let source = Solution::new(vec![0, 1, 2, 3, 4]);
        let original = source.variables().to_vec();

        let _neighbor = op.random_neighbor(&source, None, &mut rng);
        assert_eq!(source.variables(), &original[..]);
    }

    #[test]
    fn small_solution_returns_clone() {
        let op = TwoOptNeighborhood::new();
        let mut rng = Random::new(1);
        let solution = Solution::new(vec![0, 1]);
        let neighbor = op.random_neighbor(&solution, None, &mut rng);
        assert_eq!(neighbor.variables(), &[0, 1]);
    }

    #[test]
    fn neighborhood_size_is_correct() {
        let op = TwoOptNeighborhood::new();
        let solution = Solution::new(vec![0, 1, 2, 3, 4]);
        // n=5: (5-1)*(5-2)/2 = 4*3/2 = 6
        assert_eq!(op.neighborhood_size(&solution), Some(6));
    }

    #[test]
    fn all_neighbors_enumerates_correctly() {
        let op = TwoOptNeighborhood::new();
        let solution = Solution::new(vec![0, 1, 2, 3, 4]);
        let neighbors = op.all_neighbors(&solution, None).unwrap();
        assert_eq!(neighbors.len(), op.neighborhood_size(&solution).unwrap());
    }
}
