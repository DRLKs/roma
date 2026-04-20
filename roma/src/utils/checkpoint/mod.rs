use std::fs;
use std::io;
use std::io::Read;
use std::path::{Path, PathBuf};
use std::time::{SystemTime, UNIX_EPOCH};

mod binary;
mod hash;
mod path;

use self::binary::{
    byte_to_status, push_f64, push_option_string, push_option_u64, push_string, push_u64,
    push_u8, push_usize, read_f64, read_option_string, read_option_u64, read_string, read_u64,
    read_u8, read_usize, status_to_byte,
};
use self::hash::checkpoint_signature_hashes;
use self::path::{
    checkpoint_file_path, checkpoint_scope_dir, list_checkpoint_files, run_id_timestamp_ms,
};

pub use self::path::{
    ensure_checkpoint_dir, initialize_checkpoint_dir, resolve_checkpoint_dir,
    CheckpointDirSource, CheckpointInitMode, CheckpointInitResult, CheckpointPathConfig,
};

pub const DEFAULT_CHECKPOINT_ENV_VAR: &str = "ROMA_CHECKPOINT_DIR";
pub const DEFAULT_APP_NAME: &str = "roma";
pub const DEFAULT_FREQUENCY_OF_CHECKPOINT_WRITES: usize = 10;

// Binary file signature used to validate checkpoint file integrity.
const CHECKPOINT_BIN_MAGIC: [u8; 4] = *b"RCKP";
const ERR_INVALID_CHECKPOINT_MAGIC: &str = "invalid checkpoint magic header";
const ERR_TRAILING_BYTES: &str = "trailing bytes found in checkpoint file";

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum CheckpointRunStatus {
    Running,
    Completed,
    Failed,
    Interrupted,
}

impl CheckpointRunStatus {
    pub fn as_str(self) -> &'static str {
        match self {
            CheckpointRunStatus::Running => "running",
            CheckpointRunStatus::Completed => "completed",
            CheckpointRunStatus::Failed => "failed",
            CheckpointRunStatus::Interrupted => "interrupted",
        }
    }
}

#[derive(Debug, Clone, PartialEq)]
pub struct CheckpointRecord {
    pub created_at_ms: u64,
    pub run_id: String,
    pub algorithm_name: String,
    pub algorithm_parameters: String,
    pub problem_name: String,
    pub problem_parameters: String,
    pub algorithm_signature_hash: u64,
    pub problem_signature_hash: u64,
    pub seq_id: u64,
    pub iteration: usize,
    pub evaluations: usize,
    pub best_fitness: f64,
    pub average_fitness: f64,
    pub worst_fitness: f64,
    pub best_solution_presentation: String,
    pub state_payload: Option<String>,
    pub termination_criteria_payload: Option<String>,
    pub termination_state_payload: Option<String>,
    pub elapsed_millis: Option<u64>,
    pub status: CheckpointRunStatus,
    pub error_message: Option<String>,
}

impl CheckpointRecord {

    pub fn from_snapshot(
        run_id: impl Into<String>,
        algorithm_name: impl Into<String>,
        algorithm_parameters: impl Into<String>,
        problem_name: impl Into<String>,
        problem_parameters: impl Into<String>,
        algorithm_signature_hash: u64,
        problem_signature_hash: u64,
        seq_id: u64,
        iteration: usize,
        evaluations: usize,
        best_fitness: f64,
        average_fitness: f64,
        worst_fitness: f64,
        best_solution_presentation: impl Into<String>,
        state_payload: Option<String>,
        termination_criteria_payload: Option<String>,
        termination_state_payload: Option<String>,
        elapsed_millis: Option<u64>,
    ) -> Self {
        Self {
            created_at_ms: SystemTime::now()
                .duration_since(UNIX_EPOCH)
                .map(|d| d.as_millis())
                .ok()
                .and_then(|ms| u64::try_from(ms).ok())
                .unwrap_or(0),
            run_id: run_id.into(),
            algorithm_name: algorithm_name.into(),
            algorithm_parameters: algorithm_parameters.into(),
            problem_name: problem_name.into(),
            problem_parameters: problem_parameters.into(),
            algorithm_signature_hash,
            problem_signature_hash,
            seq_id,
            iteration,
            evaluations,
            best_fitness,
            average_fitness,
            worst_fitness,
            best_solution_presentation: best_solution_presentation.into(),
            state_payload,
            termination_criteria_payload,
            termination_state_payload,
            elapsed_millis,
            status: CheckpointRunStatus::Running,
            error_message: None,
        }
    }
}

#[derive(Debug, Clone)]
pub struct CheckpointEntry {
    pub path: PathBuf,
    pub record: CheckpointRecord,
}

/// Binary checkpoint file layout (little-endian):
///
/// Header:
/// - magic: 4 bytes (`RCKP`)
///
/// Identity and matching metadata:
/// - created_at_ms: u64 (UTC epoch millis when this checkpoint was produced)
/// - run_id: string
/// - algorithm_name: string
/// - algorithm_parameters: string
/// - problem_name: string
/// - problem_parameters: string
/// - algorithm_signature_hash: u64
/// - problem_signature_hash: u64
///
/// Progress and quality:
/// - seq_id: u64
/// - iteration: usize encoded as u64
/// - evaluations: usize encoded as u64
/// - best_fitness, average_fitness, worst_fitness: f64
/// - best_solution_presentation: string
///
/// Optional payloads:
/// - state_payload: Option<string> (algorithm-defined UTF-8 payload; e.g. JSON text)
/// - termination_criteria_payload: Option<string>
/// - termination_state_payload: Option<string>
/// - elapsed_millis: Option<u64>
/// - status: u8
/// - error_message: Option<string>
///
/// Example metadata values:
/// - algorithm_name: `HillClimbing`
/// - algorithm_parameters: `mutation_probability=0.20;termination=max_iterations:100`
/// - problem_name: `roma::problem::implementations::tsp_problem::TspProblem`
/// - problem_parameters: `cities=52;close_tour=true;fixed_start_city=none`
/// - algorithm_signature_hash: `11399437687642648721`
/// - problem_signature_hash: `7769642201919903012`

/// Writes a snapshot to the canonical checkpoint location for a run.
pub fn write_snapshot(base_dir: &Path, record: &CheckpointRecord) -> io::Result<PathBuf> {
    write_execution_checkpoint(base_dir, record)
}

/// Writes one checkpoint record under `base_dir` using canonical naming.
/// Returns the full file path used for persistence.
pub(crate) fn write_execution_checkpoint(
    base_dir: &Path,
    record: &CheckpointRecord,
) -> io::Result<PathBuf> {
    let scope_dir = checkpoint_scope_dir(
        base_dir,
        &record.algorithm_name,
        &record.problem_name,
        record.algorithm_signature_hash,
        record.problem_signature_hash,
    );
    fs::create_dir_all(&scope_dir)?;
    let path = checkpoint_file_path(&scope_dir, &record.run_id);
    write_checkpoint_record(&path, record)?;
    Ok(path)
}

/// Writes one checkpoint payload in a compact binary format.
///
/// Note: the file format is always binary. Some fields inside it are UTF-8
/// strings, including `state_payload`, which algorithms may encode as JSON.
pub(crate) fn write_checkpoint_record(path: &Path, record: &CheckpointRecord) -> io::Result<()> {
    let mut bytes = Vec::with_capacity(512);
    bytes.extend_from_slice(&CHECKPOINT_BIN_MAGIC);

    push_u64(&mut bytes, record.created_at_ms);
    push_string(&mut bytes, &record.run_id)?;
    push_string(&mut bytes, &record.algorithm_name)?;
    push_string(&mut bytes, &record.algorithm_parameters)?;
    push_string(&mut bytes, &record.problem_name)?;
    push_string(&mut bytes, &record.problem_parameters)?;
    push_u64(&mut bytes, record.algorithm_signature_hash);
    push_u64(&mut bytes, record.problem_signature_hash);
    push_u64(&mut bytes, record.seq_id);
    push_usize(&mut bytes, record.iteration)?;
    push_usize(&mut bytes, record.evaluations)?;
    push_f64(&mut bytes, record.best_fitness);
    push_f64(&mut bytes, record.average_fitness);
    push_f64(&mut bytes, record.worst_fitness);
    push_string(&mut bytes, &record.best_solution_presentation)?;
    push_option_string(&mut bytes, &record.state_payload)?;
    push_option_string(&mut bytes, &record.termination_criteria_payload)?;
    push_option_string(&mut bytes, &record.termination_state_payload)?;
    push_option_u64(&mut bytes, record.elapsed_millis);
    push_u8(&mut bytes, status_to_byte(record.status));
    push_option_string(&mut bytes, &record.error_message)?;

    fs::write(path, bytes)
}

/// Reads one snapshot record from disk.
pub fn read_snapshot(path: &Path) -> io::Result<CheckpointRecord> {
    read_checkpoint_record(path)
}

/// Deletes a checkpoint file when an execution finishes successfully.
///
/// Runs can persist checkpoints while they are in progress to support resume.
/// Once a run completes without errors, that checkpoint is no longer needed,
/// and this function removes it.
///
/// Returns `Ok(true)` when the file was removed, `Ok(false)` when it did not
/// exist, and `Err(...)` for any other filesystem error.
pub fn delete_snapshot_on_success(path: &Path) -> io::Result<bool> {
    match fs::remove_file(path) {
        Ok(()) => Ok(true),
        Err(err) if err.kind() == io::ErrorKind::NotFound => Ok(false),
        Err(err) => Err(err),
    }
}

/// Reads one checkpoint payload from disk.
pub(crate) fn read_checkpoint_record(path: &Path) -> io::Result<CheckpointRecord> {
    let data = fs::read(path)?;
    let mut cursor = io::Cursor::new(data.as_slice());

    let mut magic = [0u8; 4];
    cursor.read_exact(&mut magic)?;
    if magic != CHECKPOINT_BIN_MAGIC {
        return Err(io::Error::new(
            io::ErrorKind::InvalidData,
            ERR_INVALID_CHECKPOINT_MAGIC,
        ));
    }

    let record = CheckpointRecord {
        created_at_ms: read_u64(&mut cursor)?,
        run_id: read_string(&mut cursor)?,
        algorithm_name: read_string(&mut cursor)?,
        algorithm_parameters: read_string(&mut cursor)?,
        problem_name: read_string(&mut cursor)?,
        problem_parameters: read_string(&mut cursor)?,
        algorithm_signature_hash: read_u64(&mut cursor)?,
        problem_signature_hash: read_u64(&mut cursor)?,
        seq_id: read_u64(&mut cursor)?,
        iteration: read_usize(&mut cursor)?,
        evaluations: read_usize(&mut cursor)?,
        best_fitness: read_f64(&mut cursor)?,
        average_fitness: read_f64(&mut cursor)?,
        worst_fitness: read_f64(&mut cursor)?,
        best_solution_presentation: read_string(&mut cursor)?,
        state_payload: read_option_string(&mut cursor)?,
        termination_criteria_payload: read_option_string(&mut cursor)?,
        termination_state_payload: read_option_string(&mut cursor)?,
        elapsed_millis: read_option_u64(&mut cursor)?,
        status: byte_to_status(read_u8(&mut cursor)?)?,
        error_message: read_option_string(&mut cursor)?,
    };

    if (cursor.position() as usize) != data.len() {
        return Err(io::Error::new(
            io::ErrorKind::InvalidData,
            ERR_TRAILING_BYTES,
        ));
    }

    Ok(record)
}



/// Loads the latest checkpoint for an algorithm by matching run_id prefix.
///
/// A runtime run id is generated as `<algorithm_name>-<pid>-<timestamp_ms>`.
pub fn latest_checkpoint_record_for_algorithm(
    base_dir: &Path,
    algorithm_name: &str,
) -> io::Result<Option<CheckpointRecord>> {
    let mut best: Option<((u128, String, u64), CheckpointRecord)> = None;

    for path in list_checkpoint_files(base_dir)? {
        let Ok(record) = read_checkpoint_record(&path) else {
            continue;
        };
        if !record.run_id.starts_with(algorithm_name) {
            continue;
        }

        let timestamp_ms = run_id_timestamp_ms(&record.run_id).unwrap_or(0);
        let key = (timestamp_ms, record.run_id.clone(), record.seq_id);

        match &best {
            Some((best_key, _)) if key <= *best_key => {}
            _ => best = Some((key, record)),
        }
    }

    Ok(best.map(|(_, record)| record))
}

/// Lists resumable checkpoints for one algorithm + problem pair ordered oldest->newest.
pub(crate) fn list_resumable_checkpoint_entries_for(
    base_dir: &Path,
    algorithm_name: &str,
    algorithm_signature_hash: u64,
    problem_signature_hash: u64,
) -> io::Result<Vec<CheckpointEntry>> {
    let mut entries: Vec<((u128, String, u64), CheckpointEntry)> = Vec::new();

    for path in list_checkpoint_files(base_dir)? {
        let Ok(record) = read_checkpoint_record(&path) else {
            continue;
        };

        if record.algorithm_name != algorithm_name {
            continue;
        }
        if record.algorithm_signature_hash != algorithm_signature_hash {
            continue;
        }
        if record.problem_signature_hash != problem_signature_hash {
            continue;
        }
        if !is_resumable_status(record.status) {
            continue;
        }

        let timestamp_ms = run_id_timestamp_ms(&record.run_id).unwrap_or(0);
        let key = (timestamp_ms, record.run_id.clone(), record.seq_id);
        entries.push((
            key,
            CheckpointEntry {
                path,
                record,
            },
        ));
    }

    entries.sort_by(|(a, _), (b, _)| a.cmp(b));
    Ok(entries.into_iter().map(|(_, entry)| entry).collect())
}

/// Lists resumable checkpoints for a specific algorithm+problem identity.
///
/// This helper is the high-level entrypoint when you have algorithm/problem
/// metadata and want matching checkpoints quickly.
pub fn list_resumable_checkpoint_entries_for_identity(
    base_dir: &Path,
    algorithm_name: &str,
    algorithm_parameters: &str,
    problem_name: &str,
    problem_parameters: &str,
) -> io::Result<Vec<CheckpointEntry>> {
    let (algorithm_signature_hash, problem_signature_hash) = checkpoint_signature_hashes(
        algorithm_name,
        algorithm_parameters,
        problem_name,
        problem_parameters,
    );

    list_resumable_checkpoint_entries_for(
        base_dir,
        algorithm_name,
        algorithm_signature_hash,
        problem_signature_hash,
    )
}

/// Loads the latest checkpoint compatible with algorithm + problem fingerprint
/// and resumable status (running/failed/interrupted).
#[cfg(test)]
pub(crate) fn latest_resumable_checkpoint_for(
    base_dir: &Path,
    algorithm_signature_hash: u64,
    problem_signature_hash: u64,
) -> io::Result<Option<CheckpointRecord>> {
    let mut best: Option<((u128, String, u64), CheckpointRecord)> = None;

    for path in list_checkpoint_files(base_dir)? {
        let Ok(record) = read_checkpoint_record(&path) else {
            continue;
        };

        if record.algorithm_signature_hash != algorithm_signature_hash {
            continue;
        }
        if record.problem_signature_hash != problem_signature_hash {
            continue;
        }
        if !is_resumable_status(record.status) {
            continue;
        }

        let timestamp_ms = run_id_timestamp_ms(&record.run_id).unwrap_or(0);
        let key = (timestamp_ms, record.run_id.clone(), record.seq_id);

        match &best {
            Some((best_key, _)) if key <= *best_key => {}
            _ => best = Some((key, record)),
        }
    }

    Ok(best.map(|(_, record)| record))
}

/// Removes checkpoints older than the provided UTC epoch milliseconds.
///
/// Returns number of files removed.
pub fn purge_checkpoints_older_than(
    base_dir: &Path,
    older_than_ms: u64,
) -> io::Result<usize> {
    let mut removed = 0usize;
    for path in list_checkpoint_files(base_dir)? {
        let Ok(record) = read_checkpoint_record(&path) else {
            continue;
        };
        if record.created_at_ms < older_than_ms {
            fs::remove_file(&path)?;
            removed += 1;
        }
    }
    Ok(removed)
}

/// Removes checkpoints older than `max_age_ms` relative to current wall-clock time.
///
/// Returns number of files removed.
pub fn purge_checkpoints_older_than_age(
    base_dir: &Path,
    max_age_ms: u64,
) -> io::Result<usize> {
    let now_ms = SystemTime::now()
        .duration_since(UNIX_EPOCH)
        .map(|d| d.as_millis())
        .ok()
        .and_then(|ms| u64::try_from(ms).ok())
        .unwrap_or(0);

    let threshold = now_ms.saturating_sub(max_age_ms);
    purge_checkpoints_older_than(base_dir, threshold)
}

fn is_resumable_status(status: CheckpointRunStatus) -> bool {
    matches!(
        status,
        CheckpointRunStatus::Running
            | CheckpointRunStatus::Failed
            | CheckpointRunStatus::Interrupted
    )
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn write_and_read_checkpoint_record_roundtrip() {
        let base = std::env::temp_dir().join(format!(
            "roma_checkpoint_roundtrip_test_{}",
            std::process::id()
        ));
        fs::create_dir_all(&base).expect("checkpoint test dir should be creatable");

        let record = CheckpointRecord {
            created_at_ms: 123,
            run_id: "HillClimbing-1-123".to_string(),
            algorithm_name: "HillClimbing".to_string(),
            algorithm_parameters: "mutation_probability=0.2".to_string(),
            problem_name: "DemoProblem".to_string(),
            problem_parameters: "items=10".to_string(),
            algorithm_signature_hash: 111,
            problem_signature_hash: 222,
            seq_id: 7,
            iteration: 12,
            evaluations: 44,
            best_fitness: 9.75,
            average_fitness: 8.2,
            worst_fitness: 3.1,
            best_solution_presentation: "selected=2/4".to_string(),
            state_payload: Some("seed=42".to_string()),
            termination_criteria_payload: Some("max_iterations:100".to_string()),
            termination_state_payload: Some(
                "iter=12;eval=44;last_improvement=10;elapsed_ms=50;history=9.75,9.75"
                    .to_string(),
            ),
            elapsed_millis: Some(50),
            status: CheckpointRunStatus::Failed,
            error_message: Some("synthetic error".to_string()),
        };

        let path = checkpoint_file_path(&base, &record.run_id);
        write_checkpoint_record(&path, &record).expect("checkpoint should be writable");

        let loaded = read_checkpoint_record(&path).expect("checkpoint should be readable");
        assert_eq!(loaded, record);
    }

    #[test]
    fn latest_checkpoint_record_for_algorithm_uses_prefix_match() {
        let base = std::env::temp_dir().join(format!(
            "roma_checkpoint_latest_algorithm_test_{}",
            std::process::id()
        ));
        fs::create_dir_all(&base).expect("checkpoint test dir should be creatable");

        let run_id_a = format!("HillClimbing-{}-{}", std::process::id(), 1000);
        let run_id_b = format!("HillClimbing-{}-{}", std::process::id(), 2000);
        let older = CheckpointRecord {
            created_at_ms: 1,
            run_id: run_id_a,
            algorithm_name: "HillClimbing".to_string(),
            algorithm_parameters: "mutation_probability=0.2".to_string(),
            problem_name: "DemoProblem".to_string(),
            problem_parameters: "items=10".to_string(),
            algorithm_signature_hash: 111,
            problem_signature_hash: 222,
            seq_id: 1,
            iteration: 1,
            evaluations: 2,
            best_fitness: 1.0,
            average_fitness: 1.0,
            worst_fitness: 1.0,
            best_solution_presentation: "old".to_string(),
            state_payload: Some("seed=10".to_string()),
            termination_criteria_payload: None,
            termination_state_payload: None,
            elapsed_millis: None,
            status: CheckpointRunStatus::Running,
            error_message: None,
        };
        let newer = CheckpointRecord {
            created_at_ms: 2,
            run_id: run_id_b,
            algorithm_name: "HillClimbing".to_string(),
            algorithm_parameters: "mutation_probability=0.2".to_string(),
            problem_name: "DemoProblem".to_string(),
            problem_parameters: "items=10".to_string(),
            algorithm_signature_hash: 111,
            problem_signature_hash: 222,
            seq_id: 2,
            iteration: 2,
            evaluations: 3,
            best_fitness: 2.0,
            average_fitness: 2.0,
            worst_fitness: 2.0,
            best_solution_presentation: "new".to_string(),
            state_payload: Some("seed=10".to_string()),
            termination_criteria_payload: None,
            termination_state_payload: None,
            elapsed_millis: None,
            status: CheckpointRunStatus::Completed,
            error_message: None,
        };

        let older_path = checkpoint_file_path(&base, &older.run_id);
        let newer_path = checkpoint_file_path(&base, &newer.run_id);
        write_checkpoint_record(&older_path, &older).expect("older checkpoint should be writable");
        std::thread::sleep(std::time::Duration::from_millis(5));
        write_checkpoint_record(&newer_path, &newer).expect("newer checkpoint should be writable");

        let latest = latest_checkpoint_record_for_algorithm(&base, "HillClimbing")
            .expect("latest checkpoint search should work")
            .expect("latest checkpoint should exist");

        assert_eq!(latest.seq_id, 2);
        assert_eq!(latest.best_solution_presentation, "new");
    }

    #[test]
    fn latest_resumable_checkpoint_filters_by_problem_and_status() {
        let base = std::env::temp_dir().join(format!(
            "roma_checkpoint_resumable_test_{}",
            std::process::id()
        ));
        fs::create_dir_all(&base).expect("checkpoint test dir should be creatable");

        let run_a = format!("HillClimbing-{}-{}", std::process::id(), 2000);
        let run_b = format!("HillClimbing-{}-{}", std::process::id(), 3000);

        let completed = CheckpointRecord {
            created_at_ms: 3,
            run_id: run_a,
            algorithm_name: "HillClimbing".to_string(),
            algorithm_parameters: "mutation_probability=0.2".to_string(),
            problem_name: "DemoProblem".to_string(),
            problem_parameters: "dataset=a".to_string(),
            algorithm_signature_hash: 111,
            problem_signature_hash: 333,
            seq_id: 3,
            iteration: 3,
            evaluations: 4,
            best_fitness: 3.0,
            average_fitness: 3.0,
            worst_fitness: 3.0,
            best_solution_presentation: "completed".to_string(),
            state_payload: Some("seed=33".to_string()),
            termination_criteria_payload: None,
            termination_state_payload: None,
            elapsed_millis: None,
            status: CheckpointRunStatus::Completed,
            error_message: None,
        };

        let failed = CheckpointRecord {
            created_at_ms: 4,
            run_id: run_b,
            algorithm_name: "HillClimbing".to_string(),
            algorithm_parameters: "mutation_probability=0.2".to_string(),
            problem_name: "DemoProblem".to_string(),
            problem_parameters: "dataset=a".to_string(),
            algorithm_signature_hash: 111,
            problem_signature_hash: 333,
            seq_id: 8,
            iteration: 8,
            evaluations: 9,
            best_fitness: 8.0,
            average_fitness: 8.0,
            worst_fitness: 8.0,
            best_solution_presentation: "failed".to_string(),
            state_payload: Some("seed=44".to_string()),
            termination_criteria_payload: None,
            termination_state_payload: None,
            elapsed_millis: None,
            status: CheckpointRunStatus::Failed,
            error_message: Some("x".to_string()),
        };

        write_checkpoint_record(
            &checkpoint_file_path(&base, &completed.run_id),
            &completed,
        )
        .expect("completed checkpoint should be writable");
        write_checkpoint_record(
            &checkpoint_file_path(&base, &failed.run_id),
            &failed,
        )
        .expect("failed checkpoint should be writable");

        let latest = latest_resumable_checkpoint_for(&base, 111, 333)
            .expect("latest resumable should be readable")
            .expect("one resumable checkpoint should exist");

        assert_eq!(latest.status, CheckpointRunStatus::Failed);
        assert_eq!(latest.seq_id, 8);
    }

    #[test]
    fn delete_snapshot_on_success_removes_checkpoint_file() {
        let base = std::env::temp_dir().join(format!(
            "roma_checkpoint_delete_test_{}",
            std::process::id()
        ));
        fs::create_dir_all(&base).expect("checkpoint test dir should be creatable");

        let record = CheckpointRecord {
            created_at_ms: 10,
            run_id: "HillClimbing-1-10".to_string(),
            algorithm_name: "HillClimbing".to_string(),
            algorithm_parameters: "mutation_probability=0.2".to_string(),
            problem_name: "DemoProblem".to_string(),
            problem_parameters: "items=10".to_string(),
            algorithm_signature_hash: 111,
            problem_signature_hash: 222,
            seq_id: 1,
            iteration: 1,
            evaluations: 2,
            best_fitness: 1.0,
            average_fitness: 1.0,
            worst_fitness: 1.0,
            best_solution_presentation: "ok".to_string(),
            state_payload: None,
            termination_criteria_payload: None,
            termination_state_payload: None,
            elapsed_millis: Some(10),
            status: CheckpointRunStatus::Completed,
            error_message: None,
        };

        let path = checkpoint_file_path(&base, &record.run_id);
        write_checkpoint_record(&path, &record).expect("checkpoint should be writable");
        assert!(path.exists());

        let removed = delete_snapshot_on_success(&path).expect("delete should succeed");
        assert!(removed);
        assert!(!path.exists());

        let removed_again = delete_snapshot_on_success(&path).expect("missing file is fine");
        assert!(!removed_again);
    }
}
