use rMetal::algorithms::algorithm_trait::Algorithm;
use rMetal::algorithms::implementations::hill_climbing::{HillClimbing, HillClimbingParameters};
use rMetal::operator::implementations::bit_flip_mutation::BitFlipMutation;
use rMetal::problem::implementations::knapsack_problem::KnapsackProblem;
use rMetal::problem::problem_trait::Problem;
use rMetal::solution_set::solution_set_trait::SolutionSet;
use rMetal::solutions::solution_trait::Solution;

fn main() {
    // Define el problema de la mochila
    let weights = vec![2.0, 3.0, 4.0, 5.0, 9.0];
    let values = vec![3.0, 4.0, 8.0, 8.0, 10.0];
    let capacity = 20.0;
    
    let problem = KnapsackProblem::with_data(capacity, weights, values);
    
    println!("=== Problema de la Mochila ===");
    println!("{}", problem.get_problem_description());
    println!();
    
    // Configura el operador de mutación
    let mutation = BitFlipMutation::new();
    
    // Configura el algoritmo Hill Climbing con el operador
    let parameters = HillClimbingParameters::new(
        1000,       // max_iterations
        mutation,   // operador de mutación intercambiable
        0.1,        // mutation_probability (probabilidad de mutar cada bit)
    );
    
    let mut algorithm = HillClimbing::new(parameters, true); // true = maximization
    
    println!("=== Ejecutando Hill Climbing ===");
    let solution_set = algorithm.run(&problem, 1); // verbose = 1
    
    println!();
    println!("=== Resultados ===");
    println!("Número de soluciones: {}", solution_set.size());
    
    if let Some(best_solution) = solution_set.get(0) {
        println!("Mejor solución encontrada:");
        println!("  Valor (fitness): {}", best_solution.value());
        println!("  Variables: {:?}", (0..best_solution.get_number_of_variables())
            .map(|i| best_solution.get_variable(i).unwrap())
            .collect::<Vec<_>>());
    }
}
