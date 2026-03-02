use rmetal::algorithms::{
    Algorithm,
    GeneticAlgorithm,
    GeneticAlgorithmParameters,
    HillClimbing,
    HillClimbingParameters,
    TerminationCriteria,
    TerminationCriterion,
};
use rmetal::experiment::{Experiment, Objective};
use rmetal::operator::{BinaryTournamentSelection, BitFlipMutation, SinglePointCrossover};
use rmetal::problem::{KnapsackBuilder, KnapsackProblem};
use rmetal::solution_set::SolutionSet;
use rmetal::utils::cli::seed_from_cli_or;

#[derive(Clone, Copy)]
struct GaConfig {
    label: &'static str,
    population: usize,
    generations: usize,
    crossover_probability: f64,
    mutation_probability: f64,
}

#[derive(Clone, Copy)]
struct HcConfig {
    label: &'static str,
    iterations: usize,
    mutation_probability: f64,
}

fn build_problem(instance: &str) -> KnapsackProblem {
    match instance {
        "A" => KnapsackBuilder::new()
            .with_capacity(90.0)
            .add_items(vec![(12.0, 22.0), (15.0, 30.0), (20.0, 35.0), (25.0, 44.0)])
            .build(),
        "B" => KnapsackBuilder::new()
            .with_capacity(140.0)
            .add_items(vec![
                (10.0, 20.0),
                (20.0, 30.0),
                (30.0, 60.0),
                (35.0, 65.0),
                (45.0, 70.0),
                (55.0, 90.0),
            ])
            .build(),
        _ => KnapsackBuilder::new()
            .with_capacity(220.0)
            .add_items(vec![
                (8.0, 10.0),
                (12.0, 24.0),
                (18.0, 35.0),
                (24.0, 45.0),
                (30.0, 58.0),
                (36.0, 68.0),
                (40.0, 72.0),
                (50.0, 92.0),
            ])
            .build(),
    }
}

fn run_ga_once(seed: u64, instance: &'static str, cfg: GaConfig) -> f64 {
    let problem = build_problem(instance);

    let params = GeneticAlgorithmParameters::new(
        cfg.population,
        cfg.crossover_probability,
        cfg.mutation_probability,
        SinglePointCrossover::new(),
        BitFlipMutation::new(),
        BinaryTournamentSelection::new(),
        TerminationCriteria::new(vec![TerminationCriterion::MaxIterations(cfg.generations)]),
    )
    .with_seed(seed)
    .with_elite_size(1);

    let mut algorithm = GeneticAlgorithm::new(params);
    let result = algorithm.run(&problem);

    result.best_solution_value_or(0.0)
}

fn run_hc_once(seed: u64, instance: &'static str, cfg: HcConfig) -> f64 {
    let problem = build_problem(instance);

    let params = HillClimbingParameters::new(
        BitFlipMutation::new(),
        cfg.mutation_probability,
        TerminationCriteria::new(vec![TerminationCriterion::MaxIterations(cfg.iterations)]),
    )
    .with_seed(seed);

    let mut algorithm = HillClimbing::new(params, true);
    let result = algorithm.run(&problem);

    result.best_solution_value_or(0.0)
}

fn main() {
    let base_seed = seed_from_cli_or(42);
    let runs = 30;

    let ga_configs = [
        GaConfig {
            label: "C1",
            population: 50,
            generations: 40,
            crossover_probability: 0.85,
            mutation_probability: 0.08,
        },
        GaConfig {
            label: "C2",
            population: 80,
            generations: 60,
            crossover_probability: 0.90,
            mutation_probability: 0.04,
        },
    ];

    let hc_configs = [
        HcConfig {
            label: "C1",
            iterations: 120,
            mutation_probability: 0.10,
        },
        HcConfig {
            label: "C2",
            iterations: 180,
            mutation_probability: 0.06,
        },
    ];

    let problem_instances = ["A", "B", "C"];

    let mut experiment = Experiment::new("knapsack_benchmark")
        .with_runs(runs)
        .with_base_seed(base_seed)
        .with_objective(Objective::Maximize);

    for instance in problem_instances {
        for cfg in ga_configs {
            let c = cfg;
            experiment = experiment.add_case("GA", c.label, instance, move |seed| {
                run_ga_once(seed, instance, c)
            });
        }

        for cfg in hc_configs {
            let c = cfg;
            experiment = experiment.add_case("HC", c.label, instance, move |seed| {
                run_hc_once(seed, instance, c)
            });
        }
    }

    let report = experiment.execute();

    println!(
        "Experiment '{}' finished. Cases={}, runs/case={}, total runs={}",
        report.name,
        report.summaries.len(),
        report.runs_per_case,
        report.run_results.len()
    );

    println!("\nComparison (sorted by mean fitness):");
    for s in report.comparison() {
        println!(
            "  {:>2} {:>2} | best={:8.3} mean={:8.3} std={:7.3} worst={:8.3} runs={}",
            s.algorithm,
            s.configuration,
            s.best,
            s.mean,
            s.std_dev,
            s.worst,
            s.runs
        );
        println!("      problem={}", s.problem);
    }
}
