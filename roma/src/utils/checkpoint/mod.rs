use std::io;
use std::path::PathBuf;
use std::str::FromStr;
use std::sync::atomic::{AtomicU64, Ordering};
use std::time::Duration;
use std::time::{SystemTime, UNIX_EPOCH};

use crate::solution::Solution;
use crate::utils::binary::{
    push_f64, push_string, push_u8, push_u64, push_usize, read_f64, read_string, read_u8, read_u64,
    read_usize,
};
use crate::utils::hash::checkpoint_signature_hashes;

mod policy;
mod selection;

#[cfg(test)]
use crate::utils::cli::CliArgs;
#[cfg(test)]
use crate::utils::path::list_checkpoint_files;
#[cfg(test)]
use policy::resolve_checkpoint_dir_from_config_for_writes;
pub use policy::{CheckpointPolicy, DEFAULT_FREQUENCY_OF_CHECKPOINT_WRITES};

// Binary file signature used to validate checkpoint file integrity.
const CHECKPOINT_BIN_MAGIC: [u8; 4] = *b"RCKP";
const STATE_PAYLOAD_VERSION: u8 = 1;
const STATE_PAYLOAD_ABSENT: u8 = 0;
const STATE_PAYLOAD_PRESENT: u8 = 1;
const ERR_INVALID_CHECKPOINT_MAGIC: &str = "invalid checkpoint magic header";
const ERR_INVALID_STATE_PAYLOAD_VERSION: &str = "invalid checkpoint state payload version";
const ERR_INVALID_STATE_PAYLOAD_FLAG: &str = "invalid checkpoint state payload flag";
const ERR_INVALID_CHECKPOINT_ATOM: &str = "invalid typed value in checkpoint state payload";
const ERR_TRAILING_BYTES_IN_STATE_PAYLOAD: &str =
    "trailing bytes found in checkpoint state payload";
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

fn invalid_payload_data(message: impl Into<String>) -> io::Error {
    io::Error::new(io::ErrorKind::InvalidData, message.into())
}

fn validate_step_state_payload(payload: &[u8]) -> io::Result<()> {
    if payload.first().copied() == Some(STATE_PAYLOAD_VERSION) {
        Ok(())
    } else {
        Err(invalid_payload_data(ERR_INVALID_STATE_PAYLOAD_VERSION))
    }
}

/// Length-prefixed binary writer for algorithm-defined checkpoint state.
///
/// The payload is versioned and field-order based. Primitive fields are written
/// in little-endian form; generic solution variables/quality payloads are stored
/// as length-prefixed UTF-8 atoms because Roma allows user-defined `T`/`Q`.
pub(crate) struct StatePayloadEncoder {
    bytes: Vec<u8>,
}

impl StatePayloadEncoder {
    pub(crate) fn new() -> Self {
        let mut bytes = Vec::new();
        push_u8(&mut bytes, STATE_PAYLOAD_VERSION);
        Self { bytes }
    }

    pub(crate) fn finish(self) -> Vec<u8> {
        self.bytes
    }

    pub(crate) fn write_u64(&mut self, value: u64) {
        push_u64(&mut self.bytes, value);
    }

    pub(crate) fn write_usize(&mut self, value: usize) -> io::Result<()> {
        push_usize(&mut self.bytes, value)
    }

    pub(crate) fn write_f64(&mut self, value: f64) {
        push_f64(&mut self.bytes, value);
    }

    pub(crate) fn write_string(&mut self, value: &str) -> io::Result<()> {
        push_string(&mut self.bytes, value)
    }

    pub(crate) fn write_solution<T, Q>(&mut self, solution: &Solution<T, Q>) -> io::Result<()>
    where
        T: Clone + Display,
        Q: Clone + Display,
    {
        self.write_usize(solution.num_variables())?;
        for variable in solution.variables() {
            self.write_string(&variable.to_string())?;
        }

        match solution.quality() {
            Some(quality) => {
                push_u8(&mut self.bytes, STATE_PAYLOAD_PRESENT);
                self.write_string(&quality.to_string())?;
            }
            None => push_u8(&mut self.bytes, STATE_PAYLOAD_ABSENT),
        }

        Ok(())
    }

    pub(crate) fn write_solution_vec<T, Q>(
        &mut self,
        solutions: &[Solution<T, Q>],
    ) -> io::Result<()>
    where
        T: Clone + Display,
        Q: Clone + Display,
    {
        self.write_usize(solutions.len())?;
        for solution in solutions {
            self.write_solution(solution)?;
        }
        Ok(())
    }

    pub(crate) fn write_f64_vec(&mut self, values: &[f64]) -> io::Result<()> {
        self.write_usize(values.len())?;
        for value in values {
            self.write_f64(*value);
        }
        Ok(())
    }

    pub(crate) fn write_f64_vec_vec(&mut self, values: &[Vec<f64>]) -> io::Result<()> {
        self.write_usize(values.len())?;
        for inner in values {
            self.write_f64_vec(inner)?;
        }
        Ok(())
    }

    pub(crate) fn write_string_usize_map(
        &mut self,
        values: &HashMap<String, usize>,
    ) -> io::Result<()> {
        let mut entries: Vec<_> = values.iter().collect();
        entries.sort_by(|left, right| left.0.cmp(right.0));

        self.write_usize(entries.len())?;
        for (key, value) in entries {
            self.write_string(key)?;
            self.write_usize(*value)?;
        }
        Ok(())
    }
}

/// Length-prefixed binary reader for algorithm-defined checkpoint state.
pub(crate) struct StatePayloadDecoder<'a> {
    cursor: io::Cursor<&'a [u8]>,
}

impl<'a> StatePayloadDecoder<'a> {
    pub(crate) fn new(payload: &'a [u8]) -> io::Result<Self> {
        let mut cursor = io::Cursor::new(payload);
        let version = read_u8(&mut cursor)?;
        if version != STATE_PAYLOAD_VERSION {
            return Err(invalid_payload_data(ERR_INVALID_STATE_PAYLOAD_VERSION));
        }
        Ok(Self { cursor })
    }

    pub(crate) fn read_u64(&mut self) -> io::Result<u64> {
        read_u64(&mut self.cursor)
    }

    pub(crate) fn read_usize(&mut self) -> io::Result<usize> {
        read_usize(&mut self.cursor)
    }

    pub(crate) fn read_f64(&mut self) -> io::Result<f64> {
        read_f64(&mut self.cursor)
    }

    pub(crate) fn read_string(&mut self) -> io::Result<String> {
        read_string(&mut self.cursor)
    }

    pub(crate) fn read_solution<T, Q>(&mut self) -> io::Result<Solution<T, Q>>
    where
        T: Clone + Display + FromStr,
        Q: Clone + Display + FromStr,
    {
        let variable_count = self.read_usize()?;
        let mut variables = Vec::with_capacity(variable_count);
        for _ in 0..variable_count {
            let raw = self.read_string()?;
            variables.push(
                raw.parse::<T>()
                    .map_err(|_| invalid_payload_data(ERR_INVALID_CHECKPOINT_ATOM))?,
            );
        }

        let mut solution = Solution::new(variables);
        match read_u8(&mut self.cursor)? {
            STATE_PAYLOAD_ABSENT => {}
            STATE_PAYLOAD_PRESENT => {
                let raw = self.read_string()?;
                let quality = raw
                    .parse::<Q>()
                    .map_err(|_| invalid_payload_data(ERR_INVALID_CHECKPOINT_ATOM))?;
                solution.set_quality(quality);
            }
            _ => return Err(invalid_payload_data(ERR_INVALID_STATE_PAYLOAD_FLAG)),
        }

        Ok(solution)
    }

    pub(crate) fn read_solution_vec<T, Q>(&mut self) -> io::Result<Vec<Solution<T, Q>>>
    where
        T: Clone + Display + FromStr,
        Q: Clone + Display + FromStr,
    {
        let count = self.read_usize()?;
        let mut solutions = Vec::with_capacity(count);
        for _ in 0..count {
            solutions.push(self.read_solution()?);
        }
        Ok(solutions)
    }

    pub(crate) fn read_f64_vec(&mut self) -> io::Result<Vec<f64>> {
        let count = self.read_usize()?;
        let mut values = Vec::with_capacity(count);
        for _ in 0..count {
            values.push(self.read_f64()?);
        }
        Ok(values)
    }

    pub(crate) fn read_f64_vec_vec(&mut self) -> io::Result<Vec<Vec<f64>>> {
        let count = self.read_usize()?;
        let mut values = Vec::with_capacity(count);
        for _ in 0..count {
            values.push(self.read_f64_vec()?);
        }
        Ok(values)
    }

    pub(crate) fn read_string_usize_map(&mut self) -> io::Result<HashMap<String, usize>> {
        let count = self.read_usize()?;
        let mut values = HashMap::with_capacity(count);
        for _ in 0..count {
            let key = self.read_string()?;
            let value = self.read_usize()?;
            values.insert(key, value);
        }
        Ok(values)
    }

    pub(crate) fn ensure_finished(&self) -> io::Result<()> {
        if (self.cursor.position() as usize) == self.cursor.get_ref().len() {
            Ok(())
        } else {
            Err(invalid_payload_data(ERR_TRAILING_BYTES_IN_STATE_PAYLOAD))
        }
    }
}

fn decode_state_progress(payload: &[u8]) -> Option<(usize, usize)> {
    let mut payload = StatePayloadDecoder::new(payload).ok()?;
    let iteration = payload.read_usize().ok()?;
    let evaluations = payload.read_usize().ok()?;
    Some((iteration, evaluations))
}

pub trait StepStateCheckpoint<T, Q = f64>
where
    T: Clone,
    Q: Clone + Default,
{
    fn random_seed(&self) -> u64;

    fn to_payload(&self) -> Vec<u8>;

    fn from_payload(payload: &[u8]) -> Self;

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
    pub step_state_payload: Vec<u8>,
    pub seed_payload: Option<Vec<u8>>,
    pub elapsed_millis: u64,
    pub status: CheckpointRunStatus,
    pub error_message: Option<String>,
}

impl CheckpointRecord {
    pub(crate) fn state_progress_summary(&self) -> String {
        decode_state_progress(&self.step_state_payload)
            .map(|(iteration, evaluations)| {
                format!(
                    "iter={}, eval={}, seed={}",
                    iteration, evaluations, self.random_seed
                )
            })
            .unwrap_or_else(|| {
                format!(
                    "state={} bytes, seed={}",
                    self.step_state_payload.len(),
                    self.random_seed
                )
            })
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
/// - problem_description: string
/// - problem_parameters: string
/// - algorithm_signature_hash: u64
/// - problem_signature_hash: u64
///
/// State and optional payloads:
/// - step_state_payload: bytes, with an algorithm-owned typed binary layout
/// - seed_payload: `Option<bytes>`
/// - elapsed_millis: u64
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
mod storage;

pub use storage::{
    delete_snapshot_on_success, list_resumable_checkpoint_entries_for_metadata,
    purge_checkpoints_older_than, purge_checkpoints_older_than_age, read_snapshot,
    select_resume_checkpoint_for_metadata, write_snapshot,
};
#[allow(unused_imports)]
pub(crate) use storage::{
    list_resumable_checkpoint_entries_for, read_checkpoint_record, write_checkpoint_record,
    write_execution_checkpoint,
};

#[cfg(test)]
mod tests {
    use super::*;
    use crate::utils::path::CheckpointPathConfig;

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
        step_state_payload: &[u8],
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
            step_state_payload: step_state_payload.to_vec(),
            seed_payload: None,
            elapsed_millis: 0,
            status,
            error_message: None,
        }
    }

    fn sample_step_payload(marker: usize) -> Vec<u8> {
        let mut encoder = StatePayloadEncoder::new();
        encoder
            .write_usize(marker)
            .expect("sample payload iteration should serialize");
        encoder
            .write_usize(marker * 10)
            .expect("sample payload evaluations should serialize");
        encoder.finish()
    }

    #[test]
    fn state_payload_codec_roundtrips_length_prefixed_solution_fields() {
        let mut original: Solution<String> =
            Solution::new(vec!["left,comma".to_string(), "right|pipe".to_string()]);
        original.set_quality(17.25);

        let mut encoder = StatePayloadEncoder::new();
        encoder.write_usize(42).expect("iteration should serialize");
        encoder
            .write_solution(&original)
            .expect("solution should serialize");

        let encoded = encoder.finish();
        let mut decoder = StatePayloadDecoder::new(&encoded).expect("payload should start");
        let iteration = decoder.read_usize().expect("iteration should deserialize");
        let restored: Solution<String> = decoder
            .read_solution()
            .expect("solution should deserialize");
        decoder
            .ensure_finished()
            .expect("payload should have no trailing bytes");

        assert_eq!(iteration, 42);
        let expected_variables = vec!["left,comma".to_string(), "right|pipe".to_string()];
        assert_eq!(restored.variables(), expected_variables.as_slice());
        assert_eq!(restored.quality().copied(), Some(17.25));
    }

    #[test]
    fn state_payload_decoder_rejects_trailing_bytes() {
        let mut encoder = StatePayloadEncoder::new();
        encoder.write_usize(7).expect("value should serialize");
        let mut encoded = encoder.finish();
        encoded.push(255);

        let mut decoder = StatePayloadDecoder::new(&encoded).expect("payload should start");
        assert_eq!(decoder.read_usize().expect("value should deserialize"), 7);

        let error = decoder
            .ensure_finished()
            .expect_err("trailing payload bytes should fail");

        assert!(
            error
                .to_string()
                .contains("trailing bytes found in checkpoint state payload")
        );
    }

    #[test]
    fn checkpoint_record_summary_reads_progress_prefix() {
        let record = build_record(
            "HillClimbing-42-1000",
            "HillClimbing",
            "seed=11;mutation=0.2",
            "DemoProblem",
            "size=4",
            CheckpointRunStatus::Running,
            &sample_step_payload(12),
            1_000,
        );

        assert_eq!(record.state_progress_summary(), "iter=12, eval=120, seed=7");
    }

    #[test]
    fn write_snapshot_rejects_unversioned_step_payload() {
        let dir = TestCheckpointDir::new("reject_unversioned_payload");
        let record = build_record(
            "HillClimbing-42-1000",
            "HillClimbing",
            "seed=11;mutation=0.2",
            "DemoProblem",
            "size=4",
            CheckpointRunStatus::Running,
            b"iter=1;eval=0",
            1_000,
        );

        let error = write_snapshot(dir.path(), &record)
            .expect_err("unversioned state payload should be rejected");

        assert!(
            error
                .to_string()
                .contains("invalid checkpoint state payload version")
        );
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
            &sample_step_payload(1),
            1_000,
        );

        let second = build_record(
            "HillClimbing-42-1000",
            "HillClimbing",
            "seed=11;mutation=0.2",
            "DemoProblem",
            "size=4",
            CheckpointRunStatus::Running,
            &sample_step_payload(2),
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
        assert_eq!(stored.step_state_payload, sample_step_payload(2));
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
            &sample_step_payload(1),
            1_000,
        );

        let second = build_record(
            "HillClimbing-42-1001",
            "HillClimbing",
            "seed=11;mutation=0.2",
            "DemoProblem",
            "size=4",
            CheckpointRunStatus::Running,
            &sample_step_payload(2),
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
            &sample_step_payload(1),
            1_000,
        );

        let second = build_record(
            "HillClimbing-42-1000",
            "HillClimbing",
            "seed=99;mutation=0.2",
            "DemoProblem",
            "size=4",
            CheckpointRunStatus::Running,
            &sample_step_payload(2),
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
            &sample_step_payload(1),
            1_000,
        );

        let matching_failed = build_record(
            "HillClimbing-42-1001",
            "HillClimbing",
            "seed=11;mutation=0.2",
            "DemoProblem",
            "size=4",
            CheckpointRunStatus::Failed,
            &sample_step_payload(2),
            1_001,
        );

        let different_algorithm_params = build_record(
            "HillClimbing-42-1002",
            "HillClimbing",
            "seed=77;mutation=0.2",
            "DemoProblem",
            "size=4",
            CheckpointRunStatus::Running,
            &sample_step_payload(3),
            1_002,
        );

        let different_problem_params = build_record(
            "HillClimbing-42-1003",
            "HillClimbing",
            "seed=11;mutation=0.2",
            "DemoProblem",
            "size=8",
            CheckpointRunStatus::Running,
            &sample_step_payload(4),
            1_003,
        );

        let matching_completed = build_record(
            "HillClimbing-42-1004",
            "HillClimbing",
            "seed=11;mutation=0.2",
            "DemoProblem",
            "size=4",
            CheckpointRunStatus::Completed,
            &sample_step_payload(5),
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

        let metadata = CheckpointRuntimeMetadata::new(
            "HillClimbing",
            "seed=11;mutation=0.2",
            "DemoProblem",
            "size=4",
        );

        let entries = list_resumable_checkpoint_entries_for_metadata(dir.path(), &metadata)
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
            &sample_step_payload(1),
            1_000,
        );

        let different_identity = build_record(
            "HillClimbing-42-1001",
            "HillClimbing",
            "seed=77;mutation=0.2",
            "DemoProblem",
            "size=4",
            CheckpointRunStatus::Running,
            &sample_step_payload(2),
            1_001,
        );

        write_snapshot(dir.path(), &matching).expect("matching checkpoint should be written");
        write_snapshot(dir.path(), &different_identity)
            .expect("non matching checkpoint should be written");

        let metadata = CheckpointRuntimeMetadata::new(
            "HillClimbing",
            "seed=11;mutation=0.2",
            "DemoProblem",
            "size=4",
        );

        let selected = select_resume_checkpoint_for_metadata(dir.path(), &metadata)
            .expect("selection should not fail")
            .expect("one checkpoint should be auto-selected");

        assert_eq!(selected.run_id, matching.run_id);
        assert_eq!(selected.step_state_payload, sample_step_payload(1));
    }

    #[test]
    fn checkpoint_policy_distinguishes_requested_writes_from_storage_availability() {
        let base = std::env::temp_dir().join(format!(
            "roma_checkpoint_policy_test_{}_{}",
            std::process::id(),
            SystemTime::now()
                .duration_since(UNIX_EPOCH)
                .map(|d| d.as_nanos())
                .unwrap_or(0)
        ));
        let explicit_dir = base.join("checkpoints");
        let config = CheckpointPathConfig::default();

        let writes_disabled = CheckpointPolicy::resolve(
            &CliArgs::from_iter(vec![
                crate::utils::cli::CLI_FLAG_CHECKPOINT_DIR.to_string(),
                explicit_dir.to_string_lossy().into_owned(),
                crate::utils::cli::CLI_FLAG_NO_CHECKPOINT.to_string(),
            ]),
            &config,
        );

        assert!(writes_disabled.storage_ready);
        assert!(!writes_disabled.writes_requested);
        assert!(!writes_disabled.should_write());

        let writes_enabled = CheckpointPolicy::resolve(
            &CliArgs::from_iter(vec![
                crate::utils::cli::CLI_FLAG_CHECKPOINT_DIR.to_string(),
                explicit_dir.to_string_lossy().into_owned(),
                crate::utils::cli::CLI_FLAG_RESUME.to_string(),
            ]),
            &config,
        );

        assert!(writes_enabled.storage_ready);
        assert!(writes_enabled.writes_requested);
        assert!(writes_enabled.should_write());
        assert!(writes_enabled.resume_requested());

        let _ = std::fs::remove_dir_all(base);
    }

    #[test]
    fn checkpoint_policy_uses_initialized_default_directory() {
        let base = std::env::temp_dir().join(format!(
            "roma_checkpoint_policy_default_test_{}_{}",
            std::process::id(),
            SystemTime::now()
                .duration_since(UNIX_EPOCH)
                .map(|d| d.as_nanos())
                .unwrap_or(0)
        ));
        let fallback = base.join("fallback");
        let blocked_path = base.join("blocked");
        std::fs::create_dir_all(&base).expect("test base directory should be created");
        std::fs::write(&blocked_path, b"not a directory")
            .expect("test blocking file should be created");

        let config = CheckpointPathConfig {
            app_name: blocked_path.to_string_lossy().into_owned(),
            env_var_name: "ROMA_TEST_UNUSED_CHECKPOINT_ENV",
            explicit_dir: Some(blocked_path.join("roma")),
            project_fallback_dir: Some(fallback.clone()),
        };

        let policy = CheckpointPolicy::resolve(&CliArgs::from_iter(Vec::<String>::new()), &config);

        assert!(policy.should_write());
        assert_eq!(policy.checkpoint_dir(), fallback.as_path());

        let _ = std::fs::remove_dir_all(base);
    }

    #[test]
    fn resolve_checkpoint_dir_for_writes_skips_unwritable_candidates() {
        let base = std::env::temp_dir().join(format!(
            "roma_checkpoint_dir_test_{}_{}",
            std::process::id(),
            SystemTime::now()
                .duration_since(UNIX_EPOCH)
                .map(|d| d.as_nanos())
                .unwrap_or(0)
        ));
        let fallback = base.join("fallback");
        let blocked_path = base.join("blocked");
        std::fs::create_dir_all(&base).expect("test base directory should be created");
        std::fs::write(&blocked_path, b"not a directory")
            .expect("test blocking file should be created");

        let config = CheckpointPathConfig {
            app_name: blocked_path.to_string_lossy().into_owned(),
            env_var_name: "ROMA_TEST_UNUSED_CHECKPOINT_ENV",
            explicit_dir: Some(blocked_path.join("roma")),
            project_fallback_dir: Some(fallback.clone()),
        };

        let resolved = resolve_checkpoint_dir_from_config_for_writes(&config)
            .expect("a writable fallback directory should be resolved");

        assert_eq!(resolved, fallback);
        assert!(resolved.exists());

        let _ = std::fs::remove_dir_all(base);
    }
}
use std::collections::HashMap;
use std::fmt::Display;
