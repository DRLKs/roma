use std::collections::HashMap;
use std::path::{Path, PathBuf};

use roma_lib::{ChartObserver, HtmlReportObserver, Observable};
use roma_lib::algorithms::{
    Algorithm,
    GeneticAlgorithm,
    GeneticAlgorithmParameters,
    TerminationCriteria,
    TerminationCriterion,
};
use roma_lib::operator::{BinaryTournamentSelection, BitFlipMutation, SinglePointCrossover};
use roma_lib::problem::build_knapsack_from_records;
use roma_lib::solution_set::SolutionSet;
use roma_lib::utils::cli::{
    argument_value,
    infer_format_from_extension,
    parse_f64_flag_or,
    parse_usize_flag_or,
    resolve_path_from_flag_or_default,
    seed_from_cli_or,
};
use roma_lib::utils::csv_adapter::read_csv_records;
use roma_lib::utils::json_adapter::{get_json_value, read_json_records};
use roma_lib::utils::yaml_adapter::{get_yaml_value, read_yaml_records};

#[derive(Debug, Clone, Copy)]
enum InputFormat {
    Csv,
    Json,
    Yaml,
}

fn print_help_if_requested() -> bool {
    let wants_help = std::env::args().skip(1).any(|arg| arg == "--help" || arg == "-h");
    if !wants_help {
        return false;
    }

    println!(
        "Knapsack Items File Demo\n\
Usage:\n\
    cargo run --example knapsack_items_file -- [OPTIONS]\n\n\
Options:\n\
    --help, -h               Show this help message and exit\n\
    --seed <u64>             Random seed for reproducible runs (default: 42)\n\
    --input <path>           Input file path (.csv, .json, .yaml, .yml)\n\
    --format <fmt>           Input format override: csv | json | yaml\n\
                                                     If omitted, format is inferred from file extension\n\
    --capacity <f64>         (CSV only) Knapsack capacity (default: 400.0)\n\
    --limit <usize>          (CSV only) Maximum number of records (default: 120)\n\n\
Record mapping policy (configured in code, not by CLI):\n\
    CSV  -> records=root array, weight='weight', value='value'\n\
    JSON -> records='dataset.items', weight='attributes.weight', value='attributes.value'\n\
    YAML -> records='dataset.items', weight='attributes.weight', value='attributes.value'\n\n\
Capacity/limit source:\n\
    CSV  -> CLI flags (or defaults)\n\
    JSON -> fields problem.capacity and problem.limit in file\n\
    YAML -> fields problem.capacity and problem.limit in file\n\n\
Examples:\n\
    cargo run --example knapsack_items_file -- --input examples/knapsack_items_file/items.csv\n\
    cargo run --example knapsack_items_file -- --input examples/knapsack_items_file/items.json\n\
    cargo run --example knapsack_items_file -- --input examples/knapsack_items_file/items.yaml\n"
    );

    true
}

fn resolve_input_path() -> PathBuf {
    let default_path = PathBuf::from(env!("CARGO_MANIFEST_DIR"))
        .join("examples")
        .join("knapsack_items_file")
        .join("items.csv");

    resolve_path_from_flag_or_default("--input", default_path)
}

fn resolve_input_format(path: &Path) -> Result<InputFormat, String> {
    if let Some(raw) = argument_value("--format") {
        let normalized = raw.trim().to_ascii_lowercase();
        return match normalized.as_str() {
            "csv" => Ok(InputFormat::Csv),
            "json" => Ok(InputFormat::Json),
            "yaml" | "yml" => Ok(InputFormat::Yaml),
            _ => Err(format!(
                "Unsupported --format='{}'. Use csv, json or yaml",
                raw
            )),
        };
    }

    infer_format_from_extension(path)
        .and_then(|fmt| match fmt.as_str() {
            "csv" => Some(InputFormat::Csv),
            "json" => Some(InputFormat::Json),
            "yaml" => Some(InputFormat::Yaml),
            _ => None,
        })
        .ok_or_else(|| {
            format!(
                "Could not infer format from extension '{}'. Use --format=csv|json|yaml",
                path.display()
            )
        })
}

fn records_path_for_format(input_format: InputFormat) -> &'static str {
    match input_format {
        InputFormat::Csv => "",
        InputFormat::Json | InputFormat::Yaml => "dataset.items",
    }
}

fn weight_key_for_format(input_format: InputFormat) -> &'static str {
    match input_format {
        InputFormat::Csv => "weight",
        InputFormat::Json | InputFormat::Yaml => "attributes.weight",
    }
}

fn value_key_for_format(input_format: InputFormat) -> &'static str {
    match input_format {
        InputFormat::Csv => "value",
        InputFormat::Json | InputFormat::Yaml => "attributes.value",
    }
}

fn read_records_from_input(
    path: &Path,
    input_format: InputFormat,
    records_path: &str,
) -> Result<Vec<HashMap<String, String>>, String> {
    match input_format {
        InputFormat::Csv => read_csv_records(path, ',')
            .map_err(|e| format!("Failed to read CSV '{}': {}", path.display(), e)),
        InputFormat::Json => read_json_records(path, records_path)
            .map_err(|e| format!("Failed to read JSON '{}': {}", path.display(), e)),
        InputFormat::Yaml => read_yaml_records(path, records_path)
            .map_err(|e| format!("Failed to read YAML '{}': {}", path.display(), e)),
    }
}

fn read_json_required_f64(path: &Path, key_path: &str) -> Result<f64, String> {
    let raw = get_json_value(path, key_path)
        .map_err(|e| format!("Failed to read JSON '{}': {}", path.display(), e))?
        .ok_or_else(|| format!("Missing JSON key '{}'", key_path))?;

    raw.parse::<f64>()
        .map_err(|_| format!("JSON key '{}' must be numeric, got '{}'", key_path, raw))
}

fn read_json_required_usize(path: &Path, key_path: &str) -> Result<usize, String> {
    let raw = get_json_value(path, key_path)
        .map_err(|e| format!("Failed to read JSON '{}': {}", path.display(), e))?
        .ok_or_else(|| format!("Missing JSON key '{}'", key_path))?;

    raw.parse::<usize>()
        .map_err(|_| format!("JSON key '{}' must be usize, got '{}'", key_path, raw))
}

fn read_yaml_required_f64(path: &Path, key_path: &str) -> Result<f64, String> {
    let raw = get_yaml_value(path, key_path)
        .map_err(|e| format!("Failed to read YAML '{}': {}", path.display(), e))?
        .ok_or_else(|| format!("Missing YAML key '{}'", key_path))?;

    raw.parse::<f64>()
        .map_err(|_| format!("YAML key '{}' must be numeric, got '{}'", key_path, raw))
}

fn read_yaml_required_usize(path: &Path, key_path: &str) -> Result<usize, String> {
    let raw = get_yaml_value(path, key_path)
        .map_err(|e| format!("Failed to read YAML '{}': {}", path.display(), e))?
        .ok_or_else(|| format!("Missing YAML key '{}'", key_path))?;

    raw.parse::<usize>()
        .map_err(|_| format!("YAML key '{}' must be usize, got '{}'", key_path, raw))
}

fn resolve_capacity_and_limit(path: &Path, input_format: InputFormat) -> Result<(f64, usize), String> {
    match input_format {
        InputFormat::Csv => Ok((
            parse_f64_flag_or("--capacity", 400.0),
            parse_usize_flag_or("--limit", 120),
        )),
        InputFormat::Json => Ok((
            read_json_required_f64(path, "problem.capacity")?,
            read_json_required_usize(path, "problem.limit")?,
        )),
        InputFormat::Yaml => Ok((
            read_yaml_required_f64(path, "problem.capacity")?,
            read_yaml_required_usize(path, "problem.limit")?,
        )),
    }
}

fn main() {
    if print_help_if_requested() {
        return;
    }

    let seed = seed_from_cli_or(42);
    let input_path = resolve_input_path();
    let input_format = resolve_input_format(&input_path).unwrap_or_else(|msg| panic!("{}", msg));
    let records_path = records_path_for_format(input_format);
    let weight_key = weight_key_for_format(input_format);
    let value_key = value_key_for_format(input_format);
    let (capacity, row_limit) =
        resolve_capacity_and_limit(&input_path, input_format).unwrap_or_else(|msg| panic!("{}", msg));

    let records = read_records_from_input(&input_path, input_format, records_path)
        .unwrap_or_else(|msg| panic!("{}", msg));

    let (problem, loaded_items) =
        build_knapsack_from_records(&records, capacity, row_limit, weight_key, value_key)
            .unwrap_or_else(|msg| panic!("{}", msg));

    // Build the algorithm 
    let parameters = GeneticAlgorithmParameters::new(
        80,
        0.85,
        0.04,
        SinglePointCrossover::new(),
        BitFlipMutation::new(),
        BinaryTournamentSelection::new(),
        TerminationCriteria::new(vec![TerminationCriterion::MaxIterations(60)]),
    )
    .with_seed(seed);

    let chart_observer = ChartObserver::new_default();
    let html_observer = HtmlReportObserver::new_default();

    let mut algorithm = GeneticAlgorithm::new(parameters);
    algorithm.add_observer(Box::new(chart_observer));
    algorithm.add_observer(Box::new(html_observer));
    let result = algorithm.run(&problem).expect("Large CSV GA run failed");

    if let Some(best) = result.best_solution() {
        println!(
            "Large dataset GA demo finished (seed={}). input='{}', format={:?}, capacity={}, limit={}, records_path='{}', weight_key='{}', value_key='{}', items={}, best fitness={:.4}",
            seed,
            input_path.display(),
            input_format,
            capacity,
            row_limit,
            records_path,
            weight_key,
            value_key,
            loaded_items,
            best.quality_value()
        );
    } else {
        println!("Large CSV GA demo finished with no solutions (seed={})", seed);
    }
}
