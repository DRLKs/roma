use std::cmp::Ordering;
use std::path::{Path, PathBuf};

use roma_lib::algorithms::{
    Algorithm,
    NSGAII,
    NSGAIIParameters,
    TerminationCriteria,
    TerminationCriterion,
};
use roma_lib::operator::{MultiObjectiveTournamentSelection, PolynomialMutation, SBXCrossover};
use roma_lib::problem::ZDT1Problem;
use roma_lib::solution::ParetoCrowdingDistanceQuality;
use roma_lib::solution_set::SolutionSet;
use roma_lib::utils::json_adapter::{get_json_array_values, get_json_value};
use roma_lib::utils::{measure_result, process_cpu_time_ms};

struct BenchmarkInstance {
    benchmark_id: String,
    problem: String,
    instance_id: String,
    dimension: usize,
    lower_bound: f64,
    upper_bound: f64,
    reference_point: Vec<f64>,
}

struct BudgetSpec {
    r#type: String,
    value: usize,
}

struct BenchmarkConfig {
    benchmark_id: String,
    algorithm_family: String,
    runs: usize,
    budget: BudgetSpec,
    seeds: Vec<u64>,
    population_size: usize,
    crossover_probability: f64,
    mutation_probability: f64,
    sbx_distribution_index: f64,
    polynomial_distribution_index: f64,
    threads: usize,
}

struct ParetoPoint {
    variables: Vec<f64>,
    objectives: Vec<f64>,
}

struct BenchmarkResult {
    benchmark_id: String,
    library: String,
    algorithm_family: String,
    problem: String,
    instance_id: String,
    seed: u64,
    budget_type: String,
    budget_value: usize,
    result_metric_name: String,
    final_fitness: f64,
    best_fitness: f64,
    wall_time_ms: f64,
    cpu_time_ms: Option<f64>,
    evaluations: usize,
    pareto_front: Vec<ParetoPoint>,
    convergence_history: Vec<(usize, f64)>,
    status: String,
    error: Option<String>,
}

fn benchmark_root() -> PathBuf {
    Path::new(env!("CARGO_MANIFEST_DIR"))
        .join("..")
        .join("benchmark_suite")
        .join("zdt1_nsga2")
        .join("shared")
}

fn required_json_string(path: &Path, key_path: &str) -> Result<String, String> {
    get_json_value(path, key_path)
        .map_err(|error| format!("failed to read {} from {}: {}", key_path, path.display(), error))?
        .ok_or_else(|| format!("missing '{}' in {}", key_path, path.display()))
}

fn required_json_parsed<T>(path: &Path, key_path: &str) -> Result<T, String>
where
    T: std::str::FromStr,
    T::Err: std::fmt::Display,
{
    let text = required_json_string(path, key_path)?;
    text.parse::<T>()
        .map_err(|error| format!("invalid '{}' in {}: {}", key_path, path.display(), error))
}

fn required_json_array_f64(path: &Path, key_path: &str) -> Result<Vec<f64>, String> {
    get_json_array_values(path, key_path)
        .map_err(|error| format!("failed to read {} from {}: {}", key_path, path.display(), error))?
        .into_iter()
        .map(|value| {
            value.parse::<f64>().map_err(|error| {
                format!("invalid '{}' item '{}' in {}: {}", key_path, value, path.display(), error)
            })
        })
        .collect()
}

fn load_instance(path: &Path) -> Result<BenchmarkInstance, String> {
    Ok(BenchmarkInstance {
        benchmark_id: required_json_string(path, "benchmark_id")?,
        problem: required_json_string(path, "problem")?,
        instance_id: required_json_string(path, "instance_id")?,
        dimension: required_json_parsed(path, "dimension")?,
        lower_bound: required_json_parsed(path, "lower_bound")?,
        upper_bound: required_json_parsed(path, "upper_bound")?,
        reference_point: required_json_array_f64(path, "reference_point")?,
    })
}

fn load_config(path: &Path) -> Result<BenchmarkConfig, String> {
    let seeds = get_json_array_values(path, "seeds")
        .map_err(|error| format!("failed to read seeds from {}: {}", path.display(), error))?
        .into_iter()
        .map(|value| {
            value.parse::<u64>().map_err(|error| {
                format!("invalid seed '{}' in {}: {}", value, path.display(), error)
            })
        })
        .collect::<Result<Vec<u64>, String>>()?;

    Ok(BenchmarkConfig {
        benchmark_id: required_json_string(path, "benchmark_id")?,
        algorithm_family: required_json_string(path, "algorithm_family")?,
        runs: required_json_parsed(path, "runs")?,
        budget: BudgetSpec {
            r#type: required_json_string(path, "budget.type")?,
            value: required_json_parsed(path, "budget.value")?,
        },
        seeds,
        population_size: required_json_parsed(path, "roma.population_size")?,
        crossover_probability: required_json_parsed(path, "roma.crossover_probability")?,
        mutation_probability: required_json_parsed(path, "roma.mutation_probability")?,
        sbx_distribution_index: required_json_parsed(path, "roma.sbx_distribution_index")?,
        polynomial_distribution_index: required_json_parsed(path, "roma.polynomial_distribution_index")?,
        threads: required_json_parsed(path, "roma.threads")?,
    })
}

fn dominates(left: &[f64], right: &[f64]) -> bool {
    if left.len() != right.len() {
        return false;
    }

    let mut strictly_better = false;
    for (&l, &r) in left.iter().zip(right.iter()) {
        if l > r {
            return false;
        }
        if l < r {
            strictly_better = true;
        }
    }
    strictly_better
}

fn compute_non_dominated_front(
    population: &impl SolutionSet<f64, ParetoCrowdingDistanceQuality>,
) -> Vec<ParetoPoint> {
    let mut candidates = Vec::new();

    for solution in population.iter() {
        if let Some(objectives) = solution.get_objectives() {
            candidates.push(ParetoPoint {
                variables: solution.variables().to_vec(),
                objectives: objectives.to_vec(),
            });
        }
    }

    let mut front = Vec::new();
    for (index, candidate) in candidates.iter().enumerate() {
        let dominated = candidates.iter().enumerate().any(|(other_index, other)| {
            index != other_index && dominates(&other.objectives, &candidate.objectives)
        });
        if !dominated {
            front.push(ParetoPoint {
                variables: candidate.variables.clone(),
                objectives: candidate.objectives.clone(),
            });
        }
    }

    front.sort_by(|left, right| {
        left.objectives[0]
            .partial_cmp(&right.objectives[0])
            .unwrap_or(Ordering::Equal)
            .then_with(|| {
                left.objectives[1]
                    .partial_cmp(&right.objectives[1])
                    .unwrap_or(Ordering::Equal)
            })
    });
    front
}

fn hypervolume_2d(front: &[ParetoPoint], reference_point: &[f64]) -> f64 {
    if reference_point.len() != 2 {
        return 0.0;
    }

    let mut filtered: Vec<(f64, f64)> = front
        .iter()
        .filter_map(|point| {
            if point.objectives.len() != 2 {
                return None;
            }
            let f1 = point.objectives[0];
            let f2 = point.objectives[1];
            if f1.is_finite()
                && f2.is_finite()
                && f1 <= reference_point[0]
                && f2 <= reference_point[1]
            {
                Some((f1, f2))
            } else {
                None
            }
        })
        .collect();

    filtered.sort_by(|left, right| left.0.partial_cmp(&right.0).unwrap_or(Ordering::Equal));

    let mut hypervolume = 0.0;
    let mut previous_f2 = reference_point[1];
    for (f1, f2) in filtered {
        if f2 < previous_f2 {
            hypervolume += (reference_point[0] - f1).max(0.0) * (previous_f2 - f2);
            previous_f2 = f2;
        }
    }
    hypervolume
}

fn json_escape(value: &str) -> String {
    let mut escaped = String::with_capacity(value.len());
    for ch in value.chars() {
        match ch {
            '"' => escaped.push_str("\\\""),
            '\\' => escaped.push_str("\\\\"),
            '\n' => escaped.push_str("\\n"),
            '\r' => escaped.push_str("\\r"),
            '\t' => escaped.push_str("\\t"),
            _ => escaped.push(ch),
        }
    }
    escaped
}

fn json_string(value: &str) -> String {
    format!("\"{}\"", json_escape(value))
}

fn format_f64_array(values: &[f64]) -> String {
    let items: Vec<String> = values.iter().map(|value| value.to_string()).collect();
    format!("[{}]", items.join(", "))
}

fn format_pareto_front(points: &[ParetoPoint]) -> String {
    let items: Vec<String> = points
        .iter()
        .map(|point| {
            format!(
                "{{\"variables\": {}, \"objectives\": {}}}",
                format_f64_array(&point.variables),
                format_f64_array(&point.objectives)
            )
        })
        .collect();
    format!("[{}]", items.join(", "))
}

fn format_convergence_history(points: &[(usize, f64)]) -> String {
    let items: Vec<String> = points
        .iter()
        .map(|(evaluation, value)| format!("[{}, {}]", evaluation, value))
        .collect();
    format!("[{}]", items.join(", "))
}

fn format_optional_number(value: Option<f64>) -> String {
    match value {
        Some(number) => number.to_string(),
        None => "null".to_string(),
    }
}

fn format_optional_error(value: &Option<String>) -> String {
    match value {
        Some(text) => json_string(text),
        None => "null".to_string(),
    }
}

fn format_result_json(result: &BenchmarkResult) -> String {
    let lines = vec![
        "  {".to_string(),
        format!("    \"benchmark_id\": {},", json_string(&result.benchmark_id)),
        format!("    \"library\": {},", json_string(&result.library)),
        format!("    \"algorithm_family\": {},", json_string(&result.algorithm_family)),
        format!("    \"problem\": {},", json_string(&result.problem)),
        format!("    \"instance_id\": {},", json_string(&result.instance_id)),
        format!("    \"seed\": {},", result.seed),
        format!("    \"budget_type\": {},", json_string(&result.budget_type)),
        format!("    \"budget_value\": {},", result.budget_value),
        format!("    \"result_metric_name\": {},", json_string(&result.result_metric_name)),
        format!("    \"final_fitness\": {},", result.final_fitness),
        format!("    \"best_fitness\": {},", result.best_fitness),
        format!("    \"best_solution\": null,"),
        format!("    \"pareto_front\": {},", format_pareto_front(&result.pareto_front)),
        format!(
            "    \"convergence_history\": {},",
            format_convergence_history(&result.convergence_history)
        ),
        format!("    \"wall_time_ms\": {},", result.wall_time_ms),
        format!("    \"cpu_time_ms\": {},", format_optional_number(result.cpu_time_ms)),
        format!("    \"evaluations\": {},", result.evaluations),
        format!("    \"status\": {},", json_string(&result.status)),
        format!("    \"error\": {}", format_optional_error(&result.error)),
        "  }".to_string(),
    ];
    lines.join("\n")
}

fn format_results_json(results: &[BenchmarkResult]) -> String {
    let body = results
        .iter()
        .map(format_result_json)
        .collect::<Vec<String>>()
        .join(",\n");
    format!("[\n{}\n]", body)
}

fn run_once(instance: &BenchmarkInstance, config: &BenchmarkConfig, seed: u64) -> BenchmarkResult {
    let cpu_start = process_cpu_time_ms();
    let measured: Result<(std::time::Duration, (f64, Vec<ParetoPoint>)), String> = measure_result(|| {
        let problem = ZDT1Problem::new(instance.dimension);
        let termination = match config.budget.r#type.as_str() {
            "evaluations" => {
                TerminationCriteria::new(vec![TerminationCriterion::MaxEvaluations(config.budget.value)])
            }
            other => {
                return Err(format!("unsupported budget type '{}'", other));
            }
        };

        let mut parameters = NSGAIIParameters::new(
            config.population_size,
            config.crossover_probability,
            config.mutation_probability,
            SBXCrossover::new(config.sbx_distribution_index),
            PolynomialMutation::new(config.polynomial_distribution_index),
            MultiObjectiveTournamentSelection::new(),
            termination,
        )
        .with_seed(seed);

        parameters = if config.threads <= 1 {
            parameters.sequential()
        } else {
            parameters.with_threads(config.threads)
        };

        let mut algorithm = NSGAII::new(parameters);
        let result = algorithm.run(&problem)?;
        let front = compute_non_dominated_front(&result);
        let hypervolume = hypervolume_2d(&front, &instance.reference_point);
        Ok((hypervolume, front))
    });

    match measured {
        Ok((elapsed, (hypervolume, front))) => {
            let cpu_time_ms = process_cpu_time_ms().zip(cpu_start).map(|(end, start)| end - start);
            BenchmarkResult {
                benchmark_id: config.benchmark_id.clone(),
                library: "roma".to_string(),
                algorithm_family: config.algorithm_family.clone(),
                problem: instance.problem.clone(),
                instance_id: instance.instance_id.clone(),
                seed,
                budget_type: config.budget.r#type.clone(),
                budget_value: config.budget.value,
                result_metric_name: "hypervolume".to_string(),
                final_fitness: hypervolume,
                best_fitness: hypervolume,
                wall_time_ms: elapsed.as_secs_f64() * 1000.0,
                cpu_time_ms,
                evaluations: config.budget.value,
                pareto_front: front,
                convergence_history: vec![(config.budget.value, hypervolume)],
                status: "ok".to_string(),
                error: None,
            }
        }
        Err(error) => BenchmarkResult {
            benchmark_id: config.benchmark_id.clone(),
            library: "roma".to_string(),
            algorithm_family: config.algorithm_family.clone(),
            problem: instance.problem.clone(),
            instance_id: instance.instance_id.clone(),
            seed,
            budget_type: config.budget.r#type.clone(),
            budget_value: config.budget.value,
            result_metric_name: "hypervolume".to_string(),
            final_fitness: 0.0,
            best_fitness: 0.0,
            wall_time_ms: 0.0,
            cpu_time_ms: None,
            evaluations: 0,
            pareto_front: Vec::new(),
            convergence_history: Vec::new(),
            status: "error".to_string(),
            error: Some(error),
        },
    }
}

fn validate_configuration(instance: &BenchmarkInstance, config: &BenchmarkConfig) -> Result<(), String> {
    if instance.benchmark_id != config.benchmark_id {
        return Err(format!(
            "instance benchmark_id '{}' does not match config benchmark_id '{}'",
            instance.benchmark_id, config.benchmark_id
        ));
    }
    if config.budget.r#type != "evaluations" {
        return Err("Roma ZDT1 benchmark currently supports only evaluation budgets".to_string());
    }
    if instance.reference_point.len() != 2 {
        return Err("reference_point must contain exactly two objective values".to_string());
    }
    if instance.lower_bound > instance.upper_bound {
        return Err("lower_bound must be <= upper_bound".to_string());
    }
    if config.seeds.len() < config.runs {
        return Err("config.json must define at least one seed per run".to_string());
    }
    Ok(())
}

fn main() {
    let shared_root = benchmark_root();
    let instance_path = shared_root.join("instance.json");
    let config_path = shared_root.join("config.json");

    let instance = match load_instance(&instance_path) {
        Ok(instance) => instance,
        Err(error) => {
            eprintln!("{}", error);
            std::process::exit(1);
        }
    };

    let config = match load_config(&config_path) {
        Ok(config) => config,
        Err(error) => {
            eprintln!("{}", error);
            std::process::exit(1);
        }
    };

    if let Err(error) = validate_configuration(&instance, &config) {
        eprintln!("{}", error);
        std::process::exit(1);
    }

    let results: Vec<BenchmarkResult> = config
        .seeds
        .iter()
        .copied()
        .take(config.runs)
        .map(|seed| run_once(&instance, &config, seed))
        .collect();

    println!("{}", format_results_json(&results));
}