use rMetal::algorithms::algorithm_trait::Algorithm;
use rMetal::problem::implementations::knapsack_problem::KnapsackProblem;
use rMetal::problem::problem_trait::Problem;
use rMetal::solution_set::solution_set_trait::SolutionSet;
use rMetal::solutions::implementations::binary_solution::BinarySolution;
use rMetal::solutions::solution_trait::Solution;

/// Función genérica que puede ejecutar cualquier algoritmo sobre cualquier problema
/// 
/// Esta función demuestra cómo la arquitectura permite pasar algoritmos por parámetro
pub fn solve_with_algorithm<A>(
    problem: &KnapsackProblem, 
    mut algorithm: A,
    algorithm_name: &str,
) where
    A: Algorithm<bool, BinarySolution, KnapsackProblem>,
{
    println!("\n=== Resolviendo con {} ===", algorithm_name);
    
    let solution_set = algorithm.run(problem, 1);
    
    if let Some(best) = solution_set.get(0) {
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
    
    // Puedes usar diferentes algoritmos sobre el mismo problema
    // Solo necesitas importar e instanciar el algoritmo que quieras usar
    
    // Ejemplo con Hill Climbing
    use rMetal::algorithms::implementations::hill_climbing::{HillClimbing, HillClimbingParameters};
    
    let params = HillClimbingParameters {
        max_iterations: 500,
    };
    let hill_climbing = HillClimbing::new(params, true);
    
    solve_with_algorithm(&problem, hill_climbing, "Hill Climbing");
    
    // Aquí podrías agregar más algoritmos, por ejemplo:
    // let genetic_algorithm = GeneticAlgorithm::new(ga_params);
    // solve_with_algorithm(&problem, genetic_algorithm, "Genetic Algorithm");
    
    // let simulated_annealing = SimulatedAnnealing::new(sa_params);
    // solve_with_algorithm(&problem, simulated_annealing, "Simulated Annealing");
    
    println!("\n=== Ventajas de esta arquitectura ===");
    println!("1. Los algoritmos no conocen el problema específico en compile-time");
    println!("2. Puedes pasar cualquier algoritmo como parámetro a una función");
    println!("3. Puedes cambiar de algoritmo sin modificar el código del problema");
    println!("4. Cada algoritmo mantiene su propio estado y parámetros");
}
