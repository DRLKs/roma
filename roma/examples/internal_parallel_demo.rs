use std::sync::Arc;
use std::time::Duration;

use roma_lib::algorithms::{
    Algorithm,
    GeneticAlgorithm,
    GeneticAlgorithmParameters,
    run_algorithm_instances_async,
    spawn_algorithm_run,
    TerminationCriteria,
    TerminationCriterion,
};
use roma_lib::operator::{BinaryTournamentSelection, BitFlipMutation, SinglePointCrossover};
use roma_lib::problem::{KnapsackBuilder, KnapsackProblem};
use roma_lib::solution_set::SolutionSet;
use roma_lib::utils::{measure_result, speedup};
use roma_lib::utils::cli::CliArgs;

fn build_problem() -> KnapsackProblem {
    let items: Vec<(f64, f64)> = (0..90)
        .map(|i| {
            let weight = 3.0 + ((i * 19 % 47) as f64);
            let value = 9.0 + ((i * 31 % 113) as f64) + weight * 1.2;
            (weight, value)
        })
        .collect();

    KnapsackBuilder::new()
        .with_capacity(1000.0)
        .add_items(items)
        .build()
}

fn ga_params(
    seed: u64,
) -> GeneticAlgorithmParameters<bool, SinglePointCrossover, BitFlipMutation, BinaryTournamentSelection>
{
    GeneticAlgorithmParameters::new(
        120,
        0.90,
        0.05,
        SinglePointCrossover::new(),
        BitFlipMutation::new(),
        BinaryTournamentSelection::new(),
        TerminationCriteria::new(vec![TerminationCriterion::MaxIterations(120)]),
    )
    .with_elite_size(2)
    .with_seed(seed)
    // Keep inner algorithm execution sequential to isolate orchestration impact.
    .sequential()
}

fn benchmark_sequential(problem: &KnapsackProblem, instances: usize, base_seed: u64) -> Result<(Duration, f64), String> {
    measure_result(|| {
        let mut checksum = 0.0;

        for i in 0..instances {
            let mut algorithm = GeneticAlgorithm::new(ga_params(base_seed + i as u64));
            let solution_set = algorithm.run(problem)?;
            checksum += solution_set.best_solution_value_or(problem, 0.0);
        }

        Ok::<f64, String>(checksum)
    })
}

fn benchmark_spawn_runtime(problem: Arc<KnapsackProblem>, instances: usize, base_seed: u64) -> Result<(Duration, f64), String> {
    measure_result(|| {
        let mut handles = Vec::with_capacity(instances);

        for i in 0..instances {
            let algorithm = GeneticAlgorithm::new(ga_params(base_seed + i as u64));
            handles.push(spawn_algorithm_run(algorithm, Arc::clone(&problem)));
        }

        let mut checksum = 0.0;
        for handle in handles {
            let (_algorithm, run_result) = handle
                .join()
                .expect("spawn_algorithm_run worker panicked while executing");
            let solution_set = run_result?;
            checksum += solution_set.best_solution_value_or(problem.as_ref(), 0.0);
        }

        Ok::<f64, String>(checksum)
    })
}

fn benchmark_batch_async(problem: Arc<KnapsackProblem>, instances: usize, base_seed: u64) -> Result<(Duration, f64), String> {
    let algorithms: Vec<GeneticAlgorithm<bool, SinglePointCrossover, BitFlipMutation, BinaryTournamentSelection>> =
        (0..instances)
            .map(|i| GeneticAlgorithm::new(ga_params(base_seed + i as u64)))
            .collect();

    measure_result(|| {
        let results = run_algorithm_instances_async::<
            GeneticAlgorithm<bool, SinglePointCrossover, BitFlipMutation, BinaryTournamentSelection>,
            bool,
            f64,
            KnapsackProblem,
        >(Arc::clone(&problem), algorithms);

        let mut checksum = 0.0;
        for (_algorithm, run_result) in results {
            let solution_set = run_result?;
            checksum += solution_set.best_solution_value_or(problem.as_ref(), 0.0);
        }

        Ok::<f64, String>(checksum)
    })
}

fn main() {
    let seed = CliArgs::from_env().seed_or(10_000u64);
    let instances = 16usize;
    let problem = Arc::new(build_problem());

    println!("Normal-run benchmark with deterministic seeds");
    println!("This example compares orchestration overhead for identical GA runs.");
    println!("Each algorithm run is internally sequential (.sequential()).");
    println!("Instances: {}", instances);
    println!("Modes: sequential | spawn_algorithm_run | run_algorithm_instances_async");

    let (seq_time, seq_sum) = match benchmark_sequential(problem.as_ref(), instances, seed) {
        Ok(v) => v,
        Err(e) => {
            eprintln!("Sequential run failed: {}", e);
            return;
        }
    };

    let (spawn_time, spawn_sum) = match benchmark_spawn_runtime(Arc::clone(&problem), instances, seed) {
        Ok(v) => v,
        Err(e) => {
            eprintln!("spawn_algorithm_run benchmark failed: {}", e);
            return;
        }
    };

    let (async_time, async_sum) = match benchmark_batch_async(Arc::clone(&problem), instances, seed) {
        Ok(v) => v,
        Err(e) => {
            eprintln!("run_algorithm_instances_async benchmark failed: {}", e);
            return;
        }
    };

    println!("\nTiming:");
    println!("  sequential          = {:?}", seq_time);
    println!("  spawn_algorithm_run = {:?}", spawn_time);
    println!("  run_algorithm_instances_async = {:?}", async_time);

    println!("\nSpeedup vs sequential:");
    println!("  spawn_algorithm_run : {:.2}x", speedup(seq_time, spawn_time));
    println!("  run_algorithm_instances_async: {:.2}x", speedup(seq_time, async_time));

    println!("\nChecksums (sum of best objective values):");
    println!("  sequential          = {:.6}", seq_sum);
    println!("  spawn_algorithm_run = {:.6}", spawn_sum);
    println!("  run_algorithm_instances_async = {:.6}", async_sum);
}
