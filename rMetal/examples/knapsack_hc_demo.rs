use rmetal::algorithms::{
    Algorithm, HillClimbing, HillClimbingParameters, TerminationCriteria, TerminationCriterion,
};
use rmetal::observer::{ChartObserver, ConsoleObserver, Observable};
use rmetal::operator::BitFlipMutation;
use rmetal::problem::KnapsackBuilder;
use rmetal::solution_set::SolutionSet;
use rmetal::utils::cli::{has_flag, seed_from_cli_or};
use rmetal::HtmlReportObserver;

fn main() {
    let seed = seed_from_cli_or(42);
    let resume = has_flag("--resume");

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
    .with_seed(seed)
    .with_resume(resume);
    let mut algorithm = HillClimbing::new(parameters);

    algorithm.add_observer(Box::new(ConsoleObserver::new(true)));
    algorithm.add_observer(Box::new(ChartObserver::new_default()));
    algorithm.add_observer(Box::new(HtmlReportObserver::new_default()));

    let result = algorithm
        .run(&problem)
        .unwrap_or_else(|error| panic!("Hill Climbing run failed (resume={}): {}", resume, error));

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
