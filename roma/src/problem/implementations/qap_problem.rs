use crate::problem::traits::Problem;
use crate::solution::Solution;
use crate::utils::random::Random;

/// Quadratic Assignment Problem (QAP).
///
/// A solution is a permutation where index `i` represents the facility and the stored value
/// is the assigned location. The objective is to minimize:
/// ```text
/// sum(flow[i][j] * distance[assignment[i]][assignment[j]])
/// ```
#[derive(Clone)]
pub struct QapProblem {
    description: String,
    flow_matrix: Vec<Vec<f64>>,
    distance_matrix: Vec<Vec<f64>>,
}

impl QapProblem {
    pub fn with_matrices(flow_matrix: Vec<Vec<f64>>, distance_matrix: Vec<Vec<f64>>) -> Self {
        assert!(!flow_matrix.is_empty(), "flow_matrix must contain at least one facility");
        assert_eq!(
            flow_matrix.len(),
            distance_matrix.len(),
            "flow_matrix and distance_matrix must have the same size"
        );

        let size = flow_matrix.len();
        for row in &flow_matrix {
            assert_eq!(row.len(), size, "flow_matrix must be square (NxN)");
        }
        for row in &distance_matrix {
            assert_eq!(row.len(), size, "distance_matrix must be square (NxN)");
        }

        Self {
            description: format!("QAP with {} facilities", size),
            flow_matrix,
            distance_matrix,
        }
    }

    pub fn size(&self) -> usize {
        self.flow_matrix.len()
    }

    pub fn flow(&self, from: usize, to: usize) -> f64 {
        self.flow_matrix[from][to]
    }

    pub fn distance(&self, from: usize, to: usize) -> f64 {
        self.distance_matrix[from][to]
    }

    fn is_valid_assignment(&self, assignment: &[usize]) -> bool {
        let n = self.size();
        if assignment.len() != n {
            return false;
        }

        let mut seen = vec![false; n];
        for &location in assignment {
            if location >= n || seen[location] {
                return false;
            }
            seen[location] = true;
        }

        true
    }

    fn assignment_cost(&self, assignment: &[usize]) -> f64 {
        let mut total = 0.0;
        for i in 0..assignment.len() {
            for j in 0..assignment.len() {
                total += self.flow(i, j) * self.distance(assignment[i], assignment[j]);
            }
        }
        total
    }
}

impl Problem<usize> for QapProblem {
    fn new() -> Self {
        Self::with_matrices(vec![vec![0.0]], vec![vec![0.0]])
    }

    fn evaluate(&self, solution: &mut Solution<usize>) {
        let quality = if self.is_valid_assignment(solution.variables()) {
            self.assignment_cost(solution.variables())
        } else {
            f64::INFINITY
        };
        solution.set_quality(quality);
    }

    fn create_solution(&self, rng: &mut Random) -> Solution<usize> {
        let n = self.size();
        let mut assignment: Vec<usize> = (0..n).collect();

        for i in (1..n).rev() {
            let j = rng.range_between(0, (i + 1) as u64) as usize;
            assignment.swap(i, j);
        }

        Solution::new(assignment)
    }

    fn set_problem_description(&mut self, description: String) {
        self.description = description;
    }

    fn get_problem_description(&self) -> String {
        self.description.clone()
    }

    fn dominates(&self, solution_a: &Solution<usize>, solution_b: &Solution<usize>) -> bool {
        let fitness_a = solution_a.quality().copied().unwrap_or(f64::INFINITY);
        let fitness_b = solution_b.quality().copied().unwrap_or(f64::INFINITY);
        fitness_a < fitness_b
    }

    fn better_fitness_fn(&self) -> fn(f64, f64) -> bool {
        crate::solution::traits::evaluator::minimizing_fitness
    }

    fn format_solution(&self, solution: &Solution<usize>) -> String {
        let assignment = solution.variables();
        let valid = self.is_valid_assignment(assignment);
        let assignment_text = if assignment.is_empty() {
            "[]".to_string()
        } else {
            format!(
                "[{}]",
                assignment
                    .iter()
                    .enumerate()
                    .map(|(facility, location)| format!("{}->{}", facility, location))
                    .collect::<Vec<_>>()
                    .join(", ")
            )
        };
        let cost_text = if valid {
            format!("{:.3}", self.assignment_cost(assignment))
        } else {
            "invalid".to_string()
        };
        let quality_text = solution
            .try_quality_value()
            .map(|quality| format!("{:.3}", quality))
            .unwrap_or_else(|| "not evaluated".to_string());

        format!(
            "facilities={}, valid={}, assignment={}, cost={}, quality={}",
            self.size(),
            valid,
            assignment_text,
            cost_text,
            quality_text
        )
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn qap_creation_uses_requested_size() {
        let problem = QapProblem::with_matrices(
            vec![vec![0.0, 2.0], vec![2.0, 0.0]],
            vec![vec![0.0, 3.0], vec![3.0, 0.0]],
        );
        let solution = problem.create_solution(&mut Random::new(11));
        let mut assignment = solution.variables().to_vec();
        assignment.sort_unstable();

        assert_eq!(problem.size(), 2);
        assert_eq!(assignment, vec![0, 1]);
    }

    #[test]
    fn qap_evaluates_known_assignment_cost() {
        let problem = QapProblem::with_matrices(
            vec![vec![0.0, 5.0], vec![2.0, 0.0]],
            vec![vec![0.0, 3.0], vec![7.0, 0.0]],
        );
        let mut solution: Solution<usize> = Solution::new(vec![1, 0]);

        problem.evaluate(&mut solution);

        assert_eq!(solution.try_quality_value(), Some(41.0));
    }

    #[test]
    fn qap_invalid_assignment_is_penalized() {
        let problem = QapProblem::with_matrices(
            vec![vec![0.0, 1.0], vec![1.0, 0.0]],
            vec![vec![0.0, 1.0], vec![1.0, 0.0]],
        );
        let mut solution: Solution<usize> = Solution::new(vec![0, 0]);

        problem.evaluate(&mut solution);

        assert_eq!(solution.try_quality_value(), Some(f64::INFINITY));
    }

    #[test]
    fn format_solution_reports_assignment_and_quality() {
        let problem = QapProblem::with_matrices(
            vec![vec![0.0, 1.0], vec![1.0, 0.0]],
            vec![vec![0.0, 2.0], vec![2.0, 0.0]],
        );
        let mut solution: Solution<usize> = Solution::new(vec![0, 1]);
        problem.evaluate(&mut solution);

        let formatted = problem.format_solution(&solution);

        assert!(formatted.contains("facilities=2"));
        assert!(formatted.contains("assignment=[0->0, 1->1]"));
        assert!(formatted.contains("quality="));
    }

    #[test]
    #[should_panic(expected = "flow_matrix must contain at least one facility")]
    fn qap_rejects_empty_matrices() {
        QapProblem::with_matrices(Vec::new(), Vec::new());
    }

    #[test]
    #[should_panic(expected = "flow_matrix and distance_matrix must have the same size")]
    fn qap_rejects_different_sizes() {
        QapProblem::with_matrices(vec![vec![0.0]], vec![vec![0.0], vec![0.0]]);
    }
}