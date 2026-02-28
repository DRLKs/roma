use std::path::PathBuf;

use rMetal::algorithms::{Algorithm, GeneticAlgorithm, GeneticAlgorithmParameters};
use rMetal::observer::{ChartObserver, ConsoleObserver, Observable};
use rMetal::operator::{BinaryTournamentSelection, BitFlipMutation, SinglePointCrossover};
use rMetal::problem::KnapsackBuilder;
use rMetal::solution_set::SolutionSet;
use rMetal::utils::cli::seed_from_cli_or;

fn main() {
    let seed = seed_from_cli_or(42);

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

    let parameters = GeneticAlgorithmParameters::new(
        50,
        40,
        0.85,
        0.08,
        SinglePointCrossover::new(),
        BitFlipMutation::new(),
        BinaryTournamentSelection::new(),
    )
    .with_elite_size(1)
    .with_seed(seed);

    let mut algorithm = GeneticAlgorithm::new(parameters);
    algorithm.add_observer(Box::new(ConsoleObserver::new(true)));
    algorithm.add_observer(Box::new(ChartObserver::new(PathBuf::from(
        "output/knapsack_ga_observer_demo/charts",
    ))));

    let result = algorithm.run(&problem);

    if let Some(best) = result.best_solution() {
        println!("GA finished (seed={}). Best fitness={:.4}", seed, best.value());
    } else {
        println!("GA finished with no solutions (seed={})", seed);
    }
}
