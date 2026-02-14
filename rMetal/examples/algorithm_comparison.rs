use rMetal::algorithms::implementations::genetic_algorithm::{GeneticAlgorithm, GeneticAlgorithmParameters};
use rMetal::algorithms::implementations::hill_climbing::{HillClimbing, HillClimbingParameters};
use rMetal::algorithms::traits::Algorithm;
use rMetal::operator::crossover_operator_implementations::single_point_crossover::SinglePointCrossover;
use rMetal::operator::mutation_operator_implementations::bit_flip_mutation::BitFlipMutation;
use rMetal::operator::selection_operator_implementations::binary_tournament_selection::BinaryTournamentSelection;
use rMetal::problem::implementations::knapsack_problem::KnapsackProblem;
use rMetal::problem::traits::Problem;
use rMetal::solution_set::traits::SolutionSet;
use rMetal::solutions::implementations::binary_solution::BinarySolution;
use rMetal::solutions::traits::Solution;

/// Función genérica que puede ejecutar cualquier algoritmo sobre cualquier problema
pub fn solve_with_algorithm<A>(
    problem: &KnapsackProblem, 
    mut algorithm: A,
    algorithm_name: &str,
) where
    A: Algorithm<bool, BinarySolution, KnapsackProblem>,
{
    println!("\n=== Resolviendo con {} ===", algorithm_name);
    
    let solution_set = algorithm.run(problem, 1);
    
    if let Some(best) = solution_set.best_solution() {
        println!("Mejor solución:");
        println!("  Fitness: {}", best.value());
        print!("  Items seleccionados: [");
        for i in 0..best.get_number_of_variables() {
            if *best.get_variable(i).unwrap() {
                print!("{}, ", i);
            }
        }
        println!("]");
    }
}

fn main() {
    // Definir un problema de mochila
    let weights = vec![10.0, 20.0, 30.0, 40.0, 50.0];
    let values = vec![5.0, 10.0, 15.0, 20.0, 25.0];
    let capacity = 60.0;
    
    let problem = KnapsackProblem::with_data(capacity, weights, values);
    
    println!("{}", problem.get_problem_description());


    // Ejemplo con Hill Climbing
    
    let mutation: BitFlipMutation = BitFlipMutation::new();
    let params = HillClimbingParameters::new(
        500,        
        mutation,   
        0.1,        
    );
    let hill_climbing = HillClimbing::new(params, true); // true = maximization
    
    solve_with_algorithm(&problem, hill_climbing, "Hill Climbing");


    // Ejemplo con Genetic Algorithm
    let crossover = SinglePointCrossover::new();
    let mutation: BitFlipMutation = BitFlipMutation::new();
    let selection = BinaryTournamentSelection::new();
    let genetic_algorithm_parameters = GeneticAlgorithmParameters::new(50, 100, 0.8, 0.3, crossover, mutation, selection);
    let genetic_algorithm = GeneticAlgorithm::new(genetic_algorithm_parameters);
    solve_with_algorithm(&problem, genetic_algorithm, "Genetic Algorithm");
    
}
