use rMetal::algorithms::implementations::genetic_algorithm::{GeneticAlgorithm, GeneticAlgorithmParameters};
use rMetal::algorithms::traits::Algorithm;
use rMetal::operator::crossover_operator_implementations::single_point_crossover::SinglePointCrossover;
use rMetal::operator::mutation_operator_implementations::bit_flip_mutation::BitFlipMutation;
use rMetal::operator::selection_operator_implementations::binary_tournament_selection::BinaryTournamentSelection;
use rMetal::problem::implementations::knapsack_problem::KnapsackProblem;
use rMetal::solution_set::traits::SolutionSet;
use rMetal::solutions::traits::Solution;

fn main() {
    println!("=== Internal Parallel Evaluation Demo ===\n");

    // Create a larger Knapsack Problem to show parallelization benefits
    let weights = vec![
        10.0, 20.0, 30.0, 40.0, 50.0, 60.0, 70.0, 80.0,
        15.0, 25.0, 35.0, 45.0, 55.0, 65.0, 75.0, 85.0,
    ];
    let values = vec![
        5.0, 10.0, 15.0, 20.0, 25.0, 30.0, 35.0, 40.0,
        7.0, 12.0, 17.0, 22.0, 27.0, 32.0, 37.0, 42.0,
    ];
    let capacity = 300.0;
    let problem = KnapsackProblem::with_data(capacity, weights, values);

    println!("Problem: Knapsack with {} items and capacity {}", 16, capacity);
    println!();

    // ========================================
    // Test 1: Sequential Execution
    // ========================================
    println!("--- Test 1: Sequential Execution ---");
    let params_sequential = GeneticAlgorithmParameters::new(
        100,  // population_size
        50,   // max_generations
        0.9,  // crossover_probability
        0.1,  // mutation_probability
        SinglePointCrossover::new(),
        BitFlipMutation::new(),
        BinaryTournamentSelection::new(),
    ).sequential(); // Explicitly sequential

    let mut ga_sequential = GeneticAlgorithm::new(params_sequential);
    
    let start = std::time::Instant::now();
    let result_sequential = ga_sequential.run(&problem, 0);
    let duration_sequential = start.elapsed();

    let best_sequential = result_sequential.solutions().first().unwrap();
    println!("Sequential:");
    println!("  Best fitness: {}", best_sequential.value());
    println!("  Time: {:?}", duration_sequential);
    println!();

    // ========================================
    // Test 2: Parallel Execution with 2 threads
    // ========================================
    println!("--- Test 2: Parallel Execution (2 threads) ---");
    let params_parallel_2 = GeneticAlgorithmParameters::new(
        100,
        50,
        0.9,
        0.1,
        SinglePointCrossover::new(),
        BitFlipMutation::new(),
        BinaryTournamentSelection::new(),
    ).with_threads(2);

    let mut ga_parallel_2 = GeneticAlgorithm::new(params_parallel_2);
    
    let start = std::time::Instant::now();
    let result_parallel_2 = ga_parallel_2.run(&problem, 0);
    let duration_parallel_2 = start.elapsed();

    let best_parallel_2 = result_parallel_2.solutions().first().unwrap();
    println!("Parallel (2 threads):");
    println!("  Best fitness: {}", best_parallel_2.value());
    println!("  Time: {:?}", duration_parallel_2);
    println!("  Speedup: {:.2}x", duration_sequential.as_secs_f64() / duration_parallel_2.as_secs_f64());
    println!();

    // ========================================
    // Test 3: Parallel Execution with 4 threads
    // ========================================
    println!("--- Test 3: Parallel Execution (4 threads) ---");
    let params_parallel_4 = GeneticAlgorithmParameters::new(
        100,
        50,
        0.9,
        0.1,
        SinglePointCrossover::new(),
        BitFlipMutation::new(),
        BinaryTournamentSelection::new(),
    ).with_threads(4);

    let mut ga_parallel_4 = GeneticAlgorithm::new(params_parallel_4);
    
    let start = std::time::Instant::now();
    let result_parallel_4 = ga_parallel_4.run(&problem, 0);
    let duration_parallel_4 = start.elapsed();

    let best_parallel_4 = result_parallel_4.solutions().first().unwrap();
    println!("Parallel (4 threads):");
    println!("  Best fitness: {}", best_parallel_4.value());
    println!("  Time: {:?}", duration_parallel_4);
    println!("  Speedup: {:.2}x", duration_sequential.as_secs_f64() / duration_parallel_4.as_secs_f64());
    println!();

    // ========================================
    // Test 4: Auto-detect threads
    // ========================================
    println!("--- Test 4: Auto-detect threads ---");
    let params_auto = GeneticAlgorithmParameters::new(
        100,
        50,
        0.9,
        0.1,
        SinglePointCrossover::new(),
        BitFlipMutation::new(),
        BinaryTournamentSelection::new(),
    ).with_parallel(); // Auto-detect

    let mut ga_auto = GeneticAlgorithm::new(params_auto);
    
    let start = std::time::Instant::now();
    let result_auto = ga_auto.run(&problem, 0);
    let duration_auto = start.elapsed();

    let best_auto = result_auto.solutions().first().unwrap();
    let detected_threads = std::thread::available_parallelism()
        .map(|n| n.get())
        .unwrap_or(1);
    
    println!("Auto-detect ({} threads detected):", detected_threads);
    println!("  Best fitness: {}", best_auto.value());
    println!("  Time: {:?}", duration_auto);
    println!("  Speedup: {:.2}x", duration_sequential.as_secs_f64() / duration_auto.as_secs_f64());
    println!();

    // ========================================
    // Summary
    // ========================================
    println!("--- Summary ---");
    println!("La paralelización interna permite que cada generación del algoritmo");
    println!("evalúe múltiples individuos simultáneamente en diferentes hilos.");
    println!("Esto acelera la ejecución sin cambiar la lógica del algoritmo.");
    println!();
    println!("Todos los individuos evaluados en cada generación se agregan al");
    println!("SolutionSet interno, permitiendo análisis posterior de toda la población.");

    println!("\n=== Demo Complete ===");
}
