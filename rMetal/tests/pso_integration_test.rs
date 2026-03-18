use rmetal::{
    Algorithm,
    KnapsackBuilder,
    PSO,
    PSOParameters,
    SolutionSet,
    TerminationCriteria,
    TerminationCriterion,
};

#[test]
fn pso_runs_on_knapsack_and_returns_single_solution() {
    let problem = KnapsackBuilder::new()
        .with_capacity(35.0)
        .add_items(vec![(7.0, 11.0), (9.0, 16.0), (12.0, 25.0), (4.0, 7.0), (5.0, 9.0)])
        .build();

    let parameters = PSOParameters::new(
        25,
        0.7,
        1.4,
        1.4,
        TerminationCriteria::new(vec![TerminationCriterion::MaxIterations(35)]),
    )
    .with_seed(77);

    let mut algorithm = PSO::new(parameters);
    let result = algorithm.run(&problem).expect("PSO run should succeed");

    assert_eq!(result.size(), 1);
    let best = result.get(0).expect("PSO should return one best solution");
    assert!(best.quality().is_some());
    assert!(best.quality_value().is_finite());
}

#[test]
fn pso_rejects_zero_swarm_size() {
    let problem = KnapsackBuilder::new().with_capacity(10.0).add_item(5.0, 9.0).build();

    let parameters = PSOParameters::new(
        0,
        0.7,
        1.4,
        1.4,
        TerminationCriteria::new(vec![TerminationCriterion::MaxIterations(10)]),
    )
    .with_seed(7);

    let mut algorithm = PSO::new(parameters);
    let error = match algorithm.run(&problem) {
        Ok(_) => panic!("PSO with zero swarm size must fail validation"),
        Err(message) => message,
    };

    assert!(error.contains("swarm_size"));
}

#[test]
fn pso_rejects_negative_acceleration_coefficients() {
    let problem = KnapsackBuilder::new().with_capacity(10.0).add_item(5.0, 9.0).build();

    let parameters = PSOParameters::new(
        10,
        0.7,
        -1.0,
        1.4,
        TerminationCriteria::new(vec![TerminationCriterion::MaxIterations(10)]),
    )
    .with_seed(7);

    let mut algorithm = PSO::new(parameters);
    let error = match algorithm.run(&problem) {
        Ok(_) => panic!("PSO with negative coefficient must fail validation"),
        Err(message) => message,
    };

    assert!(error.contains("cognitive_coefficient"));
}
