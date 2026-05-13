use std::fs;
use std::io;
use std::io::Read;
use std::path::{Path, PathBuf};
use std::sync::atomic::{AtomicU64, Ordering};
use std::time::Duration;
use std::time::{SystemTime, UNIX_EPOCH};

use crate::utils::cli::prompt_checkpoint_selection;

use crate::utils::binary::{
    byte_to_status, push_option_string, push_string, push_u64, push_u8, read_option_string,
    read_string, read_u64, read_u8, status_to_byte,
};
use crate::utils::hash::checkpoint_signature_hashes;
use crate::utils::path::{
    checkpoint_file_path, checkpoint_scope_dir, list_checkpoint_files, run_id_timestamp_ms,
};

pub const DEFAULT_FREQUENCY_OF_CHECKPOINT_WRITES: usize = 10;

// Binary file signature used to validate checkpoint file integrity.
const CHECKPOINT_BIN_MAGIC: [u8; 4] = *b"RCKP";
const ERR_INVALID_CHECKPOINT_MAGIC: &str = "invalid checkpoint magic header";
const ERR_TRAILING_BYTES: &str = "trailing bytes found in checkpoint file";
static RUN_ID_SEQUENCE: AtomicU64 = AtomicU64::new(0);

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

/// Shared execution snapshot emitted by algorithms and consumed by
/// termination logic/observers.
#[derive(Clone, Debug)]
pub struct ExecutionStateSnapshot {
    pub iteration: usize,
    pub evaluations: usize,
    /// Cached scalar metric for termination/monitoring.
    pub best_fitness: f64,
    pub average_fitness: f64,
    pub worst_fitness: f64,
    pub best_solution_presentation: String,
}

impl ExecutionStateSnapshot {
    pub fn increment_iteration(&mut self) {
        self.iteration += 1;
    }

    pub fn increment_evaluations(&mut self, count: usize) {
        self.evaluations += count;
    }
}

/// Runtime metadata attached to checkpoint records for one algorithm run.
pub struct CheckpointRuntimeMetadata<'a> {
    pub algorithm_name: &'a str,
    pub algorithm_parameters: &'a str,
    pub problem_description: &'a str,
    pub problem_parameters: &'a str,
    pub algorithm_signature_hash: u64,
    pub problem_signature_hash: u64,
}

impl<'a> CheckpointRuntimeMetadata<'a> {
    pub fn new(
        algorithm_name: &'a str,
        algorithm_parameters: &'a str,
        problem_description: &'a str,
        problem_parameters: &'a str,
    ) -> Self {
        let (algorithm_signature_hash, problem_signature_hash) = checkpoint_signature_hashes(
            algorithm_name,
            algorithm_parameters,
            problem_description,
            problem_parameters,
        );

        Self {
            algorithm_name,
            algorithm_parameters,
            problem_description,
            problem_parameters,
            algorithm_signature_hash,
            problem_signature_hash,
        }
    }
}

pub trait StepStateCheckpoint<T, Q = f64>
where
    T: Clone,
    Q: Clone + Default,
{
    fn random_seed(&self) -> u64;

    fn to_payload(&self) -> String;

    fn from_payload(payload: &str) -> Self;

    fn iteration(&self) -> usize;

    fn evaluations(&self) -> usize;

    fn build_checkpoint_record(
        &self,
        run_id: &str,
        runtime_metadata: &CheckpointRuntimeMetadata<'_>,
        elapsed_millis: Duration,
    ) -> CheckpointRecord {
        CheckpointRecord {
            created_at_ms: SystemTime::now()
                .duration_since(UNIX_EPOCH)
                .map(|d| d.as_millis())
                .ok()
                .and_then(|ms| u64::try_from(ms).ok())
                .unwrap_or(0),
            run_id: run_id.to_string(),
            random_seed: self.random_seed(),
            algorithm_name: runtime_metadata.algorithm_name.to_string(),
            algorithm_parameters: runtime_metadata.algorithm_parameters.to_string(),
            problem_description: runtime_metadata.problem_description.to_string(),
            problem_parameters: runtime_metadata.problem_parameters.to_string(),
            algorithm_signature_hash: runtime_metadata.algorithm_signature_hash,
            problem_signature_hash: runtime_metadata.problem_signature_hash,
            step_state_payload: self.to_payload(),
            seed_payload: None,
            elapsed_millis: elapsed_millis.as_millis() as u64,
            status: CheckpointRunStatus::Running,
            error_message: None,
        }
    }
}

/// Generates a stable run id format used by checkpoint persistence.
pub fn generate_run_id(algorithm_name: &str) -> String {
    let timestamp = SystemTime::now()
        .duration_since(UNIX_EPOCH)
        .map(|d| d.as_millis())
        .ok()
        .and_then(|ms| u64::try_from(ms).ok())
        .unwrap_or(0);
    let sequence = RUN_ID_SEQUENCE.fetch_add(1, Ordering::Relaxed);
    format!(
        "{}-{}-{:020}-{}",
        algorithm_name,
        std::process::id(),
        sequence,
        timestamp
    )
}

#[derive(Debug, Clone, PartialEq)]
pub struct CheckpointRecord {
    pub created_at_ms: u64,
    pub run_id: String,
    pub random_seed: u64,
    pub algorithm_name: String,
    pub algorithm_parameters: String,
    pub problem_description: String,
    pub problem_parameters: String,
    pub algorithm_signature_hash: u64,
    pub problem_signature_hash: u64,
    pub step_state_payload: String,
    pub seed_payload: Option<String>,
    pub elapsed_millis: u64,
    pub status: CheckpointRunStatus,
    pub error_message: Option<String>,
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
/// - problem_description: string
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
/// - current_solution_payload: `Option<String>`
///
/// Optional payloads:
/// - state_payload: `Option<String>` (algorithm-defined UTF-8 payload; e.g. JSON text)
/// - elapsed_millis: `Option<u64>`
/// - status: u8
/// - error_message: `Option<String>`
///
/// Example metadata values:
/// - algorithm_name: `HillClimbing`
/// - algorithm_parameters: `mutation_probability=0.20;termination=max_iterations:100`
/// - problem_description: `roma::problem::implementations::tsp_problem::TspProblem`
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
        &record.problem_description,
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
    push_u64(&mut bytes, record.random_seed);
    push_string(&mut bytes, &record.algorithm_name)?;
    push_string(&mut bytes, &record.algorithm_parameters)?;
    push_string(&mut bytes, &record.problem_description)?;
    push_string(&mut bytes, &record.problem_parameters)?;
    push_u64(&mut bytes, record.algorithm_signature_hash);
    push_u64(&mut bytes, record.problem_signature_hash);
    push_string(&mut bytes, &record.step_state_payload)?;
    push_option_string(&mut bytes, &record.seed_payload)?;
    push_u64(&mut bytes, record.elapsed_millis);
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
        random_seed: read_u64(&mut cursor)?,
        algorithm_name: read_string(&mut cursor)?,
        algorithm_parameters: read_string(&mut cursor)?,
        problem_description: read_string(&mut cursor)?,
        problem_parameters: read_string(&mut cursor)?,
        algorithm_signature_hash: read_u64(&mut cursor)?,
        problem_signature_hash: read_u64(&mut cursor)?,
        step_state_payload: read_string(&mut cursor)?,
        seed_payload: read_option_string(&mut cursor)?,
        elapsed_millis: read_u64(&mut cursor)?,
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

/// Lists resumable checkpoints for one algorithm + problem pair ordered oldest->newest.
pub(crate) fn list_resumable_checkpoint_entries_for(
    base_dir: &Path,
    algorithm_name: &str,
    algorithm_signature_hash: u64,
    problem_signature_hash: u64,
) -> io::Result<Vec<CheckpointEntry>> {
    let mut entries: Vec<((u128, String), CheckpointEntry)> = Vec::new();

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
        let key = (timestamp_ms, record.run_id.clone());
        entries.push((key, CheckpointEntry { path, record }));
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
    problem_description: &str,
    problem_parameters: &str,
) -> io::Result<Vec<CheckpointEntry>> {
    let (algorithm_signature_hash, problem_signature_hash) = checkpoint_signature_hashes(
        algorithm_name,
        algorithm_parameters,
        problem_description,
        problem_parameters,
    );

    list_resumable_checkpoint_entries_for(
        base_dir,
        algorithm_name,
        algorithm_signature_hash,
        problem_signature_hash,
    )
}

/// Selects one resumable checkpoint for a specific algorithm+problem identity.
///
/// Selection behavior:
/// - no matches: returns `Ok(None)`
/// - one match: auto-selects it
/// - multiple matches: prompts user to choose index
pub fn select_resume_checkpoint(
    base_dir: &Path,
    algorithm_name: &str,
    algorithm_parameters: &str,
    problem_description: &str,
    problem_parameters: &str,
) -> Result<Option<CheckpointRecord>, String> {
    let entries = list_resumable_checkpoint_entries_for_identity(
        base_dir,
        algorithm_name,
        algorithm_parameters,
        problem_description,
        problem_parameters,
    )
    .map_err(|err| {
        format!(
            "failed to list resumable checkpoints in '{}': {}",
            base_dir.display(),
            err
        )
    })?;

    if entries.is_empty() {
        return Ok(None);
    }

    let selected_index = if entries.len() == 1 {
        Some(0)
    } else {
        prompt_checkpoint_selection(&entries)?
    };

    Ok(selected_index.map(|index| entries[index].record.clone()))
}

/// Removes checkpoints older than the provided UTC epoch milliseconds.
///
/// Returns number of files removed.
#[allow(dead_code)]
pub fn purge_checkpoints_older_than(base_dir: &Path, older_than_ms: u64) -> io::Result<usize> {
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
#[allow(dead_code)]
pub fn purge_checkpoints_older_than_age(base_dir: &Path, max_age_ms: u64) -> io::Result<usize> {
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

    struct TestCheckpointDir {
        path: std::path::PathBuf,
    }

    impl TestCheckpointDir {
        fn new(label: &str) -> Self {
            let stamp = SystemTime::now()
                .duration_since(UNIX_EPOCH)
                .map(|d| d.as_nanos())
                .unwrap_or(0);

            let path = std::env::temp_dir().join(format!(
                "roma_checkpoint_tests_{}_{}_{}",
                label,
                std::process::id(),
                stamp
            ));

            std::fs::create_dir_all(&path).expect("test checkpoint directory should be created");

            Self { path }
        }

        fn path(&self) -> &std::path::Path {
            &self.path
        }
    }

    impl Drop for TestCheckpointDir {
        fn drop(&mut self) {
            let _ = std::fs::remove_dir_all(&self.path);
        }
    }

    fn build_record(
        run_id: &str,
        algorithm_name: &str,
        algorithm_parameters: &str,
        problem_description: &str,
        problem_parameters: &str,
        status: CheckpointRunStatus,
        step_state_payload: &str,
        created_at_ms: u64,
    ) -> CheckpointRecord {
        let metadata = CheckpointRuntimeMetadata::new(
            algorithm_name,
            algorithm_parameters,
            problem_description,
            problem_parameters,
        );

        CheckpointRecord {
            created_at_ms,
            run_id: run_id.to_string(),
            random_seed: 7,
            algorithm_name: algorithm_name.to_string(),
            algorithm_parameters: algorithm_parameters.to_string(),
            problem_description: problem_description.to_string(),
            problem_parameters: problem_parameters.to_string(),
            algorithm_signature_hash: metadata.algorithm_signature_hash,
            problem_signature_hash: metadata.problem_signature_hash,
            step_state_payload: step_state_payload.to_string(),
            seed_payload: None,
            elapsed_millis: 0,
            status,
            error_message: None,
        }
    }

    #[test]
    fn generate_run_id_is_unique_across_rapid_calls() {
        let mut ids = std::collections::HashSet::new();

        for _ in 0..256 {
            ids.insert(generate_run_id("HillClimbing"));
        }

        assert_eq!(ids.len(), 256);
    }

    #[test]
    fn write_snapshot_overwrites_when_run_id_is_reused_in_same_scope() {
        let dir = TestCheckpointDir::new("overwrite_same_run_id");

        let first = build_record(
            "HillClimbing-42-1000",
            "HillClimbing",
            "seed=11;mutation=0.2",
            "DemoProblem",
            "size=4",
            CheckpointRunStatus::Running,
            "state:one",
            1_000,
        );

        let second = build_record(
            "HillClimbing-42-1000",
            "HillClimbing",
            "seed=11;mutation=0.2",
            "DemoProblem",
            "size=4",
            CheckpointRunStatus::Running,
            "state:two",
            2_000,
        );

        let first_path =
            write_snapshot(dir.path(), &first).expect("first checkpoint should be written");
        let second_path =
            write_snapshot(dir.path(), &second).expect("second checkpoint should be written");

        assert_eq!(first_path, second_path);

        let files = list_checkpoint_files(dir.path()).expect("checkpoint files should be listed");
        assert_eq!(files.len(), 1);

        let stored = read_snapshot(&second_path).expect("stored checkpoint should be readable");
        assert_eq!(stored.step_state_payload, "state:two");
        assert_eq!(stored.created_at_ms, 2_000);
    }

    #[test]
    fn write_snapshot_keeps_distinct_files_for_different_run_ids_in_same_scope() {
        let dir = TestCheckpointDir::new("distinct_run_ids_same_scope");

        let first = build_record(
            "HillClimbing-42-1000",
            "HillClimbing",
            "seed=11;mutation=0.2",
            "DemoProblem",
            "size=4",
            CheckpointRunStatus::Running,
            "state:one",
            1_000,
        );

        let second = build_record(
            "HillClimbing-42-1001",
            "HillClimbing",
            "seed=11;mutation=0.2",
            "DemoProblem",
            "size=4",
            CheckpointRunStatus::Running,
            "state:two",
            1_001,
        );

        let first_path =
            write_snapshot(dir.path(), &first).expect("first checkpoint should be written");
        let second_path =
            write_snapshot(dir.path(), &second).expect("second checkpoint should be written");

        assert_ne!(first_path, second_path);

        let files = list_checkpoint_files(dir.path()).expect("checkpoint files should be listed");
        assert_eq!(files.len(), 2);
    }

    #[test]
    fn write_snapshot_keeps_distinct_files_when_signature_hashes_change() {
        let dir = TestCheckpointDir::new("same_run_id_different_hash");

        let first = build_record(
            "HillClimbing-42-1000",
            "HillClimbing",
            "seed=11;mutation=0.2",
            "DemoProblem",
            "size=4",
            CheckpointRunStatus::Running,
            "state:one",
            1_000,
        );

        let second = build_record(
            "HillClimbing-42-1000",
            "HillClimbing",
            "seed=99;mutation=0.2",
            "DemoProblem",
            "size=4",
            CheckpointRunStatus::Running,
            "state:two",
            1_001,
        );

        let first_path =
            write_snapshot(dir.path(), &first).expect("first checkpoint should be written");
        let second_path =
            write_snapshot(dir.path(), &second).expect("second checkpoint should be written");

        assert_ne!(first_path, second_path);

        let files = list_checkpoint_files(dir.path()).expect("checkpoint files should be listed");
        assert_eq!(files.len(), 2);
    }

    #[test]
    fn list_resumable_entries_filters_by_identity_and_status() {
        let dir = TestCheckpointDir::new("list_filters_identity_status");

        let matching_running = build_record(
            "HillClimbing-42-1000",
            "HillClimbing",
            "seed=11;mutation=0.2",
            "DemoProblem",
            "size=4",
            CheckpointRunStatus::Running,
            "state:running",
            1_000,
        );

        let matching_failed = build_record(
            "HillClimbing-42-1001",
            "HillClimbing",
            "seed=11;mutation=0.2",
            "DemoProblem",
            "size=4",
            CheckpointRunStatus::Failed,
            "state:failed",
            1_001,
        );

        let different_algorithm_params = build_record(
            "HillClimbing-42-1002",
            "HillClimbing",
            "seed=77;mutation=0.2",
            "DemoProblem",
            "size=4",
            CheckpointRunStatus::Running,
            "state:other-algorithm-params",
            1_002,
        );

        let different_problem_params = build_record(
            "HillClimbing-42-1003",
            "HillClimbing",
            "seed=11;mutation=0.2",
            "DemoProblem",
            "size=8",
            CheckpointRunStatus::Running,
            "state:other-problem-params",
            1_003,
        );

        let matching_completed = build_record(
            "HillClimbing-42-1004",
            "HillClimbing",
            "seed=11;mutation=0.2",
            "DemoProblem",
            "size=4",
            CheckpointRunStatus::Completed,
            "state:completed",
            1_004,
        );

        for record in [
            &matching_running,
            &matching_failed,
            &different_algorithm_params,
            &different_problem_params,
            &matching_completed,
        ] {
            write_snapshot(dir.path(), record).expect("checkpoint should be written");
        }

        let entries = list_resumable_checkpoint_entries_for_identity(
            dir.path(),
            "HillClimbing",
            "seed=11;mutation=0.2",
            "DemoProblem",
            "size=4",
        )
        .expect("entries should be listed");

        let run_ids: Vec<&str> = entries
            .iter()
            .map(|entry| entry.record.run_id.as_str())
            .collect();

        assert_eq!(
            run_ids,
            vec!["HillClimbing-42-1000", "HillClimbing-42-1001"]
        );
        assert!(entries.iter().all(|entry| {
            matches!(
                entry.record.status,
                CheckpointRunStatus::Running
                    | CheckpointRunStatus::Failed
                    | CheckpointRunStatus::Interrupted
            )
        }));
    }

    #[test]
    fn select_resume_checkpoint_returns_single_matching_record() {
        let dir = TestCheckpointDir::new("select_single_checkpoint");

        let matching = build_record(
            "HillClimbing-42-1000",
            "HillClimbing",
            "seed=11;mutation=0.2",
            "DemoProblem",
            "size=4",
            CheckpointRunStatus::Running,
            "state:matching",
            1_000,
        );

        let different_identity = build_record(
            "HillClimbing-42-1001",
            "HillClimbing",
            "seed=77;mutation=0.2",
            "DemoProblem",
            "size=4",
            CheckpointRunStatus::Running,
            "state:non-matching",
            1_001,
        );

        write_snapshot(dir.path(), &matching).expect("matching checkpoint should be written");
        write_snapshot(dir.path(), &different_identity)
            .expect("non matching checkpoint should be written");

        let selected = select_resume_checkpoint(
            dir.path(),
            "HillClimbing",
            "seed=11;mutation=0.2",
            "DemoProblem",
            "size=4",
        )
        .expect("selection should not fail")
        .expect("one checkpoint should be auto-selected");

        assert_eq!(selected.run_id, matching.run_id);
        assert_eq!(selected.step_state_payload, "state:matching");
    }
}
