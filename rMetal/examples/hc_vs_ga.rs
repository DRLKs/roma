use rmetal::algorithms::{
    GeneticAlgorithmExperiment,
    GeneticAlgorithmParameters,
    HillClimbingExperiment,
    HillClimbingParameters,
    TerminationCriteria,
    TerminationCriterion,
};
use rmetal::experiment::{Experiment, Objective};
use rmetal::operator::{BinaryTournamentSelection, BitFlipMutation, SinglePointCrossover};
use rmetal::problem::KnapsackBuilder;

fn main() {
    let problem = KnapsackBuilder::new()
        .with_capacity(150.0)
        .add_item(10.0, 20.0)
        .add_item(20.0, 30.0)
        .add_item(30.0, 60.0)
        .add_item(35.0, 65.0)
        .add_item(45.0, 70.0)
        .add_item(55.0, 90.0)
        .add_item(150.0, 300.0)
        .build();

    let hill_climbing_case = HillClimbingExperiment::new(
        HillClimbingParameters::new(
            BitFlipMutation::new(),
            0.12,
            TerminationCriteria::new(vec![TerminationCriterion::MaxIterations(180)]),
        ),
        true,
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
        .with_elite_size(2)
        .with_threads(4),
    );

    let report = Experiment::new(problem)
        .with_runs(24)
        .with_objective(Objective::Maximize)
        .add_case(hill_climbing_case)
        .add_case(genetic_algorithm_case)
        .execute();

    match report {
        Ok(report) => println!("{}", report.to_text_table()),
        Err(error) => eprintln!("Experiment execution failed: {}", error),
    }
}
