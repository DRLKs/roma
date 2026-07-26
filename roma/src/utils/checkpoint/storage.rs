use std::fs;
use std::io;
use std::io::Read;
use std::path::{Path, PathBuf};
use std::time::{SystemTime, UNIX_EPOCH};

use super::selection::select_checkpoint_record;
use super::{
    CHECKPOINT_BIN_MAGIC, CheckpointEntry, CheckpointRecord, CheckpointRunStatus,
    CheckpointRuntimeMetadata, ERR_INVALID_CHECKPOINT_MAGIC, ERR_TRAILING_BYTES,
    validate_step_state_payload,
};
use crate::utils::binary::{
    byte_to_status, push_bytes, push_option_bytes, push_option_string, push_string, push_u8,
    push_u64, read_bytes, read_option_bytes, read_option_string, read_string, read_u8, read_u64,
    status_to_byte,
};
use crate::utils::path::{
    checkpoint_file_path, checkpoint_scope_dir, list_checkpoint_files, run_id_timestamp_ms,
};
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
/// Note: the file format is always binary. Text metadata is length-prefixed
/// UTF-8; algorithm step state is a length-prefixed byte payload.
pub(crate) fn write_checkpoint_record(path: &Path, record: &CheckpointRecord) -> io::Result<()> {
    validate_step_state_payload(&record.step_state_payload)?;

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
    push_bytes(&mut bytes, &record.step_state_payload)?;
    push_option_bytes(&mut bytes, &record.seed_payload)?;
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

    let created_at_ms = read_u64(&mut cursor)?;
    let run_id = read_string(&mut cursor)?;
    let random_seed = read_u64(&mut cursor)?;
    let algorithm_name = read_string(&mut cursor)?;
    let algorithm_parameters = read_string(&mut cursor)?;
    let problem_description = read_string(&mut cursor)?;
    let problem_parameters = read_string(&mut cursor)?;
    let algorithm_signature_hash = read_u64(&mut cursor)?;
    let problem_signature_hash = read_u64(&mut cursor)?;
    let step_state_payload = read_bytes(&mut cursor)?;
    validate_step_state_payload(&step_state_payload)?;

    let record = CheckpointRecord {
        created_at_ms,
        run_id,
        random_seed,
        algorithm_name,
        algorithm_parameters,
        problem_description,
        problem_parameters,
        algorithm_signature_hash,
        problem_signature_hash,
        step_state_payload,
        seed_payload: read_option_bytes(&mut cursor)?,
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

/// Lists resumable checkpoints for precomputed runtime metadata.
pub fn list_resumable_checkpoint_entries_for_metadata(
    base_dir: &Path,
    runtime_metadata: &CheckpointRuntimeMetadata<'_>,
) -> io::Result<Vec<CheckpointEntry>> {
    list_resumable_checkpoint_entries_for(
        base_dir,
        runtime_metadata.algorithm_name,
        runtime_metadata.algorithm_signature_hash,
        runtime_metadata.problem_signature_hash,
    )
}

/// Selects one resumable checkpoint from precomputed runtime metadata.
pub fn select_resume_checkpoint_for_metadata(
    base_dir: &Path,
    runtime_metadata: &CheckpointRuntimeMetadata<'_>,
) -> Result<Option<CheckpointRecord>, String> {
    let entries = list_resumable_checkpoint_entries_for_metadata(base_dir, runtime_metadata)
        .map_err(|err| {
            format!(
                "failed to list resumable checkpoints in '{}': {}",
                base_dir.display(),
                err
            )
        })?;

    select_checkpoint_record(entries)
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
