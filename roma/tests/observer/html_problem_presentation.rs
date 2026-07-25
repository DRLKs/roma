use roma_lib::algorithms::{
    Algorithm, HillClimbing, HillClimbingParameters, TerminationCriteria, TerminationCriterion,
};
use roma_lib::observer::{HtmlReportObserver, Observable};
use roma_lib::operator::BitFlipNeighborhood;
use roma_lib::problem::Problem;
use roma_lib::solution::Solution;
use roma_lib::utils::Random;
use std::time::{SystemTime, UNIX_EPOCH};

struct HtmlFormattingProblem {
    description: String,
    variables: usize,
}

impl Problem<bool> for HtmlFormattingProblem {
    fn new() -> Self {
        Self {
            description: "HTML formatting problem".to_string(),
            variables: 6,
        }
    }

    fn evaluate(&self, solution: &mut Solution<bool>) {
        let selected = solution.variables().iter().filter(|&&value| value).count() as f64;
        solution.set_quality(selected);
    }

    fn create_solution(&self, rng: &mut Random) -> Solution<bool> {
        let variables: Vec<bool> = (0..self.variables).map(|_| rng.coin_flip()).collect();
        Solution::new(variables)
    }

    fn set_problem_description(&mut self, description: String) {
        self.description = description;
    }

    fn get_problem_description(&self) -> String {
        self.description.clone()
    }

    fn dominates(&self, solution_a: &Solution<bool>, solution_b: &Solution<bool>) -> bool {
        solution_a.quality().copied().unwrap_or(f64::NEG_INFINITY)
            > solution_b.quality().copied().unwrap_or(f64::NEG_INFINITY)
    }

    fn better_fitness_fn(&self) -> fn(f64, f64) -> bool {
        fn maximizing_fitness(a: f64, b: f64) -> bool {
            a > b
        }
        maximizing_fitness
    }

    fn format_solution(&self, solution: &Solution<bool>) -> String {
        let selected = solution.variables().iter().filter(|&&value| value).count();
        format!("binary-summary: {}/{} selected", selected, self.variables)
    }
}

#[test]
fn html_report_includes_problem_specific_best_solution_rendering() {
    let problem = HtmlFormattingProblem::new();
    let run_id = SystemTime::now()
        .duration_since(UNIX_EPOCH)
        .map(|duration| duration.as_nanos())
        .unwrap_or(0);
    let output_dir = std::env::temp_dir().join(format!("roma_html_rendering_test_{}", run_id));

    let observer = HtmlReportObserver::new(output_dir.clone()).with_flat_output();
    let parameters = HillClimbingParameters::new(
        BitFlipNeighborhood::new(),
        TerminationCriteria::new(vec![TerminationCriterion::MaxIterations(3)]),
    )
    .with_seed(7);
    let mut algorithm = HillClimbing::new(parameters);
    algorithm.add_observer(Box::new(observer));

    let _ = algorithm
        .run(&problem)
        .expect("hill climbing execution should succeed");

    let report_path = output_dir.join("report.html");
    assert!(
        report_path.exists(),
        "html report should exist at {}",
        report_path.display()
    );

    let report = std::fs::read_to_string(&report_path).expect("report should be readable");
    assert!(
        report.contains("binary-summary:"),
        "report should include problem-specific presentation text"
    );
}
