use roma_lib::algorithms::{
    Algorithm, HillClimbing, HillClimbingParameters, TerminationCriteria, TerminationCriterion,
};
use roma_lib::observer::{AlgorithmEvent, AlgorithmObserver, Observable};
use roma_lib::operator::BitFlipNeighborhood;
use roma_lib::problem::Problem;
use roma_lib::solution::Solution;
use roma_lib::utils::Random;
use std::sync::{Arc, Mutex};

struct CapturePresentationObserver {
    rendered_solutions: Arc<Mutex<Vec<String>>>,
}

impl CapturePresentationObserver {
    fn new(rendered_solutions: Arc<Mutex<Vec<String>>>) -> Self {
        Self { rendered_solutions }
    }
}

impl AlgorithmObserver<bool> for CapturePresentationObserver {
    fn update(&mut self, event: &AlgorithmEvent<bool>) {
        if let AlgorithmEvent::ExecutionStateUpdated { state } = event {
            self.rendered_solutions
                .lock()
                .expect("observer buffer lock should not be poisoned")
                .push(state.best_solution_presentation.clone());
        }
    }

    fn name(&self) -> &str {
        "CapturePresentationObserver"
    }
}

struct FormattedBinaryProblem {
    description: String,
    variables: usize,
}

impl Problem<bool> for FormattedBinaryProblem {
    fn new() -> Self {
        Self {
            description: "Formatted binary problem".to_string(),
            variables: 8,
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
        let quality = solution.try_quality_value().unwrap_or(0.0);
        format!(
            "selected={}/{}, quality={:.3}",
            selected, self.variables, quality
        )
    }
}

#[test]
fn observer_receives_problem_specific_solution_presentation() {
    let problem = FormattedBinaryProblem::new();
    let parameters = HillClimbingParameters::new(
        BitFlipNeighborhood::new(),
        TerminationCriteria::new(vec![TerminationCriterion::MaxIterations(4)]),
    )
    .with_seed(42);
    let mut algorithm = HillClimbing::new(parameters);

    let rendered_solutions = Arc::new(Mutex::new(Vec::<String>::new()));
    algorithm.add_observer(Box::new(CapturePresentationObserver::new(
        rendered_solutions.clone(),
    )));

    let _ = algorithm
        .run(&problem)
        .expect("hill climbing execution should succeed");

    let rendered = rendered_solutions
        .lock()
        .expect("observer buffer lock should not be poisoned");
    assert!(
        !rendered.is_empty(),
        "observer should receive at least one snapshot"
    );
    assert!(
        rendered.iter().all(|line| line.starts_with("selected=")),
        "all snapshots should use problem-specific rendering"
    );
    assert!(
        rendered.iter().all(|line| line.contains("quality=")),
        "rendered output should include quality"
    );
}
