use roma_lib::{
    AckleyProblem,
    Algorithm,
    BitFlipNeighborhood,
    GaussianNeighborhood,
    HillClimbing,
    HillClimbingParameters,
    KnapsackBuilder,
    SolutionSet,
    TerminationCriteria,
    TerminationCriterion,
};

#[test]
fn hill_climbing_handles_empty_problem_edge_case() {
    // Edge case: a knapsack with zero items must not crash and should return a valid empty solution.
    let problem = KnapsackBuilder::new().with_capacity(100.0).build();

    let parameters = HillClimbingParameters::new(
        BitFlipNeighborhood::new(),
        TerminationCriteria::new(vec![TerminationCriterion::MaxIterations(5)]),
    )
    .with_seed(7);

    let mut algorithm = HillClimbing::new(parameters);
    let result = algorithm.run(&problem).expect("Hill Climbing run should succeed");

    assert_eq!(result.size(), 1);
    let solution = result
        .get(0)
        .expect("Hill Climbing should return one solution");
    assert_eq!(solution.num_variables(), 0);
    assert_eq!(solution.quality().copied(), Some(0.0));
}

#[test]
fn hill_climbing_runs_with_gaussian_neighborhood() {
    let problem = AckleyProblem::new(6, -5.0, 5.0);

    let parameters = HillClimbingParameters::new(
        GaussianNeighborhood::new(0.3),
        TerminationCriteria::new(vec![TerminationCriterion::MaxIterations(12)]),
    )
    .with_seed(13);

    let mut algorithm = HillClimbing::new(parameters);
    let result = algorithm
        .run(&problem)
        .expect("Hill Climbing with gaussian neighborhood should succeed");

    assert_eq!(result.size(), 1);
    let best = result.get(0).expect("Expected one solution");
    assert_eq!(best.num_variables(), 6);
    assert!(best.variables().iter().all(|value| (-5.0..=5.0).contains(value)));
    assert!(best.quality_value().is_finite());
}
