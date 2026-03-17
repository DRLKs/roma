use crate::problem::traits::Problem;
use crate::solution::Solution;
use crate::utils::random::Random;

/// Traveling Salesman Problem (TSP) with explicit distance matrix.
///
/// A solution is a permutation of city indexes.
/// The objective is to minimize total route distance.
#[derive(Clone)]
pub struct TspProblem {
    description: String,
    distance_matrix: Vec<Vec<f64>>,
    close_tour: bool,
}

impl TspProblem {
    /// Creates a TSP instance from a square distance matrix.
    pub fn with_distance_matrix(distance_matrix: Vec<Vec<f64>>) -> Self {
        assert!(
            !distance_matrix.is_empty(),
            "distance_matrix must contain at least one city"
        );

        let size = distance_matrix.len();
        for row in &distance_matrix {
            assert_eq!(
                row.len(),
                size,
                "distance_matrix must be square (NxN)"
            );
        }

        Self {
            description: format!("TSP with {} cities", size),
            distance_matrix,
            close_tour: true,
        }
    }

    pub fn with_open_route(mut self) -> Self {
        self.close_tour = false;
        self
    }

    pub fn number_of_cities(&self) -> usize {
        self.distance_matrix.len()
    }

    pub fn distance(&self, from: usize, to: usize) -> f64 {
        self.distance_matrix[from][to]
    }

    fn is_valid_permutation(&self, route: &[usize]) -> bool {
        let n = self.number_of_cities();
        if route.len() != n {
            return false;
        }

        let mut seen = vec![false; n];
        for &city in route {
            if city >= n || seen[city] {
                return false;
            }
            seen[city] = true;
        }

        true
    }

    fn route_distance(&self, route: &[usize]) -> f64 {
        if route.len() < 2 {
            return 0.0;
        }

        let mut total = 0.0;
        for i in 0..route.len() - 1 {
            total += self.distance(route[i], route[i + 1]);
        }

        if self.close_tour {
            total += self.distance(*route.last().unwrap_or(&route[0]), route[0]);
        }

        total
    }
}

impl Problem<usize> for TspProblem {
    fn new() -> Self {
        Self::with_distance_matrix(vec![vec![0.0]])
    }

    fn evaluate(&self, solution: &mut Solution<usize>) {
        let route = solution.variables();
        let fitness = if self.is_valid_permutation(route) {
            self.route_distance(route)
        } else {
            f64::INFINITY
        };

        solution.set_quality(fitness);
    }

    fn create_solution(&self, rng: &mut Random) -> Solution<usize> {
        let n = self.number_of_cities();
        let mut route: Vec<usize> = (0..n).collect();

        // Fisher-Yates shuffle.
        for i in (1..n).rev() {
            let j = rng.range_between(0, (i + 1) as u64) as usize;
            route.swap(i, j);
        }

        Solution::new(route)
    }

    fn set_problem_description(&mut self, description: String) {
        self.description = description;
    }

    fn get_problem_description(&self) -> String {
        self.description.clone()
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn evaluates_closed_route_distance() {
        let matrix = vec![
            vec![0.0, 10.0, 15.0],
            vec![10.0, 0.0, 20.0],
            vec![15.0, 20.0, 0.0],
        ];

        let problem = TspProblem::with_distance_matrix(matrix);
        let mut solution = Solution::new(vec![0, 1, 2]);
        problem.evaluate(&mut solution);

        // 0->1 (10) + 1->2 (20) + 2->0 (15) = 45
        assert_eq!(solution.quality().copied(), Some(45.0));
    }

    #[test]
    fn invalid_route_gets_infinity() {
        let matrix = vec![vec![0.0, 1.0], vec![1.0, 0.0]];
        let problem = TspProblem::with_distance_matrix(matrix);

        let mut invalid = Solution::new(vec![0, 0]);
        problem.evaluate(&mut invalid);
        assert_eq!(invalid.quality().copied(), Some(f64::INFINITY));
    }
}
