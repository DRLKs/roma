use roma_lib::{
    Algorithm,
    BinaryTournamentSelection,
    BitFlipMutation,
    ConsoleObserver,
    GeneticAlgorithm,
    GeneticAlgorithmParameters,
    KnapsackBuilder,
    Observable,
    SinglePointCrossover,
    SolutionSet,
    TerminationCriteria,
    TerminationCriterion,
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

    let best = result
        .best_solution(&problem)
        .expect("Population should not be empty");
    assert!(best.quality().is_some());
    assert!(result.best_solution_value_or(&problem, f64::NEG_INFINITY).is_finite());
}

#[test]
fn ga_rejects_zero_population_size() {
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

#[test]
fn ga_rejects_out_of_range_crossover_probability() {
    let problem = KnapsackBuilder::new()
        .with_capacity(10.0)
        .add_item(5.0, 10.0)
        .build();

    let parameters = GeneticAlgorithmParameters::new(
        8,
        1.2,
        0.1,
        SinglePointCrossover::new(),
        BitFlipMutation::new(),
        BinaryTournamentSelection::new(),
        TerminationCriteria::new(vec![TerminationCriterion::MaxIterations(5)]),
    )
    .with_seed(2);

    let mut algorithm = GeneticAlgorithm::new(parameters);
    let error = match algorithm.run(&problem) {
        Ok(_) => panic!("GA must reject crossover_probability outside [0,1]"),
        Err(message) => message,
    };

    assert!(error.contains("crossover_probability"));
}

#[test]
fn ga_rejects_negative_mutation_probability() {
    let problem = KnapsackBuilder::new()
        .with_capacity(10.0)
        .add_item(5.0, 10.0)
        .build();

    let parameters = GeneticAlgorithmParameters::new(
        8,
        0.8,
        -0.1,
        SinglePointCrossover::new(),
        BitFlipMutation::new(),
        BinaryTournamentSelection::new(),
        TerminationCriteria::new(vec![TerminationCriterion::MaxIterations(5)]),
    )
    .with_seed(3);

    let mut algorithm = GeneticAlgorithm::new(parameters);
    let error = match algorithm.run(&problem) {
        Ok(_) => panic!("GA must reject mutation_probability outside [0,1]"),
        Err(message) => message,
    };

    assert!(error.contains("mutation_probability"));
}
