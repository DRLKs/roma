use std::time::Duration;

use roma::algorithms::{
    GeneticAlgorithmParameters,
    HillClimbingParameters,
    PSOParameters,
    SimulatedAnnealingParameters,
    TerminationCriteria,
    TerminationCriterion,
};
use roma::experiment::{Experiment, ExperimentReport};
use roma::operator::{BinaryTournamentSelection, BitFlipMutation, SinglePointCrossover};
use roma::problem::KnapsackBuilder;
use roma::utils::{measure_result, speedup};

fn build_problem() -> impl roma::problem::Problem<bool, f64> + Sync {
    KnapsackBuilder::new()
        .with_capacity(150.0)
        .add_item(1.0, 2.0)
        .add_item(2.0, 6.0)
        .add_item(3.0, 7.0)
        .add_item(10.0, 20.0)
        .add_item(20.0, 30.0)
        .add_item(30.0, 60.0)
        .add_item(35.0, 65.0)
        .add_item(45.0, 100.0)
        .add_item(55.0, 120.0)
        .add_item(75.0, 211.0)
        .add_item(80.0, 160.0)
        .add_item(90.0, 301.0)
        .add_item(150.0, 301.0)
        .build()
}

fn run_experiment(parallel: bool, runs: usize) -> Result<(Duration, ExperimentReport), String> {
    let problem = build_problem();

    let hill_climbing_case = HillClimbingParameters::new(
        BitFlipMutation::new(),
        0.12,
        TerminationCriteria::new(vec![TerminationCriterion::MaxIterations(180)]),
    )
    .with_seed(111);

    // Keep GA internally sequential to focus the comparison on experiment-level parallelism.
    let genetic_algorithm_case = GeneticAlgorithmParameters::new(
            80,
            0.90,
            0.06,
            SinglePointCrossover::new(),
            BitFlipMutation::new(),
            BinaryTournamentSelection::new(),
            TerminationCriteria::new(vec![TerminationCriterion::MaxIterations(60)]),
        )
        .with_elite_size(1)
        .with_seed(222)
        .sequential();

    let simulated_annealing_case = SimulatedAnnealingParameters::new(
        BitFlipMutation::new(),
        0.10,
        45.0,
        0.985,
        TerminationCriteria::new(vec![TerminationCriterion::MaxIterations(220)]),
    )
    .with_seed(333);

    let pso_case = PSOParameters::new(
        50,
        0.72,
        1.49,
        1.49,
        TerminationCriteria::new(vec![TerminationCriterion::MaxIterations(120)]),
    )
    .with_velocity_clamp(4.0)
    .with_seed(444);

    let experiment = Experiment::new(problem)
        .with_runs(runs)
        .add_case(hill_climbing_case)
        .add_case(genetic_algorithm_case)
        .add_case(simulated_annealing_case)
        .add_case(pso_case);

    measure_result(|| {
        if parallel {
            experiment.with_parallel().execute()
        } else {
            experiment.sequential().execute()
        }
    })
}

fn main() {
    let runs = 24;

    println!("Simple benchmark: Sequential vs parallel experiment execution");
    println!("Cases: 4 | Runs per case: {}", runs);

    let (sequential_time, sequential_report) = match run_experiment(false, runs) {
        Ok(value) => value,
        Err(error) => {
            eprintln!("Sequential execution error: {}", error);
            return;
        }
    };

    let (parallel_time, parallel_report) = match run_experiment(true, runs) {
        Ok(value) => value,
        Err(error) => {
            eprintln!("Parallel execution error: {}", error);
            return;
        }
    };

    let speedup = speedup(sequential_time, parallel_time);

    println!("\nTiming results:");
    println!("  Sequential: {:?}", sequential_time);
    println!("  Parallel  : {:?}", parallel_time);
    println!("  Speedup   : {:.2}x", speedup);

    println!("\nSequential summary (top 2 by best):");
    for summary in sequential_report.summaries.iter().take(2) {
        println!(
            "  - {} | best={:.4} mean={:.4}",
            summary.case_name, summary.best, summary.mean
        );
    }

    println!("\nParallel summary (top 2 by best):");
    for summary in parallel_report.summaries.iter().take(2) {
        println!(
            "  - {} | best={:.4} mean={:.4}",
            summary.case_name, summary.best, summary.mean
        );
    }
}
