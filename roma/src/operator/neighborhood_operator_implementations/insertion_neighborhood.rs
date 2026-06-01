use crate::operator::traits::{NeighborhoodOperator, Operator};
use crate::solution::{RealBounds, Solution};
use crate::utils::random::Random;

/// Insertion neighborhood operator for permutation solutions.
///
/// Removes an element from one position and reinserts it at a different
/// position, shifting the intermediate elements. This neighborhood is
/// less disruptive than swaps for problems where relative order matters
/// (scheduling, routing with precedence constraints).
#[derive(Clone, Debug)]
pub struct InsertionNeighborhood {
    name: String,
}

impl InsertionNeighborhood {
    pub fn new() -> Self {
        Self {
            name: "InsertionNeighborhood".to_string(),
        }
    }
}

impl Default for InsertionNeighborhood {
    fn default() -> Self {
        Self::new()
    }
}

impl Operator for InsertionNeighborhood {
    fn name(&self) -> &str {
        &self.name
    }
}

impl NeighborhoodOperator<usize> for InsertionNeighborhood {
    fn neighborhood_size(&self, solution: &Solution<usize>) -> Option<usize> {
        let n = solution.num_variables();
        if n < 2 {
            return Some(0);
        }
        // For each of n elements, it can be inserted in (n-1) other positions.
        Some(n * (n - 1))
    }

    fn random_neighbor(
        &self,
        solution: &Solution<usize>,
        _bounds: Option<&RealBounds>,
        rng: &mut Random,
    ) -> Solution<usize> {
        let n = solution.num_variables();
        if n < 2 {
            return solution.clone();
        }

        let from = rng.range(n as u64) as usize;
        let mut to = rng.range(n as u64) as usize;
        if to == from {
            to = (to + 1) % n;
        }

        let mut neighbor = solution.clone();
        let variables = neighbor.variables_mut();
        let element = variables[from];

        if from < to {
            for i in from..to {
                variables[i] = variables[i + 1];
            }
        } else {
            for i in (to + 1..=from).rev() {
                variables[i] = variables[i - 1];
            }
        }
        variables[to] = element;
        neighbor
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn name_is_exposed() {
        let op = InsertionNeighborhood::new();
        assert_eq!(op.name(), "InsertionNeighborhood");
    }

    #[test]
    fn preserves_permutation_membership() {
        let op = InsertionNeighborhood::new();
        let mut rng = Random::new(99);
        let solution = Solution::new(vec![0, 1, 2, 3, 4, 5]);

        for _ in 0..50 {
            let neighbor = op.random_neighbor(&solution, None, &mut rng);
            let mut vars = neighbor.variables().to_vec();
            vars.sort();
            assert_eq!(vars, vec![0, 1, 2, 3, 4, 5]);
        }
    }

    #[test]
    fn single_element_returns_clone() {
        let op = InsertionNeighborhood::new();
        let mut rng = Random::new(1);
        let solution = Solution::new(vec![0]);
        let neighbor = op.random_neighbor(&solution, None, &mut rng);
        assert_eq!(neighbor.variables(), &[0]);
    }

    #[test]
    fn generates_different_neighbor() {
        let op = InsertionNeighborhood::new();
        let mut rng = Random::new(42);
        let source = Solution::new(vec![0, 1, 2, 3, 4, 5, 6, 7]);

        let mut found_different = false;
        for _ in 0..20 {
            let neighbor = op.random_neighbor(&source, None, &mut rng);
            if neighbor.variables() != source.variables() {
                found_different = true;
                break;
            }
        }
        assert!(found_different);
    }

    #[test]
    fn neighborhood_size_is_correct() {
        let op = InsertionNeighborhood::new();
        let solution = Solution::new(vec![0, 1, 2, 3]);
        // n=4: 4*3 = 12
        assert_eq!(op.neighborhood_size(&solution), Some(12));
    }
}
