use crate::problem::traits::Problem;
use crate::solution::{MultiObjectiveInfo, RealSolutionBuilder, Solution};
use crate::utils::random::{Random, seed_from_time};

const DEFAULT_NUMBER_OF_VARIABLES: usize = 30;

/// ZDT1: A classic bi-objective test problem
///
/// Minimize f1(x) = x1
/// Minimize f2(x) = g(x) * h(f1(x), g(x))
///
/// where:
/// g(x) = 1 + 9 * sum(x_i for i in 2..n) / (n - 1)
/// h(f1, g) = 1 - sqrt(f1 / g)
///
/// Variables: x_i in [0, 1] for i = 1..n
/// Objectives: 2 (both to minimize)
///
/// The Pareto-optimal front is f2 = 1 - sqrt(f1) for f1 in [0, 1]
pub struct ZDT1Problem {
    number_of_variables: usize,
    description: String,
}

impl ZDT1Problem {
    pub fn new(number_of_variables: usize) -> Self {
        assert!(
            number_of_variables >= 2,
            "ZDT1 requires at least 2 variables"
        );
        Self {
            number_of_variables,
            description: format!("ZDT1 problem with {} variables", number_of_variables),
        }
    }

    pub fn new_default() -> Self {
        Self::new(DEFAULT_NUMBER_OF_VARIABLES) // Standard configuration
    }

    fn evaluate_objectives(&self, variables: &[f64]) -> Vec<f64> {
        let f1 = variables[0];

        let g = if variables.len() > 1 {
            let sum: f64 = variables[1..].iter().sum();
            1.0 + 9.0 * sum / (variables.len() - 1) as f64
        } else {
            1.0
        };

        let h = 1.0 - (f1 / g).sqrt();
        let f2 = g * h;

        vec![f1, f2]
    }

    pub fn number_of_variables(&self) -> usize{
        self.number_of_variables
    }
}

impl Problem<f64, MultiObjectiveInfo> for ZDT1Problem {
    fn new() -> Self {
        Self::new_default()
    }

    fn evaluate(&self, solution: &mut Solution<f64, MultiObjectiveInfo>) {
        let variables = solution.variables();
        let objectives = self.evaluate_objectives(variables);
        solution.set_objectives(objectives);
    }

    fn create_solution(&self) -> Solution<f64, MultiObjectiveInfo> {
        let mut rng = Random::new(seed_from_time());
        let variables: Vec<f64> = (0..self.number_of_variables)
            .map(|_| rng.next_f64())
            .collect();

        RealSolutionBuilder::from_variables(variables)
            .with_bounds(0.0, 1.0)
            .into_multi_objective()
            .build()
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
    fn test_zdt1_creation() {
        let problem = ZDT1Problem::new(30);
        let solution = problem.create_solution();
        assert_eq!(solution.num_variables(), 30);
    }

    #[test]
    fn test_zdt1_evaluation() {
        let problem = ZDT1Problem::new(30);
        let mut solution = problem.create_solution();

        problem.evaluate(&mut solution);

        let objectives = solution.get_objectives().unwrap();
        assert_eq!(objectives.len(), 2);
        assert!(objectives[0] >= 0.0 && objectives[0] <= 1.0);
        assert!(objectives[1] >= 0.0);
    }

    #[test]
    fn test_zdt1_pareto_front_point() {
        let problem = ZDT1Problem::new(30);

        // Create one point with known variables and verify deterministic scalar value.
        let mut variables = vec![0.0; 30];
        variables[0] = 0.5;

        let mut solution = RealSolutionBuilder::from_variables(variables)
            .into_multi_objective()
            .build();
        problem.evaluate(&mut solution);

        let objectives = solution.get_objectives().unwrap();
        let f1 = objectives[0];
        let f2 = objectives[1];
        let expected_f2 = 1.0 - f1.sqrt();
        assert!((f2 - expected_f2).abs() < 1e-10);
    }

    #[test]
    #[should_panic(expected = "ZDT1 requires at least 2 variables")]
    fn test_zdt1_invalid_variables() {
        ZDT1Problem::new(1);
    }
}
