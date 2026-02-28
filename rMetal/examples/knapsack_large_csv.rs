use std::path::{Path, PathBuf};

use rmetal::{ChartObserver, HtmlReportObserver, Observable};
use rmetal::algorithms::{Algorithm, GeneticAlgorithm, GeneticAlgorithmParameters};
use rmetal::operator::{BinaryTournamentSelection, BitFlipMutation, SinglePointCrossover};
use rmetal::problem::KnapsackBuilder;
use rmetal::solution_set::SolutionSet;
use rmetal::utils::cli::seed_from_cli_or;
use rmetal::utils::csv_adapter::read_csv;

fn argument_value(flag: &str) -> Option<String> {
    let mut args = std::env::args().skip(1);
    while let Some(arg) = args.next() {
        if arg == flag {
            return args.next();
        }

        let prefix = format!("{}=", flag);
        if let Some(value) = arg.strip_prefix(&prefix) {
            return Some(value.to_string());
        }
    }

    None
}

fn resolve_csv_path() -> PathBuf {
    let default_path = PathBuf::from(env!("CARGO_MANIFEST_DIR"))
        .join("examples")
        .join("knapsack_large_dataset")
        .join("items.csv");

    if let Some(raw) = argument_value("--csv") {
        let candidate = PathBuf::from(raw);
        if candidate.is_absolute() {
            return candidate;
        }

        return std::env::current_dir()
            .unwrap_or_else(|_| PathBuf::from("."))
            .join(candidate);
    }

    default_path
}

fn parse_usize_or_default(value: Option<String>, default_value: usize) -> usize {
    value
        .and_then(|x| x.parse::<usize>().ok())
        .unwrap_or(default_value)
}

fn parse_f64_or_default(value: Option<String>, default_value: f64) -> f64 {
    value
        .and_then(|x| x.parse::<f64>().ok())
        .unwrap_or(default_value)
}

fn load_knapsack_from_csv(path: &Path, capacity: f64, row_limit: usize) -> Result<(rmetal::KnapsackProblem, usize), String> {
    let rows = read_csv(path, ',', false)
        .map_err(|e| format!("Failed to read CSV '{}': {}", path.display(), e))?;

    if rows.is_empty() {
        return Err("CSV file is empty".to_string());
    }

    let header = &rows[0];
    let find_column = |name: &str| {
        header
            .iter()
            .position(|h| h.trim().eq_ignore_ascii_case(name))
    };

    let weight_idx = find_column("weight")
        .ok_or_else(|| "CSV must include a 'weight' column".to_string())?;
    let value_idx = find_column("value")
        .ok_or_else(|| "CSV must include a 'value' column".to_string())?;

    let mut builder = KnapsackBuilder::new().with_capacity(capacity);
    let mut loaded_items = 0usize;

    for row in rows.iter().skip(1).take(row_limit) {
        if row.len() <= weight_idx || row.len() <= value_idx {
            continue;
        }

        let Ok(weight) = row[weight_idx].parse::<f64>() else {
            continue;
        };
        let Ok(value) = row[value_idx].parse::<f64>() else {
            continue;
        };

        builder = builder.add_item(weight, value);
        loaded_items += 1;
    }

    if loaded_items == 0 {
        return Err("No valid rows found. Ensure CSV has numeric 'weight' and 'value' values".to_string());
    }

    Ok((builder.build(), loaded_items))
}


fn main() {
    let seed = seed_from_cli_or(42);
    let csv_path = resolve_csv_path();
    let capacity = parse_f64_or_default(argument_value("--capacity"), 400.0);
    let row_limit = parse_usize_or_default(argument_value("--limit"), 120);

    let (problem, loaded_items) = load_knapsack_from_csv(&csv_path, capacity, row_limit)
        .unwrap_or_else(|msg| panic!("{}", msg));

    let parameters = GeneticAlgorithmParameters::new(
        80,
        60,
        0.85,
        0.04,
        SinglePointCrossover::new(),
        BitFlipMutation::new(),
        BinaryTournamentSelection::new(),
    )
    .with_seed(seed);

    let chart_observer = ChartObserver::new_default();
    let html_observer = HtmlReportObserver::new_default();

    let mut algorithm = GeneticAlgorithm::new(parameters);
    algorithm.add_observer(Box::new(chart_observer));
    algorithm.add_observer(Box::new(html_observer));
    let result = algorithm.run(&problem);

    if let Some(best) = result.best_solution() {
        println!(
            "Large CSV GA demo finished (seed={}). csv='{}', items={}, best fitness={:.4}",
            seed,
            csv_path.display(),
            loaded_items,
            best.quality_value()
        );
    } else {
        println!("Large CSV GA demo finished with no solutions (seed={})", seed);
    }
}
