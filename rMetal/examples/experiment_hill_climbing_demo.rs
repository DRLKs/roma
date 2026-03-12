use rmetal::algorithms::{
    HillClimbingExperiment,
    HillClimbingParameters,
    TerminationCriteria,
    TerminationCriterion,
};
use rmetal::experiment::{Experiment, Objective};
use rmetal::operator::BitFlipMutation;
use rmetal::problem::KnapsackBuilder;

fn main() {
    let problem = KnapsackBuilder::new()
        .with_capacity(90.0)
        .add_item(12.0, 24.0)
        .add_item(22.0, 33.0)
        .add_item(41.0, 80.0)
        .add_item(18.0, 29.0)
        .add_item(8.0, 12.0)
        .build();

    let case_a = HillClimbingExperiment::new(
        HillClimbingParameters::new(
            BitFlipMutation::new(),
            0.08,
            TerminationCriteria::new(vec![TerminationCriterion::MaxIterations(120)]),
        ),
        true,
    );

    let case_b = HillClimbingExperiment::new(
        HillClimbingParameters::new(
            BitFlipMutation::new(),
            0.20,
            TerminationCriteria::new(vec![TerminationCriterion::MaxIterations(120)]),
        ),
        true,
    );

    let report = Experiment::new(problem)
        .with_runs(12)
        .with_objective(Objective::Maximize)
        .add_case(case_a)
        .add_case(case_b)
        .execute();

    match report {
        Ok(report) => println!("{}", report.to_text_table()),
        Err(error) => eprintln!("Experiment execution failed: {}", error),
    }
}
