use roma_lib::{
    Algorithm,
    BitFlipMutation,
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
        BitFlipMutation::new(),
        1.0,
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
