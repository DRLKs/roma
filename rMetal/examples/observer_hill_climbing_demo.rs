use rmetal::algorithms::{
    Algorithm,
    HillClimbing,
    HillClimbingParameters,
    TerminationCriteria,
    TerminationCriterion,
};
use rmetal::observer::{ChartObserver, ConsoleObserver, Observable};
use rmetal::operator::BitFlipMutation;
use rmetal::problem::KnapsackBuilder;
use rmetal::solution_set::SolutionSet;
use rmetal::utils::cli::seed_from_cli_or;

fn main() {
    let seed = seed_from_cli_or(42);

    let problem = KnapsackBuilder::new()
        .with_capacity(90.0)
        .add_item(12.0, 24.0)
        .add_item(22.0, 33.0)
        .add_item(41.0, 80.0)
        .build();

    let parameters = HillClimbingParameters::new(
        BitFlipMutation::new(),
        0.10,
        TerminationCriteria::new(vec![TerminationCriterion::MaxIterations(120)]),
    )
    .with_seed(seed);
    let mut algorithm = HillClimbing::new(parameters, true);

    algorithm.add_observer(Box::new(ConsoleObserver::new(true)));
    algorithm.add_observer(Box::new(ChartObserver::new_default()));

    let result = algorithm
        .run(&problem)
        .expect("Hill Climbing run failed");

    if let Some(best) = result.best_solution() {
        println!(
            "Hill-Climbing finished (seed={}). Best fitness={:.4}",
            seed,
            best.quality_value()
        );
    } else {
        println!("Hill-Climbing finished with no solutions (seed={})", seed);
    }
}
