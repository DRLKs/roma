use rmetal::{
    Algorithm, BitFlipMutation, HillClimbing, HillClimbingParameters, KnapsackBuilder, Problem,
    SolutionSet, TerminationCriteria, TerminationCriterion,
};
use std::time::{SystemTime, UNIX_EPOCH};

#[test]
fn hill_climbing_handles_empty_problem_edge_case() {
    // Edge case: a knapsack with zero items must not crash and should return a valid empty solution.
    let problem = KnapsackBuilder::new().with_capacity(100.0).build();

    let parameters = HillClimbingParameters::new(
        BitFlipMutation::new(),
        1.0,
        TerminationCriteria::new(vec![TerminationCriterion::MaxIterations(5)]),
    )
    .with_seed(7);

    let mut algorithm = HillClimbing::new(parameters);
    let result = algorithm
        .run(&problem)
        .expect("Hill Climbing run should succeed");

    assert_eq!(result.size(), 1);
    let solution = result
        .get(0)
        .expect("Hill Climbing should return one solution");
    assert_eq!(solution.num_variables(), 0);
    assert_eq!(solution.quality().copied(), Some(0.0));
}

#[test]
fn hill_climbing_resume_requires_compatible_checkpoint() {
    let unique_id = SystemTime::now()
        .duration_since(UNIX_EPOCH)
        .map(|duration| duration.as_nanos())
        .unwrap_or(0);

    let mut problem = KnapsackBuilder::new()
        .with_capacity(90.0)
        .add_item(12.0, 24.0)
        .add_item(22.0, 33.0)
        .add_item(41.0, 80.0)
        .build();
    problem.set_problem_description(format!("resume_missing_checkpoint_{}", unique_id));

    let parameters = HillClimbingParameters::new(
        BitFlipMutation::new(),
        0.10,
        TerminationCriteria::new(vec![TerminationCriterion::MaxIterations(5)]),
    )
    .with_seed(42)
    .with_resume(true);

    let mut algorithm = HillClimbing::new(parameters);
    let result = algorithm.run(&problem);
    assert!(result.is_err());
    let error = result.err().unwrap_or_default();
    assert!(error.contains("no compatible checkpoint"));
}

#[test]
fn hill_climbing_resume_matches_full_run_result() {
    let unique_id = SystemTime::now()
        .duration_since(UNIX_EPOCH)
        .map(|duration| duration.as_nanos())
        .unwrap_or(0);

    let mut problem = KnapsackBuilder::new()
        .with_capacity(90.0)
        .add_item(12.0, 24.0)
        .add_item(22.0, 33.0)
        .add_item(41.0, 80.0)
        .build();
    problem.set_problem_description(format!("resume_compare_{}", unique_id));

    let baseline_parameters = HillClimbingParameters::new(
        BitFlipMutation::new(),
        0.10,
        TerminationCriteria::new(vec![TerminationCriterion::MaxIterations(12)]),
    )
    .with_seed(77);
    let mut baseline_algorithm = HillClimbing::new(baseline_parameters);
    let baseline = baseline_algorithm
        .run(&problem)
        .expect("baseline run should succeed");
    let baseline_best = baseline
        .best_solution()
        .expect("baseline best solution should exist")
        .quality_value();

    #[derive(Clone)]
    struct PanicAfterMutation {
        calls: std::sync::Arc<std::sync::atomic::AtomicUsize>,
        panic_on_call: usize,
    }

    impl rmetal::Operator for PanicAfterMutation {
        fn name(&self) -> &str {
            "BitFlipMutation"
        }
    }

    impl rmetal::MutationOperator<bool> for PanicAfterMutation {
        fn execute(
            &self,
            solution: &mut rmetal::Solution<bool>,
            probability: f64,
            rng: &mut rmetal::utils::Random,
        ) {
            let call = self.calls.fetch_add(1, std::sync::atomic::Ordering::SeqCst) + 1;
            if call >= self.panic_on_call {
                panic!("forced checkpoint-producing panic");
            }

            rmetal::BitFlipMutation::new().execute(solution, probability, rng);
        }
    }

    let panic_parameters = HillClimbingParameters::new(
        PanicAfterMutation {
            calls: std::sync::Arc::new(std::sync::atomic::AtomicUsize::new(0)),
            panic_on_call: 7,
        },
        0.10,
        TerminationCriteria::new(vec![TerminationCriterion::MaxIterations(12)]),
    )
    .with_seed(77);
    let mut checkpoint_algorithm = HillClimbing::new(panic_parameters);
    let panic_result = std::panic::catch_unwind(std::panic::AssertUnwindSafe(|| {
        let _ = checkpoint_algorithm.run(&problem);
    }));
    assert!(panic_result.is_err());

    let resume_parameters = HillClimbingParameters::new(
        BitFlipMutation::new(),
        0.10,
        TerminationCriteria::new(vec![TerminationCriterion::MaxIterations(12)]),
    )
    .with_seed(77)
    .with_resume(true);
    let mut resumed_algorithm = HillClimbing::new(resume_parameters);
    let resumed = resumed_algorithm
        .run(&problem)
        .expect("resumed run should succeed");
    let resumed_best = resumed
        .best_solution()
        .expect("resumed best solution should exist")
        .quality_value();

    assert_eq!(resumed_best, baseline_best);
}
