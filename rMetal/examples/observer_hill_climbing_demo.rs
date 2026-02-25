use std::path::PathBuf;

use rMetal::algorithms::implementations::hill_climbing::{HillClimbing, HillClimbingParameters};
use rMetal::algorithms::traits::Algorithm;
use rMetal::observer::implementations::chart_observer::ChartObserver;
use rMetal::observer::implementations::console_observer::ConsoleObserver;
use rMetal::observer::traits::Observable;
use rMetal::operator::mutation_operator_implementations::bit_flip_mutation::BitFlipMutation;
use rMetal::problem::implementations::knapsack_problem::KnapsackBuilder;
use rMetal::solution_set::traits::SolutionSet;

fn main() {
    let problem = KnapsackBuilder::new()
        .with_capacity(90.0)
        .add_item(12.0, 24.0)
        .add_item(22.0, 33.0)
        .add_item(41.0, 80.0)
        .build();

    let parameters = HillClimbingParameters::new(120, BitFlipMutation::new(), 0.10);
    let mut algorithm = HillClimbing::new(parameters, true);

    algorithm.add_observer(Box::new(ConsoleObserver::new(true)));
    algorithm.add_observer(Box::new(ChartObserver::new(PathBuf::from(
        "output/hill_climbing_charts",
    ))));

    let result = algorithm.run(&problem);

    if let Some(best) = result.best_solution() {
        println!("Hill-Climbing finished. Best fitness={:.4}", best.value());
    } else {
        println!("Hill-Climbing finished with no solutions");
    }
}
