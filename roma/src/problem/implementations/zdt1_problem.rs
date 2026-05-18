use crate::problem::Problem;
use crate::solution::RealBounds;
use crate::solution::implementations::pareto_crowding_solution::MultiObjectiveRealSolutionBuilder;
use crate::solution::traits::ParetoCrowdingDistanceQuality;
use crate::solution::Solution;
use crate::utils::random::Random;

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
    bounds: RealBounds,
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
            bounds: RealBounds::uniform(0.0, 1.0, number_of_variables),
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

    pub fn number_of_variables(&self) -> usize {
        self.number_of_variables
    }
}

impl Problem<f64, ParetoCrowdingDistanceQuality> for ZDT1Problem {
    fn new() -> Self {
        Self::new_default()
    }

    fn evaluate(&self, solution: &mut Solution<f64, ParetoCrowdingDistanceQuality>) {
        let variables = solution.variables();
        let objectives = self.evaluate_objectives(variables);
        solution.set_objectives(objectives);
    }

    fn dominates(&self, solution_a: &Solution<f64, ParetoCrowdingDistanceQuality>, solution_b: &Solution<f64, ParetoCrowdingDistanceQuality>) -> bool {
        solution_a.dominates(solution_b)
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

    fn create_solution(&self, _rng: &mut Random) -> Solution<f64, ParetoCrowdingDistanceQuality> {
        let variables: Vec<f64> = (0..self.number_of_variables)
            .map(|_| _rng.next_f64())
            .collect();

        MultiObjectiveRealSolutionBuilder::from_variables(variables)
            .with_bounds(0.0, 1.0)
            .build()
    }

    fn better_fitness_fn(&self) -> fn(f64, f64) -> bool {
        fn minimizing_fitness(candidate: f64, reference: f64) -> bool {
            candidate < reference
        }
        minimizing_fitness
    }

    fn format_solution(&self, solution: &Solution<f64, ParetoCrowdingDistanceQuality>) -> String {
        let objectives = solution.get_objectives();
        let objectives_text = match objectives {
            Some(values) if !values.is_empty() => {
                let rendered: Vec<String> =
                    values.iter().map(|value| format!("{:.6}", value)).collect();
                format!("[{}]", rendered.join(", "))
            }
            _ => "not evaluated".to_string(),
        };

        let rank_text = solution
            .rank()
            .map(|rank| rank.to_string())
            .unwrap_or_else(|| "none".to_string());

        let crowding_text = solution
            .crowding_distance()
            .map(|distance| format!("{:.6}", distance))
            .unwrap_or_else(|| "none".to_string());

        format!(
            "variables={}, objectives={}, rank={}, crowding_distance={}",
            solution.num_variables(),
            objectives_text,
            rank_text,
            crowding_text
        )
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_zdt1_creation() {
        let problem = ZDT1Problem::new(30);
        let solution = problem.create_solution(&mut Random::new(10));
        assert_eq!(solution.num_variables(), 30);
    }

    #[test]
    fn test_zdt1_evaluation() {
        let problem = ZDT1Problem::new(30);
        let mut solution = problem.create_solution(&mut Random::new(10));

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

        let mut solution = MultiObjectiveRealSolutionBuilder::from_variables(variables).build();
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

    #[test]
    fn zdt1_uses_minimizing_fitness() {
        let problem = ZDT1Problem::new(30);
        assert!(problem.is_better_fitness(0.25, 0.75));
        assert!(!problem.is_better_fitness(0.75, 0.25));
    }

    #[test]
    fn format_solution_reports_objective_metadata() {
        let problem = ZDT1Problem::new(3);
        let mut solution =
            MultiObjectiveRealSolutionBuilder::from_variables(vec![0.2, 0.4, 0.6]).build();
        problem.evaluate(&mut solution);
        solution.set_rank(0);
        solution.set_crowding_distance(1.25);

        let formatted = problem.format_solution(&solution);

        assert!(formatted.contains("variables=3"));
        assert!(formatted.contains("objectives=[0.200000, "));
        assert!(formatted.contains("rank=0"));
        assert!(formatted.contains("crowding_distance=1.250000"));
    }

    #[test]
    fn format_solution_marks_not_evaluated_when_objectives_missing() {
        let problem = ZDT1Problem::new(3);
        let solution =
            MultiObjectiveRealSolutionBuilder::from_variables(vec![0.1, 0.2, 0.3]).build();

        let formatted = problem.format_solution(&solution);

        assert!(formatted.contains("objectives=not evaluated"));
        assert!(formatted.contains("rank=none"));
        assert!(formatted.contains("crowding_distance=none"));
    }
}
