use roma::{
    Algorithm,
    HillClimbing,
    HillClimbingParameters,
    SimulatedAnnealing,
    SimulatedAnnealingParameters,
    SolutionSet,
    SwapMutation,
    TerminationCriteria,
    TerminationCriterion,
    TspProblem,
};

#[test]
fn hill_climbing_runs_on_tsp_and_returns_valid_distance() {
    let problem = TspProblem::with_distance_matrix(vec![
        vec![0.0, 8.0, 6.0, 4.0],
        vec![8.0, 0.0, 7.0, 5.0],
        vec![6.0, 7.0, 0.0, 3.0],
        vec![4.0, 5.0, 3.0, 0.0],
    ]);

    let parameters = HillClimbingParameters::new(
        SwapMutation::new(),
        0.25,
        TerminationCriteria::new(vec![TerminationCriterion::MaxIterations(50)]),
    )
    .with_seed(42);

    let mut algorithm = HillClimbing::new(parameters);
    let result = algorithm.run(&problem).expect("Hill Climbing on TSP should succeed");

    assert_eq!(result.size(), 1);
    let solution = result.get(0).expect("Expected one solution");
    assert_eq!(solution.num_variables(), 4);
    assert!(solution.quality_value().is_finite());
}

#[test]
fn simulated_annealing_runs_on_tsp_open_route() {
    let problem = TspProblem::with_distance_matrix(vec![
        vec![0.0, 9.0, 4.0, 7.0, 8.0],
        vec![9.0, 0.0, 5.0, 6.0, 3.0],
        vec![4.0, 5.0, 0.0, 2.0, 6.0],
        vec![7.0, 6.0, 2.0, 0.0, 1.0],
        vec![8.0, 3.0, 6.0, 1.0, 0.0],
    ])
    .with_open_route();

    let parameters = SimulatedAnnealingParameters::new(
        SwapMutation::new(),
        0.3,
        60.0,
        0.99,
        TerminationCriteria::new(vec![TerminationCriterion::MaxIterations(60)]),
    )
    .with_seed(7);

    let mut algorithm = SimulatedAnnealing::new(parameters);
    let result = algorithm
        .run(&problem)
        .expect("Simulated Annealing on TSP should succeed");

    assert_eq!(result.size(), 1);
    let solution = result.get(0).expect("Expected one solution");
    assert_eq!(solution.num_variables(), 5);
    assert!(solution.quality_value().is_finite());
}

#[test]
fn hill_climbing_respects_fixed_start_city_constraint() {
    let fixed_start_city = 3usize;
    let problem = TspProblem::with_distance_matrix(vec![
        vec![0.0, 8.0, 6.0, 4.0, 7.0],
        vec![8.0, 0.0, 7.0, 5.0, 9.0],
        vec![6.0, 7.0, 0.0, 3.0, 2.0],
        vec![4.0, 5.0, 3.0, 0.0, 1.0],
        vec![7.0, 9.0, 2.0, 1.0, 0.0],
    ])
    .with_fixed_start_city(fixed_start_city);

    let parameters = HillClimbingParameters::new(
        SwapMutation::new(),
        0.0,
        TerminationCriteria::new(vec![TerminationCriterion::MaxIterations(20)]),
    )
    .with_seed(99);

    let mut algorithm = HillClimbing::new(parameters);
    let result = algorithm
        .run(&problem)
        .expect("Hill Climbing with fixed-start TSP should succeed");

    let solution = result.get(0).expect("Expected one solution");
    assert_eq!(solution.get_variable(0).copied(), Some(fixed_start_city));
    assert!(solution.quality_value().is_finite());
}
