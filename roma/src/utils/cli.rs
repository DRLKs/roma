use std::path::{Path, PathBuf};
use std::{io, io::Write};

use crate::utils::checkpoint::CheckpointEntry;

/// Reads a reproducibility seed from CLI arguments.
///
/// Supported formats:
/// - `--seed 123`
/// - `--seed=123`
/// - `-s 123`
///
/// If no valid seed is provided, returns `default_seed`.
pub fn seed_from_cli_or(default_seed: u64) -> u64 {
    let mut args = std::env::args().skip(1);

    while let Some(arg) = args.next() {
        if arg == "--seed" || arg == "-s" {
            if let Some(value) = args.next() {
                if let Ok(seed) = value.parse::<u64>() {
                    return seed;
                }
            }
            continue;
        }

        if let Some(value) = arg.strip_prefix("--seed=") {
            if let Ok(seed) = value.parse::<u64>() {
                return seed;
            }
        }
    }

    default_seed
}

/// Reads a CLI argument value by flag.
///
/// Supported formats:
/// - `--flag value`
/// - `--flag=value`
pub fn argument_value(flag: &str) -> Option<String> {
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

/// Returns true when a flag appears in CLI args.
///
/// Supports exact matches, for example `--resume`.
pub fn has_flag(flag: &str) -> bool {
    std::env::args().skip(1).any(|arg| arg == flag)
}

/// Resolves a path from a CLI flag. If the flag is missing, returns the provided default path.
///
/// Relative paths are resolved against the current working directory.
pub fn resolve_path_from_flag_or_default(flag: &str, default_path: PathBuf) -> PathBuf {
    if let Some(raw) = argument_value(flag) {
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

/// Parses a `usize` from CLI by flag, returning a default when parsing fails or the flag is missing.
pub fn parse_usize_flag_or(flag: &str, default_value: usize) -> usize {
    argument_value(flag)
        .and_then(|x| x.parse::<usize>().ok())
        .unwrap_or(default_value)
}

/// Parses a `f64` from CLI by flag, returning a default when parsing fails or the flag is missing.
pub fn parse_f64_flag_or(flag: &str, default_value: f64) -> f64 {
    argument_value(flag)
        .and_then(|x| x.parse::<f64>().ok())
        .unwrap_or(default_value)
}

/// Reads a string value from CLI by flag, returning default when missing.
pub fn parse_string_flag_or(flag: &str, default_value: &str) -> String {
    argument_value(flag).unwrap_or_else(|| default_value.to_string())
}

/// Tries to infer an input format string from file extension.
///
/// Returns lower-case values such as `csv`, `json` or `yaml`.
pub fn infer_format_from_extension(path: &Path) -> Option<String> {
    let ext = path.extension()?.to_string_lossy().to_ascii_lowercase();
    match ext.as_str() {
        "csv" => Some("csv".to_string()),
        "json" => Some("json".to_string()),
        "yaml" | "yml" => Some("yaml".to_string()),
        _ => None,
    }
}

/// Prompts user to select one checkpoint entry by index.
///
/// Returns `Ok(None)` when user cancels.
pub fn prompt_checkpoint_selection(entries: &[CheckpointEntry]) -> Result<Option<usize>, String> {
    if entries.is_empty() {
        return Ok(None);
    }

    println!("Found {} resumable checkpoints:", entries.len());
    for (index, entry) in entries.iter().enumerate() {
        println!(
            "  [{}] run_id={} seq={} status={} file={}",
            index + 1,
            entry.record.run_id,
            entry.record.seq_id,
            entry.record.status.as_str(),
            entry.path.display()
        );
    }
    println!("  [0] cancel");

    print!("Select checkpoint index: ");
    io::stdout()
        .flush()
        .map_err(|err| format!("failed to flush prompt: {}", err))?;

    let mut input = String::new();
    io::stdin()
        .read_line(&mut input)
        .map_err(|err| format!("failed to read checkpoint selection: {}", err))?;

    let value = input
        .trim()
        .parse::<usize>()
        .map_err(|_| "invalid checkpoint selection: expected integer index".to_string())?;

    if value == 0 {
        return Ok(None);
    }

    if value > entries.len() {
        return Err(format!(
            "invalid checkpoint selection: expected value in [0, {}]",
            entries.len()
        ));
    }

    Ok(Some(value - 1))
}
