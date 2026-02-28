use std::path::Path;

use rmetal::algorithms::{Algorithm, GeneticAlgorithm, GeneticAlgorithmParameters};
use rmetal::operator::{BinaryTournamentSelection, BitFlipMutation, SinglePointCrossover};
use rmetal::problem::KnapsackBuilder;
use rmetal::solution_set::SolutionSet;
use rmetal::utils::cli::seed_from_cli_or;
use rmetal::utils::csv_adapter::read_csv;


fn main() {
    let seed = seed_from_cli_or(42);

    let csv_path = Path::new("examples/knapsack_large_dataset/items.csv");
    let rows = read_csv(csv_path, ',', true).expect("failed to read dataset CSV");

    let mut builder = KnapsackBuilder::new().with_capacity(400.0);
    for row in rows.iter().take(120) {
        if row.len() >= 3 {
            let weight = row[1].parse::<f64>().unwrap_or(0.0);
            let value = row[2].parse::<f64>().unwrap_or(0.0);
            builder = builder.add_item(weight, value);
        }
    }
    let problem = builder.build();

    let parameters = GeneticAlgorithmParameters::new(
        80,
        60,
        0.85,
        0.04,
        SinglePointCrossover::new(),
        BitFlipMutation::new(),
        BinaryTournamentSelection::new(),
    )
    .with_seed(seed);

    let mut algorithm = GeneticAlgorithm::new(parameters);
    let result = algorithm.run(&problem);

    if let Some(best) = result.best_solution() {
        println!(
            "Large CSV GA demo finished (seed={}). items={}, best fitness={:.4}",
            seed,
            result.get(0).map(|s| s.num_variables()).unwrap_or(0),
            best.quality_value()
        );
    } else {
        println!("Large CSV GA demo finished with no solutions (seed={})", seed);
    }
}
