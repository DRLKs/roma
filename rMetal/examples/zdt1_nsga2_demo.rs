use std::path::{Path, PathBuf};
use rMetal::algorithms::implementations::nsga2::{NSGAII, NSGAIIParameters};
use rMetal::algorithms::traits::Algorithm;
use rMetal::observer::implementations::console_observer::ConsoleObserver;
use rMetal::observer::implementations::chart_observer::ChartObserver;
use rMetal::observer::traits::Observable;
use rMetal::operator::crossover_operator_implementations::sbx_crossover::SBXCrossover;
use rMetal::operator::mutation_operator_implementations::polynomial_mutation::PolynomialMutation;
use rMetal::operator::selection_operator_implementations::multi_objective_tournament_selection::MultiObjectiveTournamentSelection;
use rMetal::problem::implementations::zdt1_problem::ZDT1Problem;
use rMetal::problem::traits::Problem;
use rMetal::solution_set::traits::SolutionSet;

fn main() {
    println!("=== NSGA-II on ZDT1 Problem ===\n");

    // Create ZDT1 problem (30 variables, bi-objective)
    let problem = ZDT1Problem::new_default();
    println!("Problem: {}", problem.get_problem_description());
    println!("  Variables: {}", problem.number_of_variables());
    println!("  Objectives: 2 (minimize both)");
    println!("  Pareto front: f2 = 1 - sqrt(f1), where f1 in [0, 1]\n");

    // Configure NSGA-II
    let population_size = 100;
    let max_generations = 250;

    let params = NSGAIIParameters::new(
        population_size,
        max_generations,
        0.9,  // crossover probability
        1.0 / 30.0,  // mutation probability (1/n_variables)
        SBXCrossover::new(20.0),
        PolynomialMutation::new(20.0),
        MultiObjectiveTournamentSelection::new(),
    );

    println!("Algorithm: NSGA-II");
    println!("  Population size: {}", population_size);
    println!("  Max generations: {}", max_generations);
    println!("  Crossover: SBX (η=20)");
    println!("  Mutation: Polynomial (η=20)");
    println!("  Selection: Multi-Objective Tournament\n");

    // Create algorithm
    let mut nsga2 = NSGAII::new(params);

    // Add observer
    nsga2.add_observer(Box::new(ConsoleObserver::new(true)));

    // Chart observer for visualization
    let output_dir = PathBuf::from("output/zdt1_nsga2_demo/charts");
    let chart_observer = ChartObserver::new(output_dir)
        .with_dimensions(1200, 800);
    nsga2.add_observer(Box::new(chart_observer));

    // Run algorithm
    println!("Starting optimization...\n");
    let result = nsga2.run(&problem, 1);

    // Print results
    println!("\n=== Results ===");
    println!("Pareto front size: {}", result.size());

    if result.size() > 0 {
        println!("\nSample solutions from Pareto front:");
        println!("{:<15} {:<15} {:<15}", "f1", "f2", "Pareto f2*");
        println!("{}", "-".repeat(45));

        // Get solutions and sort by first objective
        let mut solutions: Vec<_> = result.solutions().into_iter().cloned().collect();
        solutions.sort_by(|a, b| {
            let f1_a = a.get_objective(0).unwrap_or(f64::MAX);
            let f1_b = b.get_objective(0).unwrap_or(f64::MAX);
            f1_a.partial_cmp(&f1_b).unwrap()
        });

        // Print up to 10 evenly spaced solutions
        let step = if solutions.len() > 10 {
            solutions.len() / 10
        } else {
            1
        };

        for solution in solutions.iter().step_by(step).take(10) {
            if let (Some(f1), Some(f2)) = (solution.get_objective(0), solution.get_objective(1)) {
                let true_pareto_f2 = 1.0 - f1.sqrt();
                println!("{:<15.6} {:<15.6} {:<15.6}", f1, f2, true_pareto_f2);
            }
        }

        println!("\n* Pareto f2 = theoretical value on true Pareto front");
    }

    // Save Pareto front to file
    if let Err(e) = save_pareto_front(&result, "output/zdt1_pareto_front.csv") {
        eprintln!("Error saving Pareto front: {}", e);
    } else {
        println!("\nPareto front saved to: output/zdt1_pareto_front.csv");
    }

    println!("\n=== Optimization Complete ===");
}

fn save_pareto_front(
    solution_set: &impl SolutionSet<f64, rMetal::solutions::implementations::real_solution::RealSolution>,
    filename: &str,
) -> std::io::Result<()> {
    use std::fs;
    use std::io::Write;

    // Create output directory if it doesn't exist
    if let Some(parent) = Path::new(filename).parent() {
        fs::create_dir_all(parent)?;
    }

    let mut file = fs::File::create(filename)?;
    writeln!(file, "f1,f2")?;

    for solution in solution_set.solutions() {
        if let (Some(f1), Some(f2)) = (solution.get_objective(0), solution.get_objective(1)) {
            writeln!(file, "{},{}", f1, f2)?;
        }
    }

    Ok(())
}
