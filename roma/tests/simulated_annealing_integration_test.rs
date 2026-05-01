use roma_lib::{
    Algorithm,
    BitFlipMutation,
    KnapsackBuilder,
    SimulatedAnnealing,
    SimulatedAnnealingParameters,
    SolutionSet,
    TerminationCriteria,
    TerminationCriterion,
};

#[test]
fn simulated_annealing_runs_on_knapsack_and_returns_single_solution() {
    let problem = KnapsackBuilder::new()
        .with_capacity(25.0)
        .add_items(vec![(6.0, 12.0), (7.0, 13.0), (8.0, 15.0), (4.0, 6.0)])
        .build();

    let parameters = SimulatedAnnealingParameters::new(
        BitFlipMutation::new(),
        0.2,
        30.0,
        0.98,
        TerminationCriteria::new(vec![TerminationCriterion::MaxIterations(40)]),
    )
    .with_seed(99);

    let mut algorithm = SimulatedAnnealing::new(parameters);
    let result = algorithm
        .run(&problem)
        .expect("Simulated Annealing run should succeed");

    assert_eq!(result.size(), 1);
    let solution = result.get(0).expect("Expected one final solution");
    assert!(solution.quality().is_some());
    assert!(solution.quality_value().is_finite());
}

#[test]
fn simulated_annealing_rejects_invalid_temperature_configuration() {
    let problem = KnapsackBuilder::new().with_capacity(10.0).add_item(5.0, 10.0).build();

    let parameters = SimulatedAnnealingParameters::new(
        BitFlipMutation::new(),
        0.2,
        1.0,
        0.99,
        TerminationCriteria::new(vec![TerminationCriterion::MaxIterations(10)]),
    )
    .with_minimum_temperature(2.0)
    .with_seed(1);

    let mut algorithm = SimulatedAnnealing::new(parameters);
    let error = match algorithm.run(&problem) {
        Ok(_) => panic!("Configuration with min temperature above initial should fail"),
        Err(message) => message,
    };

    assert!(error.contains("minimum_temperature"));
}
