use std::collections::HashMap;
use std::path::{Path, PathBuf};

use rmetal::{ChartObserver, HtmlReportObserver, Observable};
use rmetal::algorithms::{
    Algorithm,
    GeneticAlgorithm,
    GeneticAlgorithmParameters,
    TerminationCriteria,
    TerminationCriterion,
};
use rmetal::operator::{BinaryTournamentSelection, BitFlipMutation, SinglePointCrossover};
use rmetal::problem::KnapsackBuilder;
use rmetal::solution_set::SolutionSet;
use rmetal::utils::cli::{
    argument_value,
    infer_format_from_extension,
    parse_f64_flag_or,
    parse_string_flag_or,
    parse_usize_flag_or,
    resolve_path_from_flag_or_default,
    seed_from_cli_or,
};
use rmetal::utils::csv_adapter::read_csv_records;
use rmetal::utils::json_adapter::read_json_records;
use rmetal::utils::yaml_adapter::read_yaml_records;

// CLI usage notes:
// - Cargo intercepts `--help` before your binary receives it.
// - To forward flags to this example, use `--`.
//
// Example:
//   cargo run --example knapsack_items_file -- --input examples/knapsack_items_file/items.json --records-path dataset.items
//
// Supported flags for this example:
//   --seed <u64>
//   --input <path>
//   --format <csv|json|yaml>
//   --capacity <f64>
//   --limit <usize>
//   --records-path <path>
//   --weight-key <key>
//   --value-key <key>

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
    --capacity <f64>         Knapsack capacity (default: 400.0)\n\
    --limit <usize>          Maximum number of records to read (default: 120)\n\
    --records-path <path>    JSON/YAML path to the records array (default: empty = root array)\n\
    --weight-key <key>       Record key for item weight (default: weight)\n\
    --value-key <key>        Record key for item value (default: value)\n\n\
Examples:\n\
    cargo run --example knapsack_items_file -- --input examples/knapsack_items_file/items.csv\n\
    cargo run --example knapsack_items_file -- --input examples/knapsack_items_file/items.json --records-path dataset.items --weight-key attributes.weight --value-key attributes.value\n\
    cargo run --example knapsack_items_file -- --input examples/knapsack_items_file/items.yaml --records-path dataset.items --weight-key attributes.weight --value-key attributes.value\n"
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

fn load_knapsack_from_records(
    records: Vec<std::collections::HashMap<String, String>>,
    capacity: f64,
    row_limit: usize,
    weight_key: &str,
    value_key: &str,
) -> Result<(rmetal::KnapsackProblem, usize), String> {
    if records.is_empty() {
        return Err("Input data has no records".to_string());
    }

    let mut builder = KnapsackBuilder::new().with_capacity(capacity);
    let mut loaded_items = 0usize;

    for record in records.iter().take(row_limit) {
        let Some(weight_text) = record.get(weight_key) else {
            continue;
        };
        let Some(value_text) = record.get(value_key) else {
            continue;
        };

        let Ok(weight) = weight_text.parse::<f64>() else {
            continue;
        };
        let Ok(value) = value_text.parse::<f64>() else {
            continue;
        };

        builder = builder.add_item(weight, value);
        loaded_items += 1;
    }

    if loaded_items == 0 {
        return Err(format!(
            "No valid records found. Ensure keys '{}' and '{}' exist and are numeric",
            weight_key, value_key
        ));
    }

    Ok((builder.build(), loaded_items))
}

fn main() {
    if print_help_if_requested() {
        return;
    }

    let seed = seed_from_cli_or(42);
    let input_path = resolve_input_path();
    let input_format = resolve_input_format(&input_path).unwrap_or_else(|msg| panic!("{}", msg));
    let capacity = parse_f64_flag_or("--capacity", 400.0);
    let row_limit = parse_usize_flag_or("--limit", 120);
    let records_path = parse_string_flag_or("--records-path", "");
    let weight_key = parse_string_flag_or("--weight-key", "weight");
    let value_key = parse_string_flag_or("--value-key", "value");

    let records = read_records_from_input(&input_path, input_format, &records_path)
        .unwrap_or_else(|msg| panic!("{}", msg));

    let (problem, loaded_items) = load_knapsack_from_records(
        records,
        capacity,
        row_limit,
        &weight_key,
        &value_key,
    )
    .unwrap_or_else(|msg| panic!("{}", msg));

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
    let result = algorithm
        .run(&problem)
        .expect("Large CSV GA run failed");

    if let Some(best) = result.best_solution() {
        println!(
            "Large dataset GA demo finished (seed={}). input='{}', format={:?}, items={}, best fitness={:.4}",
            seed,
            input_path.display(),
            input_format,
            loaded_items,
            best.quality_value()
        );
    } else {
        println!("Large CSV GA demo finished with no solutions (seed={})", seed);
    }
}
