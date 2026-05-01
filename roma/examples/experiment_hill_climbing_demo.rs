use roma_lib::TspProblem;
use roma_lib::algorithms::{
    HillClimbingParameters,
    TerminationCriteria,
    TerminationCriterion,
};
use roma_lib::experiment::Experiment;
use roma_lib::operator::SwapMutation;

fn main() {
    let problem = TspProblem::with_distance_matrix(vec![
        vec![0.0, 10.0, 25.0, 18.0, 12.0],
        vec![10.0, 0.0, 14.0, 21.0, 17.0],
        vec![25.0, 14.0, 0.0, 9.0, 16.0],
        vec![18.0, 21.0, 9.0, 0.0, 11.0],
        vec![12.0, 17.0, 16.0, 11.0, 0.0],
    ])
    .with_open_route();

    let case_a = HillClimbingParameters::new(
            SwapMutation::new(),
            0.08,
            TerminationCriteria::new(vec![TerminationCriterion::MaxIterations(120)]),
        );

    let case_b = HillClimbingParameters::new(
            SwapMutation::new(),
            0.20,
            TerminationCriteria::new(vec![TerminationCriterion::MaxIterations(120)]),
        );

    let report = Experiment::new(problem)
        .with_runs(12)
        .add_case(case_a)
        .add_case(case_b)
        .execute();

    match report {
        Ok(report) => println!("{}", report.to_text_table()),
        Err(error) => eprintln!("Experiment execution failed: {}", error),
    }
}
