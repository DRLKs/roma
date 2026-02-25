use std::path::Path;

use rMetal::algorithms::implementations::genetic_algorithm::{
    GeneticAlgorithm, GeneticAlgorithmParameters,
};
use rMetal::algorithms::traits::Algorithm;
use rMetal::operator::crossover_operator_implementations::single_point_crossover::SinglePointCrossover;
use rMetal::operator::mutation_operator_implementations::bit_flip_mutation::BitFlipMutation;
use rMetal::operator::selection_operator_implementations::binary_tournament_selection::BinaryTournamentSelection;
use rMetal::problem::implementations::knapsack_problem::KnapsackBuilder;
use rMetal::solution_set::traits::SolutionSet;
use rMetal::utils::csv_adapter::read_csv;


fn main() {
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
    );

    let mut algorithm = GeneticAlgorithm::new(parameters);
    let result = algorithm.run(&problem);

    if let Some(best) = result.best_solution() {
        println!(
            "Large CSV GA demo finished. items={}, best fitness={:.4}",
            result.get(0).map(|s| s.num_variables()).unwrap_or(0),
            best.value()
        );
    } else {
        println!("Large CSV GA demo finished with no solutions");
    }
}
