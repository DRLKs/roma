use roma_lib::prelude::Random;
use roma_lib::{
    AckleyProblem, Algorithm, BeamSearch, BeamSearchParameters, GaussianNeighborhood,
    NeighborhoodOperator, Operator, Problem, RealBounds, Solution, SolutionComparison, SolutionSet,
    TerminationCriteria, TerminationCriterion,
};

#[derive(Clone)]
struct IncrementNeighborhood;

impl Operator for IncrementNeighborhood {
    fn name(&self) -> &str {
        "IncrementNeighborhood"
    }
}

impl NeighborhoodOperator<usize> for IncrementNeighborhood {
    fn random_neighbor(
        &self,
        solution: &Solution<usize>,
        _bounds: Option<&RealBounds>,
        _rng: &mut Random,
    ) -> Solution<usize> {
        Solution::new(vec![(solution.variables()[0] + 1).min(5)])
    }

    fn all_neighbors(
        &self,
        solution: &Solution<usize>,
        _bounds: Option<&RealBounds>,
    ) -> Option<Vec<Solution<usize>>> {
        let value = solution.variables()[0];
        Some(if value < 5 {
            vec![Solution::new(vec![value + 1])]
        } else {
            Vec::new()
        })
    }
}

struct CountingProblem;

impl Problem<usize> for CountingProblem {
    fn new() -> Self {
        Self
    }

    fn evaluate(&self, solution: &mut Solution<usize>) {
        solution.set_quality(solution.variables()[0] as f64);
    }

    fn create_solution(&self, _rng: &mut Random) -> Solution<usize> {
        Solution::new(vec![0])
    }

    fn set_problem_description(&mut self, _description: String) {}

    fn get_problem_description(&self) -> String {
        "maximize a bounded counter".to_string()
    }

    fn compare_qualities(&self, left: Option<&f64>, right: Option<&f64>) -> SolutionComparison {
        roma_lib::problem::compare_scalar_qualities(left, right, |a, b| a > b)
    }

    fn better_fitness_fn(&self) -> fn(f64, f64) -> bool {
        |candidate, reference| candidate > reference
    }
}

#[derive(Clone)]
struct DecrementNeighborhood;

impl Operator for DecrementNeighborhood {
    fn name(&self) -> &str {
        "DecrementNeighborhood"
    }
}

impl NeighborhoodOperator<usize> for DecrementNeighborhood {
    fn random_neighbor(
        &self,
        solution: &Solution<usize>,
        _bounds: Option<&RealBounds>,
        _rng: &mut Random,
    ) -> Solution<usize> {
        Solution::new(vec![solution.variables()[0].saturating_sub(1)])
    }

    fn all_neighbors(
        &self,
        solution: &Solution<usize>,
        _bounds: Option<&RealBounds>,
    ) -> Option<Vec<Solution<usize>>> {
        Some(vec![Solution::new(vec![
            solution.variables()[0].saturating_sub(1),
        ])])
    }
}

struct MinimizingCountingProblem;

impl Problem<usize> for MinimizingCountingProblem {
    fn new() -> Self {
        Self
    }

    fn evaluate(&self, solution: &mut Solution<usize>) {
        solution.set_quality(solution.variables()[0] as f64);
    }

    fn create_solution(&self, _rng: &mut Random) -> Solution<usize> {
        Solution::new(vec![5])
    }

    fn set_problem_description(&mut self, _description: String) {}

    fn get_problem_description(&self) -> String {
        "minimize a bounded counter".to_string()
    }

    fn compare_qualities(&self, left: Option<&f64>, right: Option<&f64>) -> SolutionComparison {
        roma_lib::problem::compare_scalar_qualities(left, right, |a, b| a < b)
    }

    fn better_fitness_fn(&self) -> fn(f64, f64) -> bool {
        |candidate, reference| candidate < reference
    }
}

#[test]
fn beam_search_expands_and_keeps_the_best_solutions() {
    let parameters = BeamSearchParameters::new(
        IncrementNeighborhood,
        3,
        TerminationCriteria::new(vec![TerminationCriterion::MaxIterations(5)]),
    )
    .with_seed(9);
    let mut search = BeamSearch::new(parameters);

    let result = search
        .run(&CountingProblem)
        .expect("beam search should complete");

    assert_eq!(result.size(), 3);
    assert_eq!(
        result
            .best_solution(&CountingProblem)
            .expect("beam should not be empty")
            .quality()
            .copied(),
        Some(5.0)
    );
    assert!(result.iter().all(|solution| solution.has_quality()));
}

#[test]
fn beam_search_honors_minimization_ordering() {
    let parameters = BeamSearchParameters::new(
        DecrementNeighborhood,
        2,
        TerminationCriteria::new(vec![TerminationCriterion::MaxIterations(5)]),
    );
    let result = BeamSearch::new(parameters)
        .run(&MinimizingCountingProblem)
        .expect("minimizing beam search should complete");

    assert_eq!(
        result
            .best_solution(&MinimizingCountingProblem)
            .expect("beam should not be empty")
            .quality()
            .copied(),
        Some(0.0)
    );
}

#[test]
fn beam_search_handles_a_solution_with_no_successors() {
    let parameters = BeamSearchParameters::new(
        IncrementNeighborhood,
        2,
        TerminationCriteria::new(vec![TerminationCriterion::MaxIterations(8)]),
    )
    .with_seed(2);
    let mut search = BeamSearch::new(parameters);

    let result = search
        .run(&CountingProblem)
        .expect("an exhausted finite neighborhood must not panic");

    assert_eq!(result.size(), 2);
    assert_eq!(
        result
            .best_solution(&CountingProblem)
            .expect("beam should remain populated")
            .quality()
            .copied(),
        Some(5.0)
    );
}

#[test]
fn beam_search_samples_non_enumerable_continuous_neighborhoods() {
    let problem = AckleyProblem::new(4, -2.0, 2.0);
    let parameters = BeamSearchParameters::new(
        GaussianNeighborhood::new(0.2),
        4,
        TerminationCriteria::new(vec![TerminationCriterion::MaxIterations(6)]),
    )
    .with_candidates_per_solution(3)
    .with_seed(17);
    let mut search = BeamSearch::new(parameters);

    let result = search
        .run(&problem)
        .expect("beam search should sample a continuous neighborhood");

    assert_eq!(result.size(), 4);
    assert!(result.iter().all(|solution| {
        solution.quality_value().is_finite()
            && solution
                .variables()
                .iter()
                .all(|value| (-2.0..=2.0).contains(value))
    }));
}

#[test]
fn beam_search_is_reproducible_with_a_fixed_seed() {
    let problem = AckleyProblem::new(5, -3.0, 3.0);
    let parameters = BeamSearchParameters::new(
        GaussianNeighborhood::new(0.25),
        3,
        TerminationCriteria::new(vec![TerminationCriterion::MaxIterations(5)]),
    )
    .with_candidates_per_solution(4)
    .with_seed(1234);

    let first = BeamSearch::new(parameters.clone())
        .run(&problem)
        .expect("first seeded run should succeed");
    let second = BeamSearch::new(parameters)
        .run(&problem)
        .expect("second seeded run should succeed");

    let first_solutions: Vec<_> = first
        .iter()
        .map(|solution| (solution.variables().to_vec(), solution.quality().copied()))
        .collect();
    let second_solutions: Vec<_> = second
        .iter()
        .map(|solution| (solution.variables().to_vec(), solution.quality().copied()))
        .collect();

    assert_eq!(first_solutions, second_solutions);
}
