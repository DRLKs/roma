use roma_lib::{
    Algorithm,
    MultiObjectiveTournamentSelection,
    NSGAII,
    NSGAIIParameters,
    PolynomialMutation,
    SBXCrossover,
    SolutionSet,
    TerminationCriteria,
    TerminationCriterion,
    ZDT1Problem,
};

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

    for solution in result.iter() {
        let objectives = solution
            .get_objectives()
            .expect("All NSGA-II solutions must have objectives");

        assert_eq!(objectives.len(), 2);
        assert!(objectives[0].is_finite() && objectives[1].is_finite());
        assert!((0.0..=1.0).contains(&objectives[0]));
        assert!(objectives[1] >= 0.0);
    }
}
