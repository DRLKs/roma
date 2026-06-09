use std::fmt::Display;
use std::path::{Path, PathBuf};
use std::sync::atomic::{AtomicUsize, Ordering};
use std::sync::Arc;
use std::time::Duration;

use roma_lib::algorithms::{
    Algorithm,
    GeneticAlgorithm,
    GeneticAlgorithmParameters,
    TerminationCriteria,
    TerminationCriterion,
};
use roma_lib::operator::{BinaryTournamentSelection, OrderCrossover, SwapMutation};
use roma_lib::problem::TspProblem;
use roma_lib::Problem;
use roma_lib::solution::{RealBounds, Solution};
use roma_lib::solution_set::SolutionSet;
use roma_lib::utils::json_adapter::{get_json_array_values, get_json_number_matrix, get_json_value};
use roma_lib::utils::random::Random;
use roma_lib::utils::{measure_result, process_cpu_time_ms};

struct BenchmarkInstance {
    benchmark_id: String,
    problem: String,
    instance_id: String,
    dimension: usize,
    close_tour: bool,
    city_positions: Option<Vec<(f64, f64)>>,
    distance_matrix: Vec<Vec<f64>>,
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
    elite_size: usize,
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
    best_solution: Vec<usize>,
    wall_time_ms: f64,
    cpu_time_ms: Option<f64>,
    evaluations: Option<usize>,
    status: String,
    error: Option<String>,
}

struct CountingTspProblem {
    inner: TspProblem,
    evaluations: Arc<AtomicUsize>,
}

impl CountingTspProblem {
    fn from_inner(inner: TspProblem) -> Self {
        Self {
            inner,
            evaluations: Arc::new(AtomicUsize::new(0)),
        }
    }

    fn evaluation_count(&self) -> usize {
        self.evaluations.load(Ordering::Relaxed)
    }
}

impl Problem<usize> for CountingTspProblem {
    fn new() -> Self {
        Self::from_inner(TspProblem::new())
    }

    fn evaluate(&self, solution: &mut Solution<usize>) {
        self.evaluations.fetch_add(1, Ordering::Relaxed);
        self.inner.evaluate(solution);
    }

    fn create_solution(&self, rng: &mut Random) -> Solution<usize> {
        self.inner.create_solution(rng)
    }

    fn set_problem_description(&mut self, description: String) {
        self.inner.set_problem_description(description);
    }

    fn get_problem_description(&self) -> String {
        self.inner.get_problem_description()
    }

    fn dominates(&self, solution_a: &Solution<usize, f64>, solution_b: &Solution<usize, f64>) -> bool {
        self.inner.dominates(solution_a, solution_b)
    }

    fn better_fitness_fn(&self) -> fn(f64, f64) -> bool {
        self.inner.better_fitness_fn()
    }

    fn real_bounds(&self) -> Option<&RealBounds> {
        self.inner.real_bounds()
    }

    fn get_problem_parameters_payload(&self) -> String {
        self.inner.get_problem_parameters_payload()
    }

    fn format_solution(&self, solution: &Solution<usize>) -> String
    where
        usize: Display,
        f64: Display,
    {
        self.inner.format_solution(solution)
    }
}

fn benchmark_root() -> PathBuf {
    Path::new(env!("CARGO_MANIFEST_DIR"))
        .join("..")
        .join("benchmark_suite")
        .join("tsp_ga")
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
    let distance_matrix = get_json_number_matrix(path, "distance_matrix")
        .map_err(|error| format!("failed to read distance_matrix from {}: {}", path.display(), error))?;
    let city_positions_matrix = get_json_number_matrix(path, "city_positions")
        .map_err(|error| format!("failed to read city_positions from {}: {}", path.display(), error))?;
    let city_positions = if city_positions_matrix.is_empty() {
        None
    } else {
        let mut positions = Vec::with_capacity(city_positions_matrix.len());
        for row in city_positions_matrix {
            if row.len() != 2 {
                return Err("city_positions rows must contain exactly two numbers".to_string());
            }
            positions.push((row[0], row[1]));
        }
        Some(positions)
    };

    Ok(BenchmarkInstance {
        benchmark_id: required_json_string(path, "benchmark_id")?,
        problem: required_json_string(path, "problem")?,
        instance_id: required_json_string(path, "instance_id")?,
        dimension: required_json_parsed(path, "dimension")?,
        close_tour: required_json_parsed(path, "close_tour")?,
        city_positions,
        distance_matrix,
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
        elite_size: required_json_parsed(path, "roma.elite_size")?,
    })
}

fn build_problem(instance: &BenchmarkInstance) -> CountingTspProblem {
    let problem = if let Some(city_positions) = &instance.city_positions {
        TspProblem::from_city_positions(city_positions.clone())
    } else {
        TspProblem::with_distance_matrix(instance.distance_matrix.clone())
    };
    if instance.close_tour {
        CountingTspProblem::from_inner(problem)
    } else {
        CountingTspProblem::from_inner(problem.with_open_route())
    }
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

fn format_usize_array(values: &[usize]) -> String {
    let items: Vec<String> = values.iter().map(|value| value.to_string()).collect();
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

fn format_optional_usize(value: Option<usize>) -> String {
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
        format!("    \"best_solution\": {},", format_usize_array(&result.best_solution)),
        format!("    \"wall_time_ms\": {},", result.wall_time_ms),
        format!("    \"cpu_time_ms\": {},", format_optional_number(result.cpu_time_ms)),
        format!("    \"evaluations\": {},", format_optional_usize(result.evaluations)),
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
    let measured: Result<(Duration, (f64, Vec<usize>, usize)), String> = measure_result(|| {
        let problem = build_problem(instance);
        let termination = match config.budget.r#type.as_str() {
            "evaluations" => {
                TerminationCriteria::new(vec![TerminationCriterion::MaxEvaluations(config.budget.value)])
            }
            "time" => TerminationCriteria::new(vec![TerminationCriterion::TimeLimit(Duration::from_secs(
                config.budget.value as u64,
            ))]),
            other => {
                return Err(format!("unsupported budget type '{}'", other));
            }
        };
        let parameters = GeneticAlgorithmParameters::new(
            config.population_size,
            config.crossover_probability,
            config.mutation_probability,
            OrderCrossover::new(),
            SwapMutation::new(),
            BinaryTournamentSelection::new(),
            termination,
        )
        .with_elite_size(config.elite_size)
        .with_seed(seed)
        .sequential();

        let mut algorithm = GeneticAlgorithm::new(parameters);
        let solution_set = algorithm.run(&problem)?;
        let best_solution = solution_set
            .best_solution(&problem)
            .map(|solution| solution.variables().to_vec())
            .unwrap_or_default();
        let best_fitness = solution_set.best_solution_value_or(&problem, f64::INFINITY);
        let evaluations = problem.evaluation_count();

        Ok((best_fitness, best_solution, evaluations))
    });

    let cpu_time_ms = cpu_start
        .and_then(|start| process_cpu_time_ms().map(|end| end - start));

    match measured {
        Ok((elapsed, (best_fitness, best_solution, evaluations))) => BenchmarkResult {
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
            evaluations: Some(evaluations),
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
            best_fitness: f64::INFINITY,
            best_solution: Vec::new(),
            wall_time_ms: 0.0,
            cpu_time_ms,
            evaluations: None,
            status: "error".to_string(),
            error: Some(error),
        },
    }
}

fn validate_config(instance: &BenchmarkInstance, config: &BenchmarkConfig) -> Result<(), String> {
    if instance.benchmark_id != config.benchmark_id {
        return Err("instance.json and config.json must share the same benchmark_id".to_string());
    }

    if config.budget.r#type != "evaluations" && config.budget.r#type != "time" {
        return Err("TSP benchmark runner currently supports only evaluation or time budgets".to_string());
    }

    if config.budget.value == 0 {
        return Err("budget value must be positive".to_string());
    }

    if config.seeds.len() < config.runs {
        return Err("config.json must define at least one seed per run".to_string());
    }

    if config.population_size < 2 {
        return Err("roma.population_size must be at least 2".to_string());
    }

    if !(0.0..=1.0).contains(&config.crossover_probability) {
        return Err("roma.crossover_probability must be in [0,1]".to_string());
    }

    if !(0.0..=1.0).contains(&config.mutation_probability) {
        return Err("roma.mutation_probability must be in [0,1]".to_string());
    }

    if config.elite_size >= config.population_size {
        return Err("roma.elite_size must be smaller than population_size".to_string());
    }

    if instance.dimension == 0 {
        return Err("instance dimension must be positive".to_string());
    }

    if let Some(city_positions) = &instance.city_positions {
        if city_positions.len() != instance.dimension {
            return Err("city_positions size must match instance dimension".to_string());
        }
    } else {
        if instance.distance_matrix.len() != instance.dimension {
            return Err("distance_matrix size must match instance dimension".to_string());
        }

        if instance
            .distance_matrix
            .iter()
            .any(|row| row.len() != instance.dimension)
        {
            return Err("distance_matrix must be square".to_string());
        }
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