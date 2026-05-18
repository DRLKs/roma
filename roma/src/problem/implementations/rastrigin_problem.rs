use std::f64::consts::PI;

use crate::problem::Problem;
use crate::solution::RealBounds;
use crate::solution::{RealSolutionBuilder, Solution};
use crate::utils::random::Random;

const DEFAULT_NUMBER_OF_VARIABLES: usize = 30;
const DEFAULT_LOWER_BOUND: f64 = -5.12;
const DEFAULT_UPPER_BOUND: f64 = 5.12;

/// Rastrigin: a classic multimodal continuous minimization benchmark.
///
/// Minimize:
/// f(x) = 10n + sum(x_i^2 - 10 cos(2 pi x_i))
///
/// Variables: x_i in [lower_bound, upper_bound]
/// Global optimum: f(0, ..., 0) = 0
#[derive(Clone)]
pub struct RastriginProblem {
    number_of_variables: usize,
    lower_bound: f64,
    upper_bound: f64,
    bounds: RealBounds,
    description: String,
}

impl RastriginProblem {
    pub fn new(number_of_variables: usize, lower_bound: f64, upper_bound: f64) -> Self {
        assert!(
            number_of_variables > 0,
            "Rastrigin requires at least 1 variable"
        );
        assert!(
            lower_bound < upper_bound,
            "Rastrigin lower_bound must be smaller than upper_bound"
        );

        Self {
            number_of_variables,
            lower_bound,
            upper_bound,
            bounds: RealBounds::uniform(lower_bound, upper_bound, number_of_variables),
            description: format!("Rastrigin problem with {} variables", number_of_variables),
        }
    }

    pub fn new_default() -> Self {
        Self::new(
            DEFAULT_NUMBER_OF_VARIABLES,
            DEFAULT_LOWER_BOUND,
            DEFAULT_UPPER_BOUND,
        )
    }

    pub fn number_of_variables(&self) -> usize {
        self.number_of_variables
    }

    pub fn lower_bound(&self) -> f64 {
        self.lower_bound
    }

    pub fn upper_bound(&self) -> f64 {
        self.upper_bound
    }

    fn evaluate_variables(&self, variables: &[f64]) -> f64 {
        let n = variables.len() as f64;
        10.0 * n
            + variables
                .iter()
                .map(|x| x * x - 10.0 * (2.0 * PI * x).cos())
                .sum::<f64>()
    }
}

impl Problem<f64> for RastriginProblem {
    fn new() -> Self {
        Self::new_default()
    }

    fn evaluate(&self, solution: &mut Solution<f64>) {
        let value = self.evaluate_variables(solution.variables());
        solution.set_quality(value);
    }

    /// The solution that dominates is the one who is near to zero
    fn dominates(&self, solution_a: &Solution<f64, f64>, solution_b: &Solution<f64, f64>) -> bool {
        let quality_a = solution_a.quality().copied().unwrap_or(f64::INFINITY);
        let quality_b = solution_b.quality().copied().unwrap_or(f64::INFINITY);
        quality_a.abs() < quality_b.abs()
    }

    fn better_fitness_fn(&self) -> fn(f64, f64) -> bool {
        fn nerar_to_zero_fitness(candidate: f64, reference: f64) -> bool {
            candidate.abs() < reference.abs()
        }
        nerar_to_zero_fitness
    }

    fn create_solution(&self, rng: &mut Random) -> Solution<f64> {
        let span = self.upper_bound - self.lower_bound;
        let variables: Vec<f64> = (0..self.number_of_variables)
            .map(|_| self.lower_bound + rng.next_f64() * span)
            .collect();

        RealSolutionBuilder::from_variables(variables)
            .with_bounds(self.lower_bound, self.upper_bound)
            .build()
    }

    fn set_problem_description(&mut self, description: String) {
        self.description = description;
    }

    fn get_problem_description(&self) -> String {
        self.description.clone()
    }

    fn real_bounds(&self) -> Option<&RealBounds> {
        Some(&self.bounds)
    }

    fn format_solution(&self, solution: &Solution<f64>) -> String {
        let quality_text = solution
            .try_quality_value()
            .map(|value| format!("{:.6}", value))
            .unwrap_or_else(|| "not evaluated".to_string());

        format!(
            "variables={}, bounds=[{:.3}, {:.3}], quality={}",
            solution.num_variables(),
            self.lower_bound,
            self.upper_bound,
            quality_text
        )
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn rastrigin_creation_uses_requested_shape() {
        let problem = RastriginProblem::new(10, -2.5, 3.5);
        let solution = problem.create_solution(&mut Random::new(7));

        assert_eq!(problem.number_of_variables(), 10);
        assert_eq!(solution.num_variables(), 10);
        assert!(solution.variables().iter().all(|value| *value >= -2.5 && *value <= 3.5));
    }

    #[test]
    fn rastrigin_optimum_is_zero_at_origin() {
        let problem = RastriginProblem::new_default();
        let mut solution = RealSolutionBuilder::from_variables(vec![0.0; 30])
            .with_bounds(DEFAULT_LOWER_BOUND, DEFAULT_UPPER_BOUND)
            .build();

        problem.evaluate(&mut solution);

        assert_eq!(solution.try_quality_value(), Some(0.0));
    }

    #[test]
    fn rastrigin_uses_minimizing_fitness() {
        let problem = RastriginProblem::new_default();

        assert!(problem.is_better_fitness(1.0, 5.0));
        assert!(!problem.is_better_fitness(5.0, 1.0));
    }

    #[test]
    fn format_solution_reports_bounds_and_quality() {
        let problem = RastriginProblem::new(3, -5.12, 5.12);
        let mut solution = RealSolutionBuilder::from_variables(vec![0.0, 1.0, 2.0])
            .with_bounds(-5.12, 5.12)
            .build();
        problem.evaluate(&mut solution);

        let formatted = problem.format_solution(&solution);

        assert!(formatted.contains("variables=3"));
        assert!(formatted.contains("bounds=[-5.120, 5.120]"));
        assert!(formatted.contains("quality="));
    }

    #[test]
    #[should_panic(expected = "Rastrigin requires at least 1 variable")]
    fn rastrigin_rejects_zero_variables() {
        RastriginProblem::new(0, -5.12, 5.12);
    }

    #[test]
    #[should_panic(expected = "Rastrigin lower_bound must be smaller than upper_bound")]
    fn rastrigin_rejects_invalid_bounds() {
        RastriginProblem::new(10, 1.0, 1.0);
    }
}