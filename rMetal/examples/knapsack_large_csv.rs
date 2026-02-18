use rMetal::algorithms::implementations::genetic_algorithm::{GeneticAlgorithm, GeneticAlgorithmParameters};
use rMetal::algorithms::traits::Algorithm;
use rMetal::observer::implementations::chart_observer::ChartObserver;
use rMetal::operator::crossover_operator_implementations::single_point_crossover::SinglePointCrossover;
use rMetal::operator::mutation_operator_implementations::bit_flip_mutation::BitFlipMutation;
use rMetal::operator::selection_operator_implementations::binary_tournament_selection::BinaryTournamentSelection;
use rMetal::problem::implementations::knapsack_problem::KnapsackProblem;
use rMetal::solution_set::traits::SolutionSet;
use rMetal::solutions::traits::Solution;
use rMetal::utils::csv_adapter;
use std::path::Path;
use std::path::PathBuf;


fn main() {
    println!("=== Large Knapsack Problem with CSV Data ===\n");

    // Read CSV data using the CSV adapter
    let csv_path = Path::new("examples/knapsack_large_dataset/items.csv");
    
    println!("Loading items from CSV file: {:?}", csv_path);
    let csv_data = csv_adapter::read_csv(csv_path, ',', true)
        .expect("Failed to read CSV file");
    
    println!("Successfully loaded {} items from CSV\n", csv_data.len());

    // Parse CSV data into weights and values
    let mut weights = Vec::new();
    let mut values = Vec::new();
    let mut items_info = Vec::new();

    for (index, row) in csv_data.iter().enumerate() {
        if row.len() >= 4 {
            let item_id = row[0].clone();
            let weight: f64 = row[1].parse().expect("Failed to parse weight");
            let value: f64 = row[2].parse().expect("Failed to parse value");
            let category = row[3].clone();

            weights.push(weight);
            values.push(value);
            items_info.push((item_id, category, weight, value));
        } else {
            eprintln!("Warning: Row {} has insufficient columns, skipping", index);
        }
    }

    // Calculate optimal capacity (70% of total weight for a challenging problem)
    let total_weight: f64 = weights.iter().sum();
    let capacity = total_weight * 0.4; // 40% of total weight
    
    println!("Problem Statistics:");
    println!("  Total items: {}", weights.len());
    println!("  Total weight: {:.2}", total_weight);
    println!("  Total value: {:.2}", values.iter().sum::<f64>());
    println!("  Knapsack capacity: {:.2} (40% of total weight)", capacity);
    println!("  Value-to-weight ratio: {:.2}", values.iter().sum::<f64>() / total_weight);
    println!();

    // Create the knapsack problem with CSV data
    let problem = KnapsackProblem::with_data(capacity, weights, values);

    // Configure Genetic Algorithm with multiple threads
    let num_threads = 8;
    println!("=== Configuring Genetic Algorithm ===");
    println!("  Population size: 200");
    println!("  Max generations: 100");
    println!("  Crossover probability: 0.9");
    println!("  Mutation probability: 0.02");
    println!("  Parallel threads: {}", num_threads);
    println!();

    let params = GeneticAlgorithmParameters::new(
        200,   // population_size - larger for complex problem
        130,   // max_generations
        0.9,   // crossover_probability
        0.02,  // mutation_probability - lower for larger problems
        SinglePointCrossover::new(),
        BitFlipMutation::new(),
        BinaryTournamentSelection::new(),
    );

    let mut ga = GeneticAlgorithm::new(params);
    
    // Chart observer for visualization
    let output_dir = PathBuf::from("./output/knapsack_ga_large/charts");
    let chart_observer = ChartObserver::new(output_dir);
    ga.add_observer(Box::new(chart_observer));

    println!("=== Starting Genetic Algorithm ===\n");
    let start = std::time::Instant::now();
    let solution_set = ga.run(&problem, 1);
    let duration = start.elapsed();

    println!("\n=== Results ===");
    println!("Execution time: {:.2?}", duration);
    println!("Solutions found: {}", solution_set.size());
    println!();

    // Find best solution across all threads
    if let Some(best) = solution_set.solutions().iter()
        .max_by(|a, b| a.value().partial_cmp(&b.value()).unwrap()) 
    {
        println!("Best Solution:");
        println!("  Fitness (total value): {:.2}", best.value());
        
        // Calculate statistics about the best solution
        let selected_items: Vec<_> = best.get_solution_info().get_variables()
            .iter()
            .enumerate()
            .filter(|&(_, &selected)| selected)
            .collect();
        
        let total_weight: f64 = selected_items.iter()
            .map(|(idx, _)| items_info[*idx].2)
            .sum();
        
        let total_value: f64 = selected_items.iter()
            .map(|(idx, _)| items_info[*idx].3)
            .sum();
        
        println!("  Items selected: {}", selected_items.len());
        println!("  Total weight: {:.2} / {:.2} capacity", total_weight, capacity);
        println!("  Capacity utilization: {:.1}%", (total_weight / capacity) * 100.0);
        println!("  Total value: {:.2}", total_value);
        println!();

        // Show breakdown by category
        let mut category_stats: std::collections::HashMap<String, (usize, f64, f64)> = 
            std::collections::HashMap::new();
        
        for (idx, _) in &selected_items {
            let (_, category, weight, value) = &items_info[*idx];
            let entry = category_stats.entry(category.clone()).or_insert((0, 0.0, 0.0));
            entry.0 += 1;
            entry.1 += weight;
            entry.2 += value;
        }

        println!("Items by Category:");
        for (category, (count, weight, value)) in category_stats.iter() {
            println!("  {}: {} items, weight={:.2}, value={:.2}", 
                category, count, weight, value);
        }
        println!();

        // Show top 10 selected items
        println!("Top 10 Selected Items (by value):");
        let mut selected_with_info: Vec<_> = selected_items.iter()
            .map(|(idx, _)| {
                let (id, category, weight, value) = &items_info[*idx];
                (id, category, weight, value, value / weight)
            })
            .collect();
        
        selected_with_info.sort_by(|a, b| b.3.partial_cmp(a.3).unwrap());
        
        for (i, (id, category, weight, value, ratio)) in selected_with_info.iter().take(10).enumerate() {
            println!("  {}. Item {} ({}): weight={:.2}, value={:.2}, ratio={:.2}", 
                i + 1, id, category, weight, value, ratio);
        }
    }

    println!();
    println!("=== Solution Quality Across All Threads ===");
    let mut fitness_values: Vec<f64> = solution_set.solutions()
        .iter()
        .map(|s| s.value())
        .collect();
    
    fitness_values.sort_by(|a, b| b.partial_cmp(a).unwrap());
    
    for (i, fitness) in fitness_values.iter().enumerate() {
        println!("  Thread {}: fitness = {:.2}", i, fitness);
    }
    
    let avg_fitness: f64 = fitness_values.iter().sum::<f64>() / fitness_values.len() as f64;
    println!("\n  Average fitness: {:.2}", avg_fitness);
    println!("  Best fitness: {:.2}", fitness_values[0]);
    println!("  Worst fitness: {:.2}", fitness_values[fitness_values.len() - 1]);
    println!("  Fitness range: {:.2}", fitness_values[0] - fitness_values[fitness_values.len() - 1]);

    println!("\n=== Demo Complete ===");
}
