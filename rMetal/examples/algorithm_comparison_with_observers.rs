use rMetal::algorithms::implementations::genetic_algorithm::{GeneticAlgorithm, GeneticAlgorithmParameters};
use rMetal::algorithms::implementations::hill_climbing::{HillClimbing, HillClimbingParameters};
use rMetal::operator::crossover_operator_implementations::single_point_crossover::SinglePointCrossover;
use rMetal::operator::mutation_operator_implementations::bit_flip_mutation::BitFlipMutation;
use rMetal::operator::selection_operator_implementations::binary_tournament_selection::BinaryTournamentSelection;
use rMetal::observer::implementations::chart_observer::ChartObserver;
use rMetal::observer::implementations::console_observer::ConsoleObserver;
use rMetal::problem::implementations::knapsack_problem::KnapsackProblem;
use rMetal::algorithms::traits::Algorithm;
use rMetal::solution_set::traits::SolutionSet;
use rMetal::solutions::traits::Solution;
use std::path::PathBuf;

fn main() {
    println!("=== Algorithm Comparison with Observers ===\n");

    // Define a knapsack problem
    let weights = vec![23.0, 31.0, 29.0, 44.0, 53.0, 38.0, 63.0, 85.0, 89.0, 82.0];
    let values = vec![92.0, 57.0, 49.0, 68.0, 60.0, 43.0, 67.0, 84.0, 87.0, 72.0];
    let capacity = 165.0;
    
    println!("Problem configuration:");
    println!("  Items: {}", weights.len());
    println!("  Capacity: {}", capacity);
    println!("  Total weight: {}", weights.iter().sum::<f64>());
    println!("  Total value: {}\n", values.iter().sum::<f64>());

    // ============= Genetic Algorithm =============
    println!("━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━");
    println!("          GENETIC ALGORITHM");
    println!("━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━\n");

    let problem_ga = KnapsackProblem::with_data(capacity, weights.clone(), values.clone());

    let crossover = SinglePointCrossover::new();
    let mutation = BitFlipMutation::new();
    let selection = BinaryTournamentSelection::new();

    let params_ga = GeneticAlgorithmParameters::new(
        30,     // population_size
        50,     // max_generations
        0.9,    // crossover_probability
        0.1,    // mutation_probability
        crossover,
        mutation,
        selection,
    );

    let mut ga = GeneticAlgorithm::new(params_ga);

    // Add observers
    let console_ga = ConsoleObserver::new(false);
    ga.add_observer(Box::new(console_ga));
    
    let chart_ga = ChartObserver::new(PathBuf::from("output/comparison/genetic_algorithm"))
        .with_dimensions(1200, 800);
    ga.add_observer(Box::new(chart_ga));

    let result_ga = ga.run(&problem_ga, 0);

    // ============= Hill Climbing =============
    println!("\n━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━");
    println!("           HILL CLIMBING");
    println!("━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━\n");

    let problem_hc = KnapsackProblem::with_data(capacity, weights.clone(), values.clone());

    let mutation_hc = BitFlipMutation::new();
    let params_hc = HillClimbingParameters::new(
        1500,      // max_iterations (comparable to GA evaluations)
        mutation_hc,
        0.1,
    );

    let mut hc = HillClimbing::new(params_hc, true);

    let console_hc = ConsoleObserver::new(false);
    hc.add_observer(Box::new(console_hc));
    
    let chart_hc = ChartObserver::new(PathBuf::from("output/comparison/hill_climbing"))
        .with_dimensions(1200, 800);
    hc.add_observer(Box::new(chart_hc));

    let result_hc = hc.run(&problem_hc, 0);

    // ============= Comparison =============
    println!("\n━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━");
    println!("            COMPARISON");
    println!("━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━\n");

    let best_ga = result_ga.get(0).map(|s| s.value()).unwrap_or(0.0);
    let best_hc = result_hc.get(0).map(|s| s.value()).unwrap_or(0.0);

    println!("┌────────────────────────┬──────────────┐");
    println!("│ Algorithm              │ Best Fitness │");
    println!("├────────────────────────┼──────────────┤");
    println!("│ Genetic Algorithm      │ {:>12.2} │", best_ga);
    println!("│ Hill Climbing          │ {:>12.2} │", best_hc);
    println!("└────────────────────────┴──────────────┘");

    if best_ga > best_hc {
        println!("\n🏆 Winner: Genetic Algorithm (by {:.2})", best_ga - best_hc);
    } else if best_hc > best_ga {
        println!("\n🏆 Winner: Hill Climbing (by {:.2})", best_hc - best_ga);
    } else {
        println!("\n🤝 Tie!");
    }

    // Show best solutions
    println!("\n━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━");
    println!("          BEST SOLUTIONS");
    println!("━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━\n");

    if let Some(solution) = result_ga.get(0) {
        println!("Genetic Algorithm solution:");
        let vars = solution.get_solution_info().get_variables();
        let total_weight: f64 = vars.iter().zip(weights.iter())
            .filter(|(selected, _)| **selected)
            .map(|(_, w)| w)
            .sum();
        let total_value: f64 = vars.iter().zip(values.iter())
            .filter(|(selected, _)| **selected)
            .map(|(_, v)| v)
            .sum();
        
        println!("  Items selected: {:?}", vars);
        println!("  Total weight: {:.2} / {:.2}", total_weight, capacity);
        println!("  Total value: {:.2}", total_value);
        println!("  Fitness: {:.2}\n", solution.value());
    }

    if let Some(solution) = result_hc.get(0) {
        println!("Hill Climbing solution:");
        let vars = solution.get_solution_info().get_variables();
        let total_weight: f64 = vars.iter().zip(weights.iter())
            .filter(|(selected, _)| **selected)
            .map(|(_, w)| w)
            .sum();
        let total_value: f64 = vars.iter().zip(values.iter())
            .filter(|(selected, _)| **selected)
            .map(|(_, v)| v)
            .sum();
        
        println!("  Items selected: {:?}", vars);
        println!("  Total weight: {:.2} / {:.2}", total_weight, capacity);
        println!("  Total value: {:.2}", total_value);
        println!("  Fitness: {:.2}\n", solution.value());
    }

    println!("✅ Charts have been generated in 'output/comparison/' directory");
    println!("   - genetic_algorithm/convergence.png");
    println!("   - genetic_algorithm/evaluations.png");
    println!("   - hill_climbing/convergence.png");
    println!("   - hill_climbing/evaluations.png");
}
