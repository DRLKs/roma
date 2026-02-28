use rmetal::algorithms::{Algorithm, GeneticAlgorithm, GeneticAlgorithmParameters};
use rmetal::operator::{BinaryTournamentSelection, BitFlipMutation, SinglePointCrossover};
use rmetal::problem::KnapsackBuilder;
use rmetal::solution_set::SolutionSet;
use rmetal::utils::cli::seed_from_cli_or;

fn main() {
    let seed = seed_from_cli_or(42);

    let problem = KnapsackBuilder::new()
        .with_capacity(50.0)
        .add_item(10.0, 60.0)
        .add_item(20.0, 100.0)
        .add_item(30.0, 120.0)
        .build();

    let parameters = GeneticAlgorithmParameters::new(
        60,
        50,
        0.85,
        0.08,
        SinglePointCrossover::new(),
        BitFlipMutation::new(),
        BinaryTournamentSelection::new(),
    )
    .with_seed(seed)
    .with_threads(4);

    let mut algorithm = GeneticAlgorithm::new(parameters);
    let result = algorithm.run(&problem);

    if let Some(best) = result.best_solution() {
        println!(
            "Parallel GA demo finished (seed={}). Best fitness={:.4}",
            seed,
            best.quality_value()
        );
    } else {
        println!("Parallel GA demo finished with no solutions (seed={})", seed);
    }
}
