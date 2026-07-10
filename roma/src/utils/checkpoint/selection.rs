use std::io::Write;
use std::time::{SystemTime, UNIX_EPOCH};

use super::{CheckpointEntry, CheckpointRecord, CheckpointRunStatus};

const TABLE_WIDTH: usize = 90;
const AGE_COLUMN_WIDTH: usize = 12;

pub(super) fn select_checkpoint_record(
    entries: Vec<CheckpointEntry>,
) -> Result<Option<CheckpointRecord>, String> {
    if entries.is_empty() {
        return Ok(None);
    }
    let selected_index = if entries.len() == 1 {
        Some(0)
    } else {
        prompt_selection(&entries)?
    };
    Ok(selected_index.map(|index| entries[index].record.clone()))
}

fn prompt_selection(entries: &[CheckpointEntry]) -> Result<Option<usize>, String> {
    let _lock = crate::algorithms::traits::CONSOLE_LOCK
        .lock()
        .map_err(|_| "failed to acquire console lock".to_string())?;

    println!(
        "\n{:^width$}",
        "--- CHECKPOINT SELECTION ---",
        width = TABLE_WIDTH
    );
    println!(
        "{:<4} | {:<age_width$} | {:<8} | {:<8}",
        "ID",
        "AGE",
        "ELAPSED.",
        "INFO",
        age_width = AGE_COLUMN_WIDTH
    );
    println!("{:-<width$}", "", width = TABLE_WIDTH);
    for (index, entry) in entries.iter().enumerate() {
        let record = &entry.record;
        let status = if matches!(record.status, CheckpointRunStatus::Running) {
            ">"
        } else {
            "[]"
        };
        println!(
            "[{:>2}] | {:<age_width$} | {:>8} | {} {:<8.80}",
            index + 1,
            format_age(record.created_at_ms),
            format_duration(record.elapsed_millis),
            status,
            record.state_progress_summary(),
            age_width = AGE_COLUMN_WIDTH
        );
    }
    println!("{:-<width$}", "", width = TABLE_WIDTH);
    println!(" [0] Start a new run (ignore existing)");
    println!(
        "{:^width$}\n",
        "----------------------------",
        width = TABLE_WIDTH
    );
    print!("> Select checkpoint index: ");
    std::io::stdout().flush().map_err(|err| err.to_string())?;

    let mut input = String::new();
    std::io::stdin()
        .read_line(&mut input)
        .map_err(|err| err.to_string())?;
    let selection = input
        .trim()
        .parse::<usize>()
        .map_err(|_| "please enter a valid numeric index".to_string())?;
    if selection == 0 {
        Ok(None)
    } else if selection <= entries.len() {
        Ok(Some(selection - 1))
    } else {
        Err(format!("checkpoint index {} is out of range", selection))
    }
}

fn format_duration(ms: u64) -> String {
    let secs = ms / 1_000;
    format!(
        "{:02}:{:02}:{:02}",
        secs / 3_600,
        (secs % 3_600) / 60,
        secs % 60
    )
}

fn format_age(created_ms: u64) -> String {
    let now_ms = SystemTime::now()
        .duration_since(UNIX_EPOCH)
        .map(|duration| duration.as_millis() as u64)
        .unwrap_or(0);
    let secs = now_ms.saturating_sub(created_ms) / 1_000;
    if secs < 60 {
        format!("{}s ago", secs)
    } else if secs < 3_600 {
        format!("{}m ago", secs / 60)
    } else if secs < 86_400 {
        format!("{}h {}m ago", secs / 3_600, (secs % 3_600) / 60)
    } else {
        format!("{} days ago", secs / 86_400)
    }
}
