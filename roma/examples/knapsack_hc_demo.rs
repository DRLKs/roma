use roma_lib::HtmlReportObserver;
use roma_lib::algorithms::{
    Algorithm,
    HillClimbing,
    HillClimbingParameters,
    TerminationCriteria,
    TerminationCriterion,
};
use roma_lib::observer::{ChartObserver, ConsoleObserver, Observable};
use roma_lib::operator::BitFlipMutation;
use roma_lib::problem::KnapsackBuilder;
use roma_lib::solution_set::SolutionSet;
use roma_lib::utils::cli::CliArgs;

fn main() {
    let seed = CliArgs::from_env().seed_or(42);

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
    let mut algorithm = HillClimbing::new(parameters);

    algorithm.add_observer(Box::new(ConsoleObserver::new(true)));
    algorithm.add_observer(Box::new(ChartObserver::new_default()));
    algorithm.add_observer(Box::new(HtmlReportObserver::new_default()));

    let result = algorithm
        .run(&problem)
        .expect("Hill Climbing run failed");

    if let Some(best) = result.best_solution(&problem) {
        println!(
            "Hill-Climbing finished (seed={}). Best fitness={:.4}",
            seed,
            best.quality_value()
        );
    } else {
        println!("Hill-Climbing finished with no solutions (seed={})", seed);
    }
}
