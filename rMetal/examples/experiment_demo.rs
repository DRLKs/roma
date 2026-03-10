use rmetal::algorithms::{
    HillClimbingExperiment,
    HillClimbingExperimentConfig,
    HillClimbingParameters,
    TerminationCriteria,
    TerminationCriterion,
};
use rmetal::experiment::{AlgorithmConfiguration, Experiment, Objective};
use rmetal::operator::BitFlipMutation;
use rmetal::problem::{KnapsackBuilder, KnapsackProblem};
use rmetal::utils::cli::seed_from_cli_or;

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

fn main() {
    let base_seed = seed_from_cli_or(42);
    let runs = 30;

    let hc_configs = [
        HcConfig {
            label: "exploration",
            iterations: 120,
            mutation_probability: 0.10,
        },
        HcConfig {
            label: "exploitation",
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
        let mut hc_experiment =
            HillClimbingExperiment::new("HC", move || build_problem(instance));

        for cfg in hc_configs {
            let parameters = HillClimbingParameters::new(
                BitFlipMutation::new(),
                cfg.mutation_probability,
                TerminationCriteria::new(vec![TerminationCriterion::MaxIterations(cfg.iterations)]),
            );

            let configuration = AlgorithmConfiguration::new(
                cfg.label,
                HillClimbingExperimentConfig::new(parameters, true),
            )
            .with_attribute("operator", "BitFlipMutation")
            .with_attribute("iterations", cfg.iterations.to_string())
            .with_attribute("mutation_probability", format!("{:.3}", cfg.mutation_probability));

            hc_experiment = hc_experiment.add_configuration(configuration);
        }

        experiment = experiment.add_experimentable_algorithm(instance, hc_experiment);
    }

    let report = experiment.execute();
    let output_path = "output/experiments/knapsack_benchmark.json";
    match report.write_json(output_path) {
        Ok(()) => println!("Experiment JSON report written to: {}", output_path),
        Err(error) => eprintln!("Failed to write experiment report JSON: {}", error),
    }
}
