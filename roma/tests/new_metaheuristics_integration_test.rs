use roma_lib::{
    AckleyProblem,
    Algorithm,
    DifferentialEvolution,
    DifferentialEvolutionParameters,
    NeighborhoodOperator,
    PermutationSwapNeighborhood,
    Problem,
    QapProblem,
    RealPerturbationNeighborhood,
    SolutionSet,
    TabuSearch,
    TabuSearchParameters,
    TerminationCriteria,
    TerminationCriterion,
    VNS,
    VNSParameters,
};
use roma_lib::prelude::Random;

#[test]
fn differential_evolution_solves_ackley_via_crate_root_exports() {
    let problem = AckleyProblem::new(10, -5.0, 5.0);
    let parameters = DifferentialEvolutionParameters::new(
        18,
        0.9,
        0.8,
        TerminationCriteria::new(vec![TerminationCriterion::MaxIterations(20)]),
    )
    .with_seed(101);

    let mut algorithm = DifferentialEvolution::new(parameters);
    let result = algorithm.run(&problem).expect("DE on Ackley should succeed");

    assert_eq!(result.size(), 18);
    let best = result
        .best_solution(&problem)
        .expect("Expected a best Ackley solution");
    assert!(best.quality_value().is_finite());
    assert!(best.variables().iter().all(|value| (-5.0..=5.0).contains(value)));
}

#[test]
fn tabu_search_solves_qap_via_crate_root_exports() {
    let problem = QapProblem::with_matrices(
        vec![
            vec![0.0, 4.0, 1.0, 2.0],
            vec![4.0, 0.0, 3.0, 5.0],
            vec![1.0, 3.0, 0.0, 2.0],
            vec![2.0, 5.0, 2.0, 0.0],
        ],
        vec![
            vec![0.0, 9.0, 6.0, 4.0],
            vec![9.0, 0.0, 7.0, 5.0],
            vec![6.0, 7.0, 0.0, 3.0],
            vec![4.0, 5.0, 3.0, 0.0],
        ],
    );

    let parameters = TabuSearchParameters::new(
        PermutationSwapNeighborhood::new(1),
        12,
        5,
        TerminationCriteria::new(vec![TerminationCriterion::MaxIterations(20)]),
    )
    .with_seed(55);

    let mut algorithm = TabuSearch::new(parameters);
    let result = algorithm.run(&problem).expect("Tabu Search on QAP should succeed");

    assert_eq!(result.size(), 1);
    let best = result.best_solution(&problem).expect("Expected one QAP solution");
    let mut assignment = best.variables().to_vec();
    assignment.sort_unstable();

    assert_eq!(assignment, vec![0, 1, 2, 3]);
    assert!(best.quality_value().is_finite());
}

#[test]
fn vns_solves_ackley_via_crate_root_exports() {
    let problem = AckleyProblem::new(8, -4.0, 4.0);
    let parameters = VNSParameters::new(
        vec![
            RealPerturbationNeighborhood::new(0.05, 0.5),
            RealPerturbationNeighborhood::new(0.15, 0.75),
        ],
        5,
        TerminationCriteria::new(vec![TerminationCriterion::MaxIterations(18)]),
    )
    .with_seed(77);

    let mut algorithm = VNS::new(parameters);
    let result = algorithm.run(&problem).expect("VNS on Ackley should succeed");

    assert_eq!(result.size(), 1);
    let best = result.best_solution(&problem).expect("Expected one Ackley solution");
    assert!(best.quality_value().is_finite());
    assert!(best.variables().iter().all(|value| (-4.0..=4.0).contains(value)));
}

#[test]
fn neighborhood_trait_is_available_from_crate_root() {
    let operator = RealPerturbationNeighborhood::new(0.1, 1.0);
    let problem = AckleyProblem::new(3, -1.0, 1.0);
    let base = problem.create_solution(&mut Random::new(3));
    let mut rng = Random::new(9);

    let neighbor = operator.generate_neighbor(&base, &mut rng);

    assert_eq!(neighbor.num_variables(), 3);
}