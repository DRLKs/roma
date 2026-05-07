use roma_lib::problem::Problem;
use roma_lib::solution::Solution;
use roma_lib::utils::Random;

struct DefaultFormattingProblem {
    description: String,
}

impl Problem<i32> for DefaultFormattingProblem {
    fn new() -> Self {
        Self {
            description: "Default formatting problem".to_string(),
        }
    }

    fn evaluate(&self, solution: &mut Solution<i32>) {
        let quality = solution.variables().iter().sum::<i32>() as f64;
        solution.set_quality(quality);
    }

    fn create_solution(&self, _rng: &mut Random) -> Solution<i32> {
        Solution::new(vec![1, 2, 3])
    }

    fn set_problem_description(&mut self, description: String) {
        self.description = description;
    }

    fn get_problem_description(&self) -> String {
        self.description.clone()
    }

    fn dominates(&self, solution_a: &Solution<i32>, solution_b: &Solution<i32>) -> bool {
        solution_a.quality().copied().unwrap_or(f64::NEG_INFINITY)
            > solution_b.quality().copied().unwrap_or(f64::NEG_INFINITY)
    }

    fn better_fitness_fn(&self) -> fn(f64, f64) -> bool {
        roma_lib::problem::maximizing_fitness
    }
}

#[test]
fn default_solution_format_includes_variables_and_quality_state() {
    let problem = DefaultFormattingProblem::new();

    let not_evaluated = Solution::new(vec![1, 2, 3]);
    let rendered_not_evaluated = problem.format_solution(&not_evaluated);
    assert_eq!(rendered_not_evaluated, "variables=3, quality=not evaluated");

    let mut evaluated = Solution::new(vec![1, 2, 3]);
    problem.evaluate(&mut evaluated);
    let rendered_evaluated = problem.format_solution(&evaluated);
    assert_eq!(rendered_evaluated, "variables=3, quality=evaluated");
}
