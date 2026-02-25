use rMetal::algorithms::implementations::genetic_algorithm::{
    GeneticAlgorithm, GeneticAlgorithmParameters,
};
use rMetal::algorithms::traits::Algorithm;
use rMetal::operator::crossover_operator_implementations::single_point_crossover::SinglePointCrossover;
use rMetal::operator::mutation_operator_implementations::bit_flip_mutation::BitFlipMutation;
use rMetal::operator::selection_operator_implementations::binary_tournament_selection::BinaryTournamentSelection;
use rMetal::problem::implementations::knapsack_problem::KnapsackBuilder;
use rMetal::solution_set::traits::SolutionSet;

fn main() {
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
    .with_threads(4);

    let mut algorithm = GeneticAlgorithm::new(parameters);
    let result = algorithm.run(&problem);

    if let Some(best) = result.best_solution() {
        println!("Parallel GA demo finished. Best fitness={:.4}", best.value());
    } else {
        println!("Parallel GA demo finished with no solutions");
    }
}
