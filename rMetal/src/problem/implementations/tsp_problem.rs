use crate::problem::traits::Problem;
use crate::solution::Solution;
use crate::utils::random::Random;
use crate::algorithms::objective::ImprovementDirection;

/// Traveling Salesman Problem (TSP) with explicit distance matrix.
///
/// A solution is a permutation of city indexes.
/// The objective is to minimize total route distance.
#[derive(Clone)]
pub struct TspProblem {
    description: String,
    /// Full pairwise distance matrix, where `distance_matrix[i][j]`
    /// is the distance from city `i` to city `j`.
    distance_matrix: Vec<Vec<f64>>,
    /// Controls route topology:
    /// - `true`: closed tour (Hamiltonian cycle), adds last -> first edge.
    /// - `false`: open route (Hamiltonian path), does not add return edge.
    close_tour: bool,
    /// Optional fixed start city.
    ///
    /// When set, all valid routes must start with this city index.
    fixed_start_city: Option<usize>,
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
            fixed_start_city: None,
        }
    }

    /// Configures a fixed start city.
    ///
    /// The route will be considered valid only when `route[0] == city`.
    /// Random solution generation also respects this constraint.
    pub fn with_fixed_start_city(mut self, city: usize) -> Self {
        assert!(
            city < self.number_of_cities(),
            "fixed start city must be a valid city index"
        );
        self.fixed_start_city = Some(city);
        self
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

        if let Some(start_city) = self.fixed_start_city {
            if route.first().copied() != Some(start_city) {
                return false;
            }
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
        let route: Vec<usize> = match self.fixed_start_city {
            Some(start_city) => {
                let mut rest: Vec<usize> = (0..n).filter(|&city| city != start_city).collect();

                // Fisher-Yates shuffle for the non-fixed suffix.
                for i in (1..rest.len()).rev() {
                    let j = rng.range_between(0, (i + 1) as u64) as usize;
                    rest.swap(i, j);
                }

                let mut route = Vec::with_capacity(n);
                route.push(start_city);
                route.extend(rest);
                route
            }
            None => {
                let mut route: Vec<usize> = (0..n).collect();

                // Fisher-Yates shuffle.
                for i in (1..n).rev() {
                    let j = rng.range_between(0, (i + 1) as u64) as usize;
                    route.swap(i, j);
                }

                route
            }
        };

        Solution::new(route)
    }

    fn set_problem_description(&mut self, description: String) {
        self.description = description;
    }

    fn get_problem_description(&self) -> String {
        self.description.clone()
    }

    fn get_improvement_direction(&self) -> ImprovementDirection {
        ImprovementDirection::Minimize
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::utils::random::Random;

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

    #[test]
    fn open_route_does_not_add_return_edge() {
        let matrix = vec![
            vec![0.0, 10.0, 15.0],
            vec![10.0, 0.0, 20.0],
            vec![15.0, 20.0, 0.0],
        ];

        let problem = TspProblem::with_distance_matrix(matrix).with_open_route();
        let mut solution = Solution::new(vec![0, 1, 2]);
        problem.evaluate(&mut solution);

        // 0->1 (10) + 1->2 (20) = 30
        assert_eq!(solution.quality().copied(), Some(30.0));
    }

    #[test]
    fn create_solution_returns_valid_permutation() {
        let matrix = vec![
            vec![0.0, 1.0, 2.0, 3.0],
            vec![1.0, 0.0, 4.0, 5.0],
            vec![2.0, 4.0, 0.0, 6.0],
            vec![3.0, 5.0, 6.0, 0.0],
        ];
        let problem = TspProblem::with_distance_matrix(matrix);

        let solution = problem.create_solution(&mut Random::new(42));
        assert_eq!(solution.num_variables(), 4);
        assert!(problem.is_valid_permutation(solution.variables()));
    }

    #[test]
    fn tsp_improvement_direction_is_minimize() {
        let problem = TspProblem::with_distance_matrix(vec![vec![0.0]]);
        assert_eq!(
            problem.get_improvement_direction(),
            ImprovementDirection::Minimize
        );
    }

    #[test]
    fn create_solution_respects_fixed_start_city() {
        let matrix = vec![
            vec![0.0, 2.0, 3.0, 4.0],
            vec![2.0, 0.0, 1.0, 5.0],
            vec![3.0, 1.0, 0.0, 6.0],
            vec![4.0, 5.0, 6.0, 0.0],
        ];

        let problem = TspProblem::with_distance_matrix(matrix).with_fixed_start_city(2);
        let solution = problem.create_solution(&mut Random::new(123));

        assert_eq!(solution.get_variable(0).copied(), Some(2));
        assert!(problem.is_valid_permutation(solution.variables()));
    }

    #[test]
    fn fixed_start_city_invalidates_routes_with_different_first_city() {
        let matrix = vec![
            vec![0.0, 10.0, 15.0],
            vec![10.0, 0.0, 20.0],
            vec![15.0, 20.0, 0.0],
        ];

        let problem = TspProblem::with_distance_matrix(matrix).with_fixed_start_city(1);
        let mut invalid_start = Solution::new(vec![0, 1, 2]);
        problem.evaluate(&mut invalid_start);

        assert_eq!(invalid_start.quality().copied(), Some(f64::INFINITY));
    }

    #[test]
    #[should_panic(expected = "fixed start city must be a valid city index")]
    fn fixed_start_city_out_of_range_panics() {
        let _ = TspProblem::with_distance_matrix(vec![vec![0.0, 1.0], vec![1.0, 0.0]])
            .with_fixed_start_city(2);
    }
}
