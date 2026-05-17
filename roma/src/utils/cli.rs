use std::path::{Path, PathBuf};
use std::time::{SystemTime, UNIX_EPOCH};
use std::{io, io::Write};

use crate::algorithms::checkpoint::{CheckpointEntry, CheckpointRunStatus};

pub const CLI_FLAG_SEED: &str = "--seed";
pub const CLI_FLAG_SEED_SHORT: &str = "-s";
pub const CLI_FLAG_RESUME: &str = "--resume";
pub const CLI_FLAG_NO_CHECKPOINT: &str = "--no-checkpoint";
pub const CLI_FLAG_NO_CHECKPOINT_SHORT: &str = "--nc";
pub const CLI_FLAG_CHECKPOINT_DIR: &str = "--checkpoint-dir";

const CLI_INLINE_VALUE_SEPARATOR: &str = "=";
const CURRENT_DIR_FALLBACK: &str = ".";

const FORMAT_CSV: &str = "csv";
const FORMAT_JSON: &str = "json";
const FORMAT_YAML: &str = "yaml";
const FORMAT_YML: &str = "yml";

const CHECKPOINT_LOCK_ERROR: &str = "Failed to acquire console lock";
const CHECKPOINT_SELECTION_TITLE: &str = "--- CHECKPOINT SELECTION ---";
const CHECKPOINT_COLUMN_ID: &str = "ID";
const CHECKPOINT_COLUMN_AGE: &str = "AGE";
const CHECKPOINT_COLUMN_ELAPSED: &str = "ELAPSED.";
const CHECKPOINT_COLUMN_INFO: &str = "INFO";
const CHECKPOINT_NEW_RUN_OPTION: &str = " [0] Start a new run (ignore existing)";
const CHECKPOINT_SELECTION_FOOTER: &str = "----------------------------";
const CHECKPOINT_SELECTION_PROMPT: &str = "> Select checkpoint index: ";
const CHECKPOINT_INVALID_SELECTION: &str = "Please enter a valid numeric index.";
const CHECKPOINT_INDEX_OUT_OF_RANGE_PREFIX: &str = "Index ";
const CHECKPOINT_INDEX_OUT_OF_RANGE_SUFFIX: &str = " is out of range.";
const CHECKPOINT_STATUS_RUNNING_ICON: &str = ">";
const CHECKPOINT_STATUS_IDLE_ICON: &str = "[]";

const CHECKPOINT_TABLE_WIDTH: usize = 90;
const CHECKPOINT_AGE_COLUMN_WIDTH: usize = 12;

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct CliArgs {
    args: Vec<String>,
}

impl CliArgs {
    pub fn from_env() -> Self {
        Self::from_iter(std::env::args().skip(1))
    }

    pub fn from_iter<I, S>(args: I) -> Self
    where
        I: IntoIterator<Item = S>,
        S: Into<String>,
    {
        Self {
            args: args.into_iter().map(Into::into).collect(),
        }
    }

    pub fn has_flag(&self, flag: &str) -> bool {
        self.args.iter().any(|arg| arg == flag)
    }

    pub fn has_any_flag(&self, flags: &[&str]) -> bool {
        self.args
            .iter()
            .any(|arg| flags.iter().any(|flag| arg == flag))
    }

    pub fn argument_value(&self, flag: &str) -> Option<String> {
        self.argument_value_for_any(&[flag])
    }

    pub fn argument_value_for_any(&self, flags: &[&str]) -> Option<String> {
        let mut args = self.args.iter();

        while let Some(arg) = args.next() {
            if flags.iter().any(|flag| arg == flag) {
                return args.next().cloned();
            }

            for flag in flags {
                let prefix = format!("{flag}{CLI_INLINE_VALUE_SEPARATOR}");
                if let Some(value) = arg.strip_prefix(&prefix) {
                    return Some(value.to_string());
                }
            }
        }

        None
    }

    pub fn parse_u64_for_any_or(&self, flags: &[&str], default_value: u64) -> u64 {
        self.argument_value_for_any(flags)
            .and_then(|value| value.parse::<u64>().ok())
            .unwrap_or(default_value)
    }

    pub fn seed_or(&self, default_value: u64) -> u64 {
        self.parse_u64_for_any_or(&[CLI_FLAG_SEED_SHORT, CLI_FLAG_SEED], default_value)
    }

    pub fn parse_usize_or(&self, flag: &str, default_value: usize) -> usize {
        self.argument_value(flag)
            .and_then(|value| value.parse::<usize>().ok())
            .unwrap_or(default_value)
    }

    pub fn parse_f64_or(&self, flag: &str, default_value: f64) -> f64 {
        self.argument_value(flag)
            .and_then(|value| value.parse::<f64>().ok())
            .unwrap_or(default_value)
    }

    pub fn parse_string_or(&self, flag: &str, default_value: &str) -> String {
        self.argument_value(flag)
            .unwrap_or_else(|| default_value.to_string())
    }

    pub fn resume_requested(&self) -> bool {
        self.has_flag(CLI_FLAG_RESUME)
    }

    pub fn checkpoints_disabled(&self) -> bool {
        self.has_any_flag(&[CLI_FLAG_NO_CHECKPOINT, CLI_FLAG_NO_CHECKPOINT_SHORT])
    }

    pub fn has_checkpoint_dir_override(&self) -> bool {
        self.argument_value(CLI_FLAG_CHECKPOINT_DIR).is_some()
    }

    pub fn checkpoint_dir_or(&self, default_path: PathBuf) -> PathBuf {
        self.resolve_path_from_flag_or_default(CLI_FLAG_CHECKPOINT_DIR, default_path)
    }

    pub fn resolve_path_from_flag_or_default(&self, flag: &str, default_path: PathBuf) -> PathBuf {
        if let Some(raw) = self.argument_value(flag) {
            let candidate = PathBuf::from(raw);
            if candidate.is_absolute() {
                return candidate;
            }

            return std::env::current_dir()
                .unwrap_or_else(|_| PathBuf::from(CURRENT_DIR_FALLBACK))
                .join(candidate);
        }

        default_path
    }
}

/// Tries to infer an input format string from file extension.
///
/// Returns lower-case values such as `csv`, `json` or `yaml`.
pub fn infer_format_from_extension(path: &Path) -> Option<String> {
    let ext = path.extension()?.to_string_lossy().to_ascii_lowercase();
    match ext.as_str() {
        FORMAT_CSV => Some(FORMAT_CSV.to_string()),
        FORMAT_JSON => Some(FORMAT_JSON.to_string()),
        FORMAT_YAML | FORMAT_YML => Some(FORMAT_YAML.to_string()),
        _ => None,
    }
}

/// Converts milliseconds to a HH:MM:SS duration string.
fn format_duration(ms: u64) -> String {
    let secs = ms / 1000;
    let h = secs / 3600;
    let m = (secs % 3600) / 60;
    let s = secs % 60;
    format!("{:02}:{:02}:{:02}", h, m, s)
}

/// Calculates relative time from a millisecond timestamp.
fn format_time_ago(created_ms: u64) -> String {
    let now_ms = SystemTime::now()
        .duration_since(UNIX_EPOCH)
        .map(|d| d.as_millis() as u64)
        .unwrap_or(0);

    let diff_ms = now_ms.saturating_sub(created_ms);
    let secs = diff_ms / 1000;

    if secs < 60 {
        format!("{}s ago", secs)
    } else if secs < 3600 {
        format!("{}m ago", secs / 60)
    } else if secs < 86400 {
        format!("{}h {}m ago", secs / 3600, (secs % 3600) / 60)
    } else {
        format!("{} days ago", secs / 86400)
    }
}

pub fn prompt_checkpoint_selection(entries: &[CheckpointEntry]) -> Result<Option<usize>, String> {
    if entries.is_empty() {
        return Ok(None);
    }

    use crate::algorithms::traits::CONSOLE_LOCK;
    let _lock = CONSOLE_LOCK
        .lock()
        .map_err(|_| CHECKPOINT_LOCK_ERROR.to_string())?;

    println!("\n{:^width$}", CHECKPOINT_SELECTION_TITLE, width = CHECKPOINT_TABLE_WIDTH);
    println!(
        "{:<4} | {:<age_width$} | {:<8} | {:<8}",
        CHECKPOINT_COLUMN_ID,
        CHECKPOINT_COLUMN_AGE,
        CHECKPOINT_COLUMN_ELAPSED,
        CHECKPOINT_COLUMN_INFO,
        age_width = CHECKPOINT_AGE_COLUMN_WIDTH,
    );
    println!("{:-<width$}", "", width = CHECKPOINT_TABLE_WIDTH);

    for (index, entry) in entries.iter().enumerate() {
        let rec = &entry.record;

        let age_str = format_time_ago(rec.created_at_ms);
        let time_str = format_duration(rec.elapsed_millis);
        let status_icon = if matches!(rec.status, CheckpointRunStatus::Running) {
            CHECKPOINT_STATUS_RUNNING_ICON
        } else {
            CHECKPOINT_STATUS_IDLE_ICON
        };

        println!(
            "[{:>2}] | {:<age_width$} | {:>8} | {} {:<8.80}",
            index + 1,
            age_str,
            time_str,
            status_icon,
            rec.step_state_payload,
            age_width = CHECKPOINT_AGE_COLUMN_WIDTH,
        );
    }

    println!("{:-<width$}", "", width = CHECKPOINT_TABLE_WIDTH);
    println!("{}", CHECKPOINT_NEW_RUN_OPTION);
    println!("{:^width$}\n", CHECKPOINT_SELECTION_FOOTER, width = CHECKPOINT_TABLE_WIDTH);

    print!("{}", CHECKPOINT_SELECTION_PROMPT);
    io::stdout().flush().map_err(|e| e.to_string())?;

    let mut input = String::new();
    io::stdin()
        .read_line(&mut input)
        .map_err(|e| e.to_string())?;

    let selection = input
        .trim()
        .parse::<usize>()
        .map_err(|_| CHECKPOINT_INVALID_SELECTION.to_string())?;

    if selection == 0 {
        return Ok(None);
    }

    if selection > entries.len() {
        return Err(format!(
            "{}{}{}",
            CHECKPOINT_INDEX_OUT_OF_RANGE_PREFIX,
            selection,
            CHECKPOINT_INDEX_OUT_OF_RANGE_SUFFIX
        ));
    }

    Ok(Some(selection - 1))
}

#[cfg(test)]
mod tests {
    use super::*;

    const TEST_FLAG_RESUME: &str = "--resume";
    const TEST_FLAG_OTHER: &str = "--other";
    const TEST_FLAG_CHECKPOINT_DIR: &str = "--checkpoint-dir";
    const TEST_INLINE_CHECKPOINT_DIR: &str = "--checkpoint-dir=tmp/run";
    const TEST_SEED_VALUE: &str = "7";
    const TEST_CHECKPOINT_DIR_VALUE: &str = "tmp/run";
    const TEST_NESTED_CHECKPOINT_DIR: &str = "nested/checkpoints";
    const TEST_DEFAULT_CHECKPOINT_DIR: &str = "/tmp/default-checkpoints";

    #[test]
    fn cli_args_reads_exact_and_equals_style_values() {
        let args = CliArgs::from_iter([
            TEST_FLAG_RESUME,
            TEST_INLINE_CHECKPOINT_DIR,
            CLI_FLAG_SEED,
            TEST_SEED_VALUE,
        ]);

        assert!(args.has_flag(TEST_FLAG_RESUME));
        assert!(args.has_any_flag(&[TEST_FLAG_RESUME, TEST_FLAG_OTHER]));
        assert_eq!(
            args.argument_value(TEST_FLAG_CHECKPOINT_DIR),
            Some(TEST_CHECKPOINT_DIR_VALUE.to_string())
        );
        assert_eq!(
            args.argument_value_for_any(&[CLI_FLAG_SEED_SHORT, CLI_FLAG_SEED]),
            Some(TEST_SEED_VALUE.to_string())
        );
    }

    #[test]
    fn cli_args_parses_scalar_values_from_flags() {
        let args = CliArgs::from_iter([
            CLI_FLAG_SEED,
            TEST_SEED_VALUE,
            CLI_FLAG_RESUME,
            CLI_FLAG_NO_CHECKPOINT,
            "--checkpoint-dir=checkpoint-dir",
            "--iterations=32",
            "--cooling=0.75",
            "--label=demo",
        ]);

        assert_eq!(args.seed_or(11), 7);
        assert_eq!(args.parse_usize_or("--iterations", 10), 32);
        assert_eq!(args.parse_f64_or("--cooling", 1.0), 0.75);
        assert_eq!(args.parse_string_or("--label", "fallback"), "demo".to_string());
        assert!(args.resume_requested());
        assert!(args.checkpoints_disabled());
        assert!(args.has_checkpoint_dir_override());
    }

    #[test]
    fn cli_args_resolves_relative_paths_against_current_dir() {
        let args = CliArgs::from_iter([TEST_FLAG_CHECKPOINT_DIR, TEST_NESTED_CHECKPOINT_DIR]);
        let default_path = PathBuf::from(TEST_DEFAULT_CHECKPOINT_DIR);

        let resolved = args.checkpoint_dir_or(default_path);

        assert_eq!(
            resolved,
            std::env::current_dir()
                .expect("current working directory should be available")
                .join(TEST_NESTED_CHECKPOINT_DIR)
        );
    }
}