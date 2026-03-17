use rmetal::algorithms::{
    GeneticAlgorithmExperiment,
    GeneticAlgorithmParameters,
    HillClimbingParameters,
    PSOParameters,
    SimulatedAnnealingParameters,
    TerminationCriteria,
    TerminationCriterion,
};
use rmetal::experiment::{Experiment, Objective};
use rmetal::operator::{BinaryTournamentSelection, BitFlipMutation, SinglePointCrossover};
use rmetal::problem::KnapsackBuilder;

fn main() {
    let problem = KnapsackBuilder::new()
        .with_capacity(150.0)
        .add_item(1.0, 2.0)
        .add_item(1.0, 2.0)
        .add_item(2.0, 6.0)
        .add_item(2.0, 6.5)
        .add_item(3.0, 7.0)
        .add_item(10.0, 20.0)
        .add_item(20.0, 30.0)
        .add_item(30.0, 60.0)
        .add_item(35.0, 65.0)
        .add_item(45.0, 100.0)
        .add_item(55.0, 120.0)
        .add_item(75.0, 211.0)
        .add_item(75.0, 211.0)
        .add_item(80.0, 160.0)
        .add_item(90.0, 301.0)
        .add_item(150.0, 301.0)
        .build();

    let hill_climbing_case = HillClimbingParameters::new(
            BitFlipMutation::new(),
            0.12,
            TerminationCriteria::new(vec![TerminationCriterion::MaxIterations(180)]),
        );

    let genetic_algorithm_case = GeneticAlgorithmExperiment::new(
        GeneticAlgorithmParameters::new(
            80,
            0.90,
            0.06,
            SinglePointCrossover::new(),
            BitFlipMutation::new(),
            BinaryTournamentSelection::new(),
            TerminationCriteria::new(vec![TerminationCriterion::MaxIterations(60)]),
        )
        .with_elite_size(1)
        .with_threads(4),
    );

    let simulated_annealing_case = SimulatedAnnealingParameters::new(
        BitFlipMutation::new(),
        0.10,
        45.0,
        0.985,
        TerminationCriteria::new(vec![TerminationCriterion::MaxIterations(220)]),
    )
    .maximization()
    .with_seed(777);

    let pso_case = PSOParameters::new(
        50,
        0.72,
        1.49,
        1.49,
        TerminationCriteria::new(vec![TerminationCriterion::MaxIterations(120)]),
    )
    .with_velocity_clamp(4.0)
    .maximization()
    .with_seed(999);

    let report = Experiment::new(problem)
        .with_runs(24)
        .with_objective(Objective::Maximize)
        .add_case(hill_climbing_case)
        .add_case(genetic_algorithm_case)
        .add_case(simulated_annealing_case)
        .add_case(pso_case)
        .execute();

    match report {
        Ok(report) => println!("{}", report.to_text_table()),
        Err(error) => eprintln!("Experiment execution failed: {}", error),
    }
}
