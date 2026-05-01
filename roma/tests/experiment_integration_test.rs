use roma_lib::{
    BinaryTournamentSelection, BitFlipMutation, Experiment,
    GeneticAlgorithmParameters, HillClimbingParameters, KnapsackBuilder, SinglePointCrossover,
    TerminationCriteria, TerminationCriterion,
};

#[test]
fn experiment_compares_hill_climbing_and_ga_end_to_end() {
    // End-to-end experiment: both algorithms run on the same problem and produce summaries.
    let problem = KnapsackBuilder::new()
        .with_capacity(40.0)
        .add_items(vec![
            (4.0, 8.0),
            (7.0, 13.0),
            (5.0, 10.0),
            (3.0, 4.0),
            (8.0, 15.0),
        ])
        .build();

    let hill_climbing_case = HillClimbingParameters::new(
        BitFlipMutation::new(),
        0.15,
        TerminationCriteria::new(vec![TerminationCriterion::MaxIterations(20)]),
    )
    .with_seed(111);

    let genetic_algorithm_case = GeneticAlgorithmParameters::new(
            24,
            0.9,
            0.08,
            SinglePointCrossover::new(),
            BitFlipMutation::new(),
            BinaryTournamentSelection::new(),
            TerminationCriteria::new(vec![TerminationCriterion::MaxIterations(12)]),
        )
        .with_elite_size(2)
        .with_seed(222)
        .sequential();

    let report = Experiment::new(problem)
        .with_runs(3)
        .add_case(hill_climbing_case)
        .add_case(genetic_algorithm_case)
        .execute()
        .expect("Experiment execution should succeed");

    assert_eq!(report.runs_per_case, 3);
    assert_eq!(report.failures.len(), 0);
    assert_eq!(report.summaries.len(), 2);
    assert_eq!(report.run_results.len(), 6);

    // Verify each summary contains coherent statistics.
    for summary in &report.summaries {
        assert_eq!(summary.runs_ok, 3);
        assert!(summary.best.is_finite());
        assert!(summary.mean.is_finite());
        assert!(summary.worst.is_finite());
        assert!(summary.std_dev.is_finite());
        assert!(summary.best >= summary.worst);
    }
}

#[test]
fn experiment_fails_when_no_cases_are_registered() {
    // Contract check: running an empty experiment should return a clear error.
    let problem = KnapsackBuilder::new()
        .with_capacity(10.0)
        .add_item(5.0, 10.0)
        .build();

    let error = Experiment::new(problem)
        .with_runs(2)
        .execute()
        .expect_err("Experiment must fail when no cases are provided");

    assert!(error.contains("no algorithms/configurations"));
}

#[test]
fn experiment_accepts_genetic_algorithm_parameters_directly_as_case() {
    let problem = KnapsackBuilder::new()
        .with_capacity(40.0)
        .add_items(vec![
            (4.0, 8.0),
            (7.0, 13.0),
            (5.0, 10.0),
            (3.0, 4.0),
            (8.0, 15.0),
        ])
        .build();

    let ga_case = GeneticAlgorithmParameters::new(
        24,
        0.9,
        0.08,
        SinglePointCrossover::new(),
        BitFlipMutation::new(),
        BinaryTournamentSelection::new(),
        TerminationCriteria::new(vec![TerminationCriterion::MaxIterations(12)]),
    )
    .with_elite_size(2)
    .with_seed(333)
    .sequential();

    let report = Experiment::new(problem)
        .with_runs(2)
        .add_case(ga_case)
        .execute()
        .expect("Experiment with GA parameters case should succeed");

    assert_eq!(report.runs_per_case, 2);
    assert_eq!(report.failures.len(), 0);
    assert_eq!(report.summaries.len(), 1);
    assert_eq!(report.run_results.len(), 2);
}
