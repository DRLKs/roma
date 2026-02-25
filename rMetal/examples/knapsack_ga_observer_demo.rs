use std::path::PathBuf;

use rMetal::algorithms::implementations::genetic_algorithm::{
    GeneticAlgorithm, GeneticAlgorithmParameters,
};
use rMetal::algorithms::traits::Algorithm;
use rMetal::observer::implementations::chart_observer::ChartObserver;
use rMetal::observer::implementations::console_observer::ConsoleObserver;
use rMetal::observer::traits::Observable;
use rMetal::operator::crossover_operator_implementations::single_point_crossover::SinglePointCrossover;
use rMetal::operator::mutation_operator_implementations::bit_flip_mutation::BitFlipMutation;
use rMetal::operator::selection_operator_implementations::binary_tournament_selection::BinaryTournamentSelection;
use rMetal::problem::implementations::knapsack_problem::KnapsackBuilder;
use rMetal::solution_set::traits::SolutionSet;

fn main() {
    let problem = KnapsackBuilder::new()
        .with_capacity(150.0)
        .add_item(10.0, 20.0)
        .add_item(20.0, 30.0)
        .add_item(30.0, 60.0)
        .build();

    let parameters = GeneticAlgorithmParameters::new(
        50,
        40,
        0.85,
        0.08,
        SinglePointCrossover::new(),
        BitFlipMutation::new(),
        BinaryTournamentSelection::new(),
    );

    let mut algorithm = GeneticAlgorithm::new(parameters);
    algorithm.add_observer(Box::new(ConsoleObserver::new(true)));
    algorithm.add_observer(Box::new(ChartObserver::new(PathBuf::from(
        "output/knapsack_ga_observer_demo/charts",
    ))));

    let result = algorithm.run(&problem);

    if let Some(best) = result.best_solution() {
        println!("GA finished. Best fitness={:.4}", best.value());
    } else {
        println!("GA finished with no solutions");
    }
}
