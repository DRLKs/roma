use crate::problem::traits::Problem;
use crate::solution::Solution;
use crate::utils::random::Random;
use std::collections::{BTreeMap, HashMap};

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
    /// Optional city coordinates aligned by city index.
    ///
    /// When present, `city_positions[i]` stores the `(x, y)` position of city `i`.
    city_positions: Option<Vec<(f64, f64)>>,
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
    fn rounded_euclidean_distance(from: (f64, f64), to: (f64, f64)) -> f64 {
        let dx = from.0 - to.0;
        let dy = from.1 - to.1;
        (dx.mul_add(dx, dy * dy)).sqrt().round()
    }

    /// Creates a TSP instance directly from city coordinates using TSPLIB EUC_2D rounding.
    pub fn from_city_positions(city_positions: Vec<(f64, f64)>) -> Self {
        assert!(
            !city_positions.is_empty(),
            "city_positions must contain at least one city"
        );

        let size = city_positions.len();
        let mut distance_matrix = vec![vec![0.0; size]; size];
        for i in 0..size {
            for j in i + 1..size {
                let distance = Self::rounded_euclidean_distance(city_positions[i], city_positions[j]);
                distance_matrix[i][j] = distance;
                distance_matrix[j][i] = distance;
            }
        }

        Self {
            description: format!("TSP with {} cities", size),
            distance_matrix,
            city_positions: Some(city_positions),
            close_tour: true,
            fixed_start_city: None,
        }
    }

    /// Creates a TSP instance from a square distance matrix.
    pub fn with_distance_matrix(distance_matrix: Vec<Vec<f64>>) -> Self {
        assert!(
            !distance_matrix.is_empty(),
            "distance_matrix must contain at least one city"
        );

        let size = distance_matrix.len();
        for row in &distance_matrix {
            assert_eq!(row.len(), size, "distance_matrix must be square (NxN)");
        }

        Self {
            description: format!("TSP with {} cities", size),
            distance_matrix,
            city_positions: None,
            close_tour: true,
            fixed_start_city: None,
        }
    }

    /// Attaches explicit city coordinates aligned with the distance matrix indexes.
    pub fn with_city_positions(mut self, city_positions: Vec<(f64, f64)>) -> Self {
        assert_eq!(
            city_positions.len(),
            self.number_of_cities(),
            "city_positions must have one entry per city"
        );
        self.city_positions = Some(city_positions);
        self
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

    pub fn city_position(&self, city: usize) -> Option<(f64, f64)> {
        self.city_positions
            .as_ref()
            .and_then(|positions| positions.get(city).copied())
    }

    pub fn city_positions(&self) -> Option<&[(f64, f64)]> {
        self.city_positions.as_deref()
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

    fn dominates(&self, solution_a: &Solution<usize, f64>, solution_b: &Solution<usize, f64>) -> bool {
        let fitness_a = solution_a.quality().copied().unwrap_or(f64::INFINITY);
        let fitness_b = solution_b.quality().copied().unwrap_or(f64::INFINITY);
        fitness_a < fitness_b
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

    fn better_fitness_fn(&self) -> fn(f64, f64) -> bool {
        crate::solution::traits::evaluator::minimizing_fitness
    }

    fn format_solution(&self, solution: &Solution<usize>) -> String {
        let route = solution.variables();
        let valid = self.is_valid_permutation(route);
        let topology = if self.close_tour { "closed" } else { "open" };
        let fixed_start = self
            .fixed_start_city
            .map(|city| city.to_string())
            .unwrap_or_else(|| "none".to_string());

        let route_text = if route.is_empty() {
            "[]".to_string()
        } else {
            let mut nodes: Vec<String> = route.iter().map(|city| city.to_string()).collect();
            if self.close_tour {
                nodes.push(route[0].to_string());
            }
            format!("[{}]", nodes.join(" -> "))
        };

        let distance_text = if valid {
            format!("{:.3}", self.route_distance(route))
        } else {
            "invalid".to_string()
        };

        let quality_text = solution
            .try_quality_value()
            .map(|quality| format!("{:.3}", quality))
            .unwrap_or_else(|| "not evaluated".to_string());

        format!(
            "topology={}, fixed_start={}, valid={}, route={}, distance={}, quality={}",
            topology, fixed_start, valid, route_text, distance_text, quality_text
        )
    }
}

/// Builds a `TspProblem` from edge-like records.
///
/// Each record must contain three scalar fields:
/// - `from_key`: source city label
/// - `to_key`: destination city label
/// - `distance_key`: edge distance as `f64`
///
/// City labels are mapped to indexes in deterministic lexicographic order.
/// The resulting matrix must be complete for all non-diagonal pairs; otherwise an error is returned.
///
/// Returns the built problem and the number of loaded edges.
pub fn build_tsp_from_records(
    records: &[HashMap<String, String>],
    from_key: &str,
    to_key: &str,
    distance_key: &str,
) -> Result<(TspProblem, usize), String> {
    if records.is_empty() {
        return Err("Input data has no records".to_string());
    }

    let mut city_labels = BTreeMap::<String, ()>::new();
    let mut edges: Vec<(String, String, f64)> = Vec::new();

    for record in records {
        let Some(from_label) = record.get(from_key) else {
            continue;
        };
        let Some(to_label) = record.get(to_key) else {
            continue;
        };
        let Some(distance_text) = record.get(distance_key) else {
            continue;
        };

        let Ok(distance) = distance_text.parse::<f64>() else {
            continue;
        };

        city_labels.insert(from_label.clone(), ());
        city_labels.insert(to_label.clone(), ());
        edges.push((from_label.clone(), to_label.clone(), distance));
    }

    if edges.is_empty() {
        return Err(format!(
            "No valid edges found. Ensure keys '{}', '{}' and '{}' exist and distance is numeric",
            from_key, to_key, distance_key
        ));
    }

    let city_to_index: BTreeMap<String, usize> = city_labels
        .keys()
        .enumerate()
        .map(|(index, label)| (label.clone(), index))
        .collect();

    let n = city_to_index.len();
    let mut matrix = vec![vec![f64::INFINITY; n]; n];
    for (i, row) in matrix.iter_mut().enumerate() {
        row[i] = 0.0;
    }

    for (from_label, to_label, distance) in &edges {
        let from_index = city_to_index
            .get(from_label)
            .copied()
            .ok_or_else(|| format!("Unknown source city label '{}'", from_label))?;
        let to_index = city_to_index
            .get(to_label)
            .copied()
            .ok_or_else(|| format!("Unknown destination city label '{}'", to_label))?;

        matrix[from_index][to_index] = *distance;
    }

    for (i, row) in matrix.iter().enumerate() {
        for (j, value) in row.iter().enumerate() {
            if i != j && !value.is_finite() {
                return Err("TSP edge set does not define a complete distance matrix".to_string());
            }
        }
    }

    Ok((TspProblem::with_distance_matrix(matrix), edges.len()))
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
    fn from_city_positions_builds_euclidean_distance_matrix() {
        let problem = TspProblem::from_city_positions(vec![(0.0, 0.0), (3.0, 4.0), (6.0, 8.0)]);

        assert_eq!(problem.number_of_cities(), 3);
        assert_eq!(problem.distance(0, 1), 5.0);
        assert_eq!(problem.distance(1, 2), 5.0);
        assert_eq!(problem.distance(0, 2), 10.0);
        assert_eq!(problem.city_position(2), Some((6.0, 8.0)));
    }

    #[test]
    fn city_positions_can_be_retrieved_by_city_index() {
        let matrix = vec![
            vec![0.0, 1.0, 2.0],
            vec![1.0, 0.0, 3.0],
            vec![2.0, 3.0, 0.0],
        ];
        let problem = TspProblem::with_distance_matrix(matrix)
            .with_city_positions(vec![(10.0, 20.0), (30.0, 40.0), (50.0, 60.0)]);

        assert_eq!(problem.city_position(1), Some((30.0, 40.0)));
        assert_eq!(problem.city_position(3), None);
        assert_eq!(problem.city_positions(), Some(&[(10.0, 20.0), (30.0, 40.0), (50.0, 60.0)][..]));
    }

    #[test]
    #[should_panic(expected = "city_positions must have one entry per city")]
    fn city_positions_length_must_match_city_count() {
        let _ = TspProblem::with_distance_matrix(vec![vec![0.0, 1.0], vec![1.0, 0.0]])
            .with_city_positions(vec![(0.0, 0.0)]);
    }

    #[test]
    fn tsp_uses_minimizing_fitness() {
        let problem = TspProblem::with_distance_matrix(vec![vec![0.0]]);
        assert!(problem.is_better_fitness(2.0, 3.0));
        assert!(!problem.is_better_fitness(3.0, 2.0));
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

    #[test]
    fn format_solution_reports_route_and_distance() {
        let matrix = vec![
            vec![0.0, 10.0, 15.0],
            vec![10.0, 0.0, 20.0],
            vec![15.0, 20.0, 0.0],
        ];
        let problem = TspProblem::with_distance_matrix(matrix);
        let mut solution = Solution::new(vec![0, 1, 2]);
        problem.evaluate(&mut solution);

        let formatted = problem.format_solution(&solution);

        assert!(formatted.contains("topology=closed"));
        assert!(formatted.contains("fixed_start=none"));
        assert!(formatted.contains("valid=true"));
        assert!(formatted.contains("route=[0 -> 1 -> 2 -> 0]"));
        assert!(formatted.contains("distance=45.000"));
        assert!(formatted.contains("quality=45.000"));
    }

    #[test]
    fn format_solution_flags_invalid_route() {
        let matrix = vec![vec![0.0, 1.0], vec![1.0, 0.0]];
        let problem = TspProblem::with_distance_matrix(matrix).with_fixed_start_city(1);
        let mut invalid = Solution::new(vec![0, 1]);
        problem.evaluate(&mut invalid);

        let formatted = problem.format_solution(&invalid);

        assert!(formatted.contains("valid=false"));
        assert!(formatted.contains("distance=invalid"));
        assert!(formatted.contains("quality=inf"));
    }
}
