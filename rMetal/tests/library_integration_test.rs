use rmetal::{
    Algorithm,
    Observable,
    BinaryTournamentSelection,
    BitFlipMutation,
    ConsoleObserver,
    GeneticAlgorithm,
    GeneticAlgorithmParameters,
    HillClimbing,
    HillClimbingParameters,
    KnapsackBuilder,
    MultiObjectiveTournamentSelection,
    NSGAII,
    NSGAIIParameters,
    PolynomialMutation,
    SBXCrossover,
    SinglePointCrossover,
    SolutionSet,
    TerminationCriteria,
    TerminationCriterion,
    ZDT1Problem,
};

#[test]
fn ga_solves_knapsack_end_to_end_with_observer() {
    // Full single-objective pipeline: problem + operators + algorithm + observer.
    let problem = KnapsackBuilder::new()
        .with_capacity(15.0)
        .add_items(vec![(4.0, 8.0), (7.0, 13.0), (5.0, 10.0), (3.0, 4.0)])
        .build();

    let parameters = GeneticAlgorithmParameters::new(
        20,
        0.9,
        0.05,
        SinglePointCrossover::new(),
        BitFlipMutation::new(),
        BinaryTournamentSelection::new(),
        TerminationCriteria::new(vec![TerminationCriterion::MaxIterations(5)]),
    )
    .with_elite_size(2)
    .with_seed(123)
    .sequential();

    let mut algorithm = GeneticAlgorithm::new(parameters);
    algorithm.add_observer(Box::new(ConsoleObserver::new(false)));

    let result = algorithm.run(&problem).expect("GA run should succeed");

    assert_eq!(result.size(), 20);
    assert!(algorithm.get_solution_set().is_some());

    let best = result.best_solution().expect("Population should not be empty");
    assert!(best.quality().is_some());
    assert!(result.best_solution_value_or(f64::NEG_INFINITY).is_finite());
}

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
    let mut algorithm = HillClimbing::new(parameters, true);

    let result = algorithm.run(&problem).expect("Hill Climbing run should succeed");

    assert_eq!(result.size(), 1);
    let solution = result.get(0).expect("Hill Climbing should return one solution");
    assert_eq!(solution.num_variables(), 0);
    assert_eq!(solution.quality().copied(), Some(0.0));
}

#[test]
fn nsga2_runs_on_minimum_valid_zdt1_dimension() {
    // Edge case: ZDT1 minimum supported dimensionality (2 variables).
    let problem = ZDT1Problem::new(2);

    let parameters = NSGAIIParameters::new(
        16,
        0.9,
        0.2,
        SBXCrossover::new_default(),
        PolynomialMutation::new_default(),
        MultiObjectiveTournamentSelection::new(),
        TerminationCriteria::new(vec![TerminationCriterion::MaxIterations(4)]),
    )
    .with_seed(99);

    let mut algorithm = NSGAII::new(parameters);
    let result = algorithm.run(&problem).expect("NSGA-II run should succeed");

    assert_eq!(result.size(), 16);

    for solution in result.solutions() {
        let objectives = solution
            .get_objectives()
            .expect("All NSGA-II solutions must have objectives");

        assert_eq!(objectives.len(), 2);
        assert!(objectives[0].is_finite() && objectives[1].is_finite());
        assert!((0.0..=1.0).contains(&objectives[0]));
        assert!(objectives[1] >= 0.0);
    }
}

#[test]
fn ga_returns_error_with_invalid_parameters_edge_case() {
    // Edge case: invalid parameter configuration should return a validation error.
    let problem = KnapsackBuilder::new()
        .with_capacity(10.0)
        .add_item(5.0, 10.0)
        .build();

    let parameters = GeneticAlgorithmParameters::new(
        0,
        0.8,
        0.1,
        SinglePointCrossover::new(),
        BitFlipMutation::new(),
        BinaryTournamentSelection::new(),
        TerminationCriteria::new(vec![TerminationCriterion::MaxIterations(5)]),
    )
    .with_seed(1);

    let mut algorithm = GeneticAlgorithm::new(parameters);
    let error = match algorithm.run(&problem) {
        Ok(_) => panic!("GA must return an error for invalid parameters"),
        Err(message) => message,
    };
    assert!(error.contains("population_size"));
}
