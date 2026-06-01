use std::path::{Path, PathBuf};
use std::time::Duration;

use roma_lib::algorithms::{
    Algorithm,
    SimulatedAnnealing,
    SimulatedAnnealingParameters,
    TerminationCriteria,
    TerminationCriterion,
};
use roma_lib::operator::BitFlipNeighborhood;
use roma_lib::problem::KnapsackBuilder;
use roma_lib::solution_set::SolutionSet;
use roma_lib::utils::json_adapter::{get_json_array_values, get_json_value};
use roma_lib::utils::{measure_result, process_cpu_time_ms};

struct BenchmarkInstance {
    benchmark_id: String,
    problem: String,
    instance_id: String,
    capacity: f64,
    weights: Vec<f64>,
    values: Vec<f64>,
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
    roma_initial_temperature: f64,
    roma_cooling_rate: f64,
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
    best_fitness: f64,
    best_solution: Vec<u8>,
    wall_time_ms: f64,
    cpu_time_ms: Option<f64>,
    status: String,
    error: Option<String>,
}

fn benchmark_root() -> PathBuf {
    Path::new(env!("CARGO_MANIFEST_DIR"))
        .join("..")
        .join("benchmark_suite")
        .join("knapsack_sa")
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

fn load_instance(path: &Path) -> Result<BenchmarkInstance, String> {
    let weights_str = get_json_array_values(path, "weights")
        .map_err(|error| format!("failed to read weights from {}: {}", path.display(), error))?;
    let values_str = get_json_array_values(path, "values")
        .map_err(|error| format!("failed to read values from {}: {}", path.display(), error))?;

    let weights = weights_str
        .into_iter()
        .map(|s|
            s.parse::<f64>().map_err(|e| format!("invalid weight '{}' in {}: {}", s, path.display(), e))
        )
        .collect::<Result<Vec<f64>, String>>()?;

    let values = values_str
        .into_iter()
        .map(|s|
            s.parse::<f64>().map_err(|e| format!("invalid value '{}' in {}: {}", s, path.display(), e))
        )
        .collect::<Result<Vec<f64>, String>>()?;

    let capacity = required_json_parsed::<f64>(path, "capacity")?;

    Ok(BenchmarkInstance {
        benchmark_id: required_json_string(path, "benchmark_id")?,
        problem: required_json_string(path, "problem")?,
        instance_id: required_json_string(path, "instance_id")?,
        capacity,
        weights,
        values,
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
        roma_initial_temperature: required_json_parsed(path, "roma.initial_temperature")?,
        roma_cooling_rate: required_json_parsed(path, "roma.cooling_rate")?,
    })
}

fn build_problem(instance: &BenchmarkInstance) -> roma_lib::KnapsackProblem {
    let items: Vec<(f64, f64)> = instance
        .weights
        .iter()
        .cloned()
        .zip(instance.values.iter().cloned())
        .collect();

    KnapsackBuilder::new()
        .with_capacity(instance.capacity)
        .add_items(items)
        .build()
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

fn format_u8_array(values: &[u8]) -> String {
    let items: Vec<String> = values.iter().map(|v| v.to_string()).collect();
    format!("[{}]", items.join(", "))
}

fn format_optional_error(value: &Option<String>) -> String {
    match value {
        Some(text) => json_string(text),
        None => "null".to_string(),
    }
}

fn format_optional_number(value: Option<f64>) -> String {
    match value {
        Some(number) => number.to_string(),
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
        format!("    \"best_fitness\": {},", result.best_fitness),
        format!("    \"best_solution\": {},", format_u8_array(&result.best_solution)),
        format!("    \"wall_time_ms\": {},", result.wall_time_ms),
        format!("    \"cpu_time_ms\": {},", format_optional_number(result.cpu_time_ms)),
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
    let measured: Result<(Duration, (f64, Vec<u8>)), String> = measure_result(|| {
        let problem = build_problem(instance);

        let parameters = SimulatedAnnealingParameters::new(
            BitFlipNeighborhood::new(),
            config.roma_initial_temperature,
            config.roma_cooling_rate,
            TerminationCriteria::new(vec![TerminationCriterion::MaxEvaluations(config.budget.value)]),
        )
        .with_seed(seed);

        let mut algorithm = SimulatedAnnealing::new(parameters);
        let solution_set = algorithm.run(&problem)?;
        let best_solution = solution_set
            .best_solution(&problem)
            .map(|solution| solution.variables().iter().map(|&b| if b { 1u8 } else { 0u8 }).collect())
            .unwrap_or_default();
        let best_fitness = solution_set.best_solution_value_or(&problem, f64::NEG_INFINITY);

        Ok((best_fitness, best_solution))
    });

    let cpu_time_ms = cpu_start.and_then(|start| process_cpu_time_ms().map(|end| end - start));

    match measured {
        Ok((elapsed, (best_fitness, best_solution))) => BenchmarkResult {
            benchmark_id: config.benchmark_id.clone(),
            library: "roma".to_string(),
            algorithm_family: config.algorithm_family.clone(),
            problem: instance.problem.clone(),
            instance_id: instance.instance_id.clone(),
            seed,
            budget_type: config.budget.r#type.clone(),
            budget_value: config.budget.value,
            best_fitness,
            best_solution,
            wall_time_ms: elapsed.as_secs_f64() * 1000.0,
            cpu_time_ms,
            status: "ok".to_string(),
            error: None,
        },
        Err(error) => BenchmarkResult {
            benchmark_id: config.benchmark_id.clone(),
            library: "roma".to_string(),
            algorithm_family: config.algorithm_family.clone(),
            problem: instance.problem.clone(),
            instance_id: instance.instance_id.clone(),
            seed,
            budget_type: config.budget.r#type.clone(),
            budget_value: config.budget.value,
            best_fitness: f64::NEG_INFINITY,
            best_solution: Vec::new(),
            wall_time_ms: 0.0,
            cpu_time_ms,
            status: "error".to_string(),
            error: Some(error),
        },
    }
}

fn validate_config(instance: &BenchmarkInstance, config: &BenchmarkConfig) -> Result<(), String> {
    if instance.benchmark_id != config.benchmark_id {
        return Err("instance.json and config.json must share the same benchmark_id".to_string());
    }

    if config.budget.r#type != "evaluations" {
        return Err("Knapsack benchmark runner currently supports only evaluation budgets".to_string());
    }

    if config.seeds.len() < config.runs {
        return Err("config.json must define at least one seed per run".to_string());
    }

    if instance.weights.len() != instance.values.len() {
        return Err("weights and values arrays must have the same length".to_string());
    }

    Ok(())
}

fn main() -> Result<(), String> {
    let root = benchmark_root();
    let instance = load_instance(&root.join("instance.json"))?;
    let config = load_config(&root.join("config.json"))?;

    validate_config(&instance, &config)?;

    let results: Vec<BenchmarkResult> = config
        .seeds
        .iter()
        .copied()
        .take(config.runs)
        .map(|seed| run_once(&instance, &config, seed))
        .collect();

    println!("{}", format_results_json(&results));
    Ok(())
}
