use roma::algorithms::{
    Algorithm,
    NSGAII,
    NSGAIIParameters,
    TerminationCriteria,
    TerminationCriterion,
};
use roma::{ChartObserver, ConsoleObserver, Observable};
use roma::operator::{MultiObjectiveTournamentSelection, PolynomialMutation, SBXCrossover};
use roma::problem::ZDT1Problem;
use roma::solution_set::SolutionSet;
use roma::utils::cli::seed_from_cli_or;

fn main() {
    let seed = seed_from_cli_or(42);

    let problem = ZDT1Problem::new(30);

    let parameters = NSGAIIParameters::new(
        60,
        0.9,
        1.0 / 30.0,
        SBXCrossover::new_default(),
        PolynomialMutation::new_default(),
        MultiObjectiveTournamentSelection::new(),
        TerminationCriteria::new(vec![TerminationCriterion::MaxIterations(40)]),
    )
    .with_seed(seed);

    let mut algorithm = NSGAII::new(parameters);
    let observer = ConsoleObserver::new(true);
    let observer2 = ChartObserver::new_default();
    algorithm.add_observer(Box::new(observer));
    algorithm.add_observer(Box::new(observer2));
    let result = algorithm
        .run(&problem)
        .expect("NSGA-II run failed");

    if let Some(best) = result.get(0) {
        println!(
            "NSGA-II finished (seed={}). population={}, best objectives={:?}",
            seed,
            result.size(),
            best.objectives()
        );
    } else {
        println!("NSGA-II finished with empty population (seed={})", seed);
    }
}
