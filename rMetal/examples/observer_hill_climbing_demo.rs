use rMetal::algorithms::implementations::hill_climbing::{HillClimbing, HillClimbingParameters};
use rMetal::operator::mutation_operator_implementations::bit_flip_mutation::BitFlipMutation;
use rMetal::observer::implementations::chart_observer::ChartObserver;
use rMetal::observer::implementations::console_observer::ConsoleObserver;
use rMetal::problem::implementations::knapsack_problem::KnapsackProblem;
use rMetal::algorithms::traits::Algorithm;
use rMetal::solution_set::traits::SolutionSet;
use rMetal::solutions::traits::Solution;
use std::path::PathBuf;

fn main() {
    println!("=== Hill Climbing with Observers Demo ===\n");

    // Define a knapsack problem
    let weights = vec![10.0, 20.0, 30.0, 40.0, 50.0, 60.0, 70.0, 80.0, 90.0, 100.0];
    let values = vec![1.0, 2.0, 3.0, 4.0, 5.0, 6.0, 7.0, 8.0, 9.0, 10.0];
    let capacity = 150.0;
    
    let problem = KnapsackProblem::with_data(capacity, weights.clone(), values.clone());

    // Create mutation operator
    let mutation = BitFlipMutation::new();

    // Create algorithm parameters
    let params = HillClimbingParameters::new(
        1000,   // max_iterations
        mutation,
        0.1,    // mutation_probability
    );

    // Create algorithm (maximization)
    let mut hc = HillClimbing::new(params, true);

    // Add observers
    println!("Adding observers...\n");
    
    // Console observer for real-time updates
    let console_observer = ConsoleObserver::new(true); // verbose=true
    hc.add_observer(Box::new(console_observer));
    
    // Chart observer for visualization
    let output_dir = PathBuf::from("output/hill_climbing_charts");
    let chart_observer = ChartObserver::new(output_dir)
        .with_dimensions(1200, 800);
    hc.add_observer(Box::new(chart_observer));

    // Run the algorithm
    println!("Running Hill Climbing...\n");
    let result = hc.run(&problem, 0); // verbose=0 to let observers handle output

    // Display results
    println!("\n=== Final Results ===");
    println!("Number of solutions: {}", result.size());
    
    if let Some(best_solution) = result.get(0) {
        println!("Best solution found:");
        println!("  Variables: {:?}", best_solution.get_solution_info().get_variables());
        println!("  Fitness: {}", best_solution.value());
    }

    println!("\n✅ Charts have been generated in the 'output/hill_climbing_charts' directory");
    println!("   - convergence.png: Shows fitness evolution over iterations");
    println!("   - evaluations.png: Shows total evaluations over time");
}
