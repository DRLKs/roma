use roma::algorithms::{
    Algorithm,
    GeneticAlgorithm,
    GeneticAlgorithmParameters,
    TerminationCriteria,
    TerminationCriterion,
};
use roma::observer::{ChartObserver, ConsoleObserver, Observable};
use roma::operator::{BinaryTournamentSelection, BitFlipMutation, SinglePointCrossover};
use roma::problem::KnapsackBuilder;
use roma::solution_set::SolutionSet;
use roma::utils::cli::seed_from_cli_or;

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
        0.85,
        0.08,
        SinglePointCrossover::new(),
        BitFlipMutation::new(),
        BinaryTournamentSelection::new(),
        TerminationCriteria::new(vec![TerminationCriterion::MaxIterations(40)]),
    )
    .with_elite_size(1)
    .with_seed(seed);

    let mut algorithm = GeneticAlgorithm::new(parameters);
    algorithm.add_observer(Box::new(ConsoleObserver::new(true)));
    algorithm.add_observer(Box::new(ChartObserver::new_default()));

    let result = algorithm
        .run(&problem)
        .expect("GA run failed");

    if let Some(best) = result.best_solution() {
        println!(
            "GA finished (seed={}). Best fitness={:.4}",
            seed,
            best.quality_value()
        );
    } else {
        println!("GA finished with no solutions (seed={})", seed);
    }
}
