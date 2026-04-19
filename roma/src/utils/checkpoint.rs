use std::env;
use std::fs;
use std::io;
use std::io::Read;
use std::path::{Path, PathBuf};
use std::time::{SystemTime, UNIX_EPOCH};

pub const DEFAULT_CHECKPOINT_ENV_VAR: &str = "ROMA_CHECKPOINT_DIR";
pub const DEFAULT_APP_NAME: &str = "roma";
pub const DEFAULT_FREQUENCY_OF_CHECKPOINT_WRITES: usize = 10;

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum CheckpointDirSource {
    Explicit,
    EnvVar,
    OsDefault,
    ProjectFallback,
    LocalFallback,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum CheckpointInitMode {
    /// Any initialization error is returned to caller.
    Required,
    /// Initialization errors are tolerated; caller may skip checkpointing.
    BestEffort,
}

#[derive(Debug, Clone)]
pub struct CheckpointInitResult {
    pub directory: Option<PathBuf>,
    pub source: Option<CheckpointDirSource>,
    pub attempts: Vec<(CheckpointDirSource, PathBuf, io::ErrorKind)>,
}

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

#[derive(Debug, Clone)]
pub struct CheckpointEntry {
    pub path: PathBuf,
    pub record: CheckpointRecord,
}

const CHECKPOINT_BIN_MAGIC: [u8; 4] = *b"RCKP";

/// Deterministic 64-bit FNV-1a hash used to build checkpoint signatures.
pub(crate) fn stable_hash64(text: &str) -> u64 {
    let mut hash: u64 = 0xcbf29ce484222325;
    for byte in text.as_bytes() {
        hash ^= u64::from(*byte);
        hash = hash.wrapping_mul(0x100000001b3);
    }
    hash
}

/// Computes algorithm/problem signature hashes from their names and parameters.
pub(crate) fn checkpoint_signature_hashes(
    algorithm_name: &str,
    algorithm_parameters: &str,
    problem_name: &str,
    problem_parameters: &str,
) -> (u64, u64) {
    let algorithm_signature_hash =
        stable_hash64(&format!("{}|{}", algorithm_name, algorithm_parameters));
    let problem_signature_hash = stable_hash64(&format!("{}|{}", problem_name, problem_parameters));
    (algorithm_signature_hash, problem_signature_hash)
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

/// Configures how checkpoint directories are resolved.
///
/// Priority order:
/// 1. `explicit_dir`
/// 2. env var named by `env_var_name`
/// 3. OS-specific default location
/// 4. `project_fallback_dir`
/// 5. `./.roma/checkpoints`
#[derive(Debug, Clone)]
pub struct CheckpointPathConfig {
    pub app_name: String,
    pub env_var_name: &'static str,
    pub explicit_dir: Option<PathBuf>,
    pub project_fallback_dir: Option<PathBuf>,
}

impl Default for CheckpointPathConfig {
    fn default() -> Self {
        Self {
            app_name: DEFAULT_APP_NAME.to_string(),
            env_var_name: DEFAULT_CHECKPOINT_ENV_VAR,
            explicit_dir: None,
            project_fallback_dir: None,
        }
    }
}

impl CheckpointPathConfig {
    pub fn with_app_name(mut self, app_name: impl Into<String>) -> Self {
        self.app_name = app_name.into();
        self
    }

    pub fn with_explicit_dir(mut self, dir: impl Into<PathBuf>) -> Self {
        self.explicit_dir = Some(dir.into());
        self
    }

    pub fn with_project_fallback_dir(mut self, dir: impl Into<PathBuf>) -> Self {
        self.project_fallback_dir = Some(dir.into());
        self
    }
}

/// Resolves the checkpoint directory following `CheckpointPathConfig` priority.
pub fn resolve_checkpoint_dir(config: &CheckpointPathConfig) -> PathBuf {
    checkpoint_dir_candidates(config)
        .into_iter()
        .next()
        .map(|(_, path)| path)
        .unwrap_or_else(local_fallback_dir)
}

/// Resolves and creates the checkpoint directory if missing.
pub fn ensure_checkpoint_dir(config: &CheckpointPathConfig) -> io::Result<PathBuf> {
    let prepared = initialize_checkpoint_dir(config, CheckpointInitMode::Required)?;
    prepared
        .directory
        .ok_or_else(|| io::Error::other("checkpoint directory unavailable in required mode"))
}

/// Builds checkpoint directory candidates in priority order.
pub(crate) fn checkpoint_dir_candidates(
    config: &CheckpointPathConfig,
) -> Vec<(CheckpointDirSource, PathBuf)> {
    let mut candidates = Vec::new();

    if let Some(path) = &config.explicit_dir {
        push_unique_candidate(&mut candidates, CheckpointDirSource::Explicit, path.clone());
    }

    if let Some(path) = env_path(config.env_var_name) {
        push_unique_candidate(&mut candidates, CheckpointDirSource::EnvVar, path);
    }

    if let Some(path) = default_checkpoint_dir_for_current_os(&config.app_name) {
        push_unique_candidate(&mut candidates, CheckpointDirSource::OsDefault, path);
    }

    if let Some(path) = &config.project_fallback_dir {
        push_unique_candidate(
            &mut candidates,
            CheckpointDirSource::ProjectFallback,
            path.clone(),
        );
    }

    push_unique_candidate(
        &mut candidates,
        CheckpointDirSource::LocalFallback,
        local_fallback_dir(),
    );

    candidates
}

/// Tries candidates in order and returns the first writable directory.
///
/// In `BestEffort` mode, returns `Ok` with `directory = None` when all
/// candidates fail, so callers can continue execution without snapshots.
pub fn initialize_checkpoint_dir(
    config: &CheckpointPathConfig,
    mode: CheckpointInitMode,
) -> io::Result<CheckpointInitResult> {
    let mut attempts: Vec<(CheckpointDirSource, PathBuf, io::ErrorKind)> = Vec::new();

    for (source, path) in checkpoint_dir_candidates(config) {
        match fs::create_dir_all(&path) {
            Ok(()) => {
                return Ok(CheckpointInitResult {
                    directory: Some(path),
                    source: Some(source),
                    attempts,
                });
            }
            Err(err) => {
                attempts.push((source, path, err.kind()));
            }
        }
    }

    match mode {
        CheckpointInitMode::Required => Err(io::Error::new(
            io::ErrorKind::PermissionDenied,
            format!(
                "unable to initialize checkpoint directory from any candidate ({} attempts)",
                attempts.len()
            ),
        )),
        CheckpointInitMode::BestEffort => Ok(CheckpointInitResult {
            directory: None,
            source: None,
            attempts,
        }),
    }
}

/// Creates a stable checkpoint file path for a run.
pub(crate) fn checkpoint_file_path(base_dir: &Path, run_id: &str) -> PathBuf {
    let run = sanitize_path_segment(run_id);
    base_dir.join(format!("run-{}.ckpt", run))
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

/// Writes one checkpoint record under `base_dir` using canonical naming.
/// Returns the full file path used for persistence.
pub fn write_execution_checkpoint(
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

/// Reads one checkpoint payload from disk.
pub(crate) fn read_checkpoint_record(path: &Path) -> io::Result<CheckpointRecord> {
    let data = fs::read(path)?;
    let mut cursor = io::Cursor::new(data.as_slice());

    let mut magic = [0u8; 4];
    cursor.read_exact(&mut magic)?;
    if magic != CHECKPOINT_BIN_MAGIC {
        return Err(io::Error::new(
            io::ErrorKind::InvalidData,
            "invalid checkpoint magic header",
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
            "trailing bytes found in checkpoint file",
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

fn list_checkpoint_files(base_dir: &Path) -> io::Result<Vec<PathBuf>> {
    let mut files = Vec::new();
    let mut pending = vec![base_dir.to_path_buf()];

    while let Some(dir) = pending.pop() {
        for entry in fs::read_dir(&dir)? {
            let entry = entry?;
            let path = entry.path();
            if path.is_dir() {
                pending.push(path);
                continue;
            }
            if path.extension().and_then(|ext| ext.to_str()) == Some("ckpt") {
                files.push(path);
            }
        }
    }
    Ok(files)
}

fn checkpoint_scope_dir(
    base_dir: &Path,
    algorithm_name: &str,
    problem_name: &str,
    algorithm_signature_hash: u64,
    problem_signature_hash: u64,
) -> PathBuf {
    let algorithm_segment = format!(
        "alg-{}-{:016x}",
        sanitize_path_segment(algorithm_name),
        algorithm_signature_hash
    );
    let problem_segment = format!(
        "prob-{}-{:016x}",
        sanitize_path_segment(problem_name),
        problem_signature_hash
    );
    base_dir.join(algorithm_segment).join(problem_segment)
}

fn run_id_timestamp_ms(run_id: &str) -> Option<u128> {
    let (_, timestamp) = run_id.rsplit_once('-')?;
    timestamp.parse::<u128>().ok()
}

fn is_resumable_status(status: CheckpointRunStatus) -> bool {
    matches!(
        status,
        CheckpointRunStatus::Running
            | CheckpointRunStatus::Failed
            | CheckpointRunStatus::Interrupted
    )
}

fn push_u8(out: &mut Vec<u8>, value: u8) {
    out.push(value);
}

fn push_u32(out: &mut Vec<u8>, value: u32) {
    out.extend_from_slice(&value.to_le_bytes());
}

fn push_u64(out: &mut Vec<u8>, value: u64) {
    out.extend_from_slice(&value.to_le_bytes());
}

fn push_f64(out: &mut Vec<u8>, value: f64) {
    out.extend_from_slice(&value.to_le_bytes());
}

fn push_usize(out: &mut Vec<u8>, value: usize) -> io::Result<()> {
    let value = u64::try_from(value).map_err(|_| {
        io::Error::new(
            io::ErrorKind::InvalidInput,
            "usize value too large to serialize into checkpoint",
        )
    })?;
    push_u64(out, value);
    Ok(())
}

fn push_string(out: &mut Vec<u8>, value: &str) -> io::Result<()> {
    let bytes = value.as_bytes();
    let len = u32::try_from(bytes.len()).map_err(|_| {
        io::Error::new(
            io::ErrorKind::InvalidInput,
            "string too large to serialize into checkpoint",
        )
    })?;
    push_u32(out, len);
    out.extend_from_slice(bytes);
    Ok(())
}

fn push_option_string(out: &mut Vec<u8>, value: &Option<String>) -> io::Result<()> {
    match value {
        Some(text) => {
            push_u8(out, 1);
            push_string(out, text)
        }
        None => {
            push_u8(out, 0);
            Ok(())
        }
    }
}

fn push_option_u64(out: &mut Vec<u8>, value: Option<u64>) {
    match value {
        Some(x) => {
            push_u8(out, 1);
            push_u64(out, x);
        }
        None => push_u8(out, 0),
    }
}

fn read_u8(input: &mut impl Read) -> io::Result<u8> {
    let mut bytes = [0u8; 1];
    input.read_exact(&mut bytes)?;
    Ok(bytes[0])
}

fn read_u32(input: &mut impl Read) -> io::Result<u32> {
    let mut bytes = [0u8; 4];
    input.read_exact(&mut bytes)?;
    Ok(u32::from_le_bytes(bytes))
}

fn read_u64(input: &mut impl Read) -> io::Result<u64> {
    let mut bytes = [0u8; 8];
    input.read_exact(&mut bytes)?;
    Ok(u64::from_le_bytes(bytes))
}

fn read_f64(input: &mut impl Read) -> io::Result<f64> {
    let mut bytes = [0u8; 8];
    input.read_exact(&mut bytes)?;
    Ok(f64::from_le_bytes(bytes))
}

fn read_usize(input: &mut impl Read) -> io::Result<usize> {
    let value = read_u64(input)?;
    usize::try_from(value).map_err(|_| {
        io::Error::new(
            io::ErrorKind::InvalidData,
            "u64 value too large to deserialize into usize",
        )
    })
}

fn read_string(input: &mut impl Read) -> io::Result<String> {
    let len = read_u32(input)? as usize;
    let mut bytes = vec![0u8; len];
    input.read_exact(&mut bytes)?;
    String::from_utf8(bytes).map_err(|_| {
        io::Error::new(
            io::ErrorKind::InvalidData,
            "invalid UTF-8 string in checkpoint",
        )
    })
}

fn read_option_string(input: &mut impl Read) -> io::Result<Option<String>> {
    match read_u8(input)? {
        0 => Ok(None),
        1 => Ok(Some(read_string(input)?)),
        flag => Err(io::Error::new(
            io::ErrorKind::InvalidData,
            format!("invalid option flag for string: {}", flag),
        )),
    }
}

fn read_option_u64(input: &mut impl Read) -> io::Result<Option<u64>> {
    match read_u8(input)? {
        0 => Ok(None),
        1 => Ok(Some(read_u64(input)?)),
        flag => Err(io::Error::new(
            io::ErrorKind::InvalidData,
            format!("invalid option flag for u64: {}", flag),
        )),
    }
}

fn status_to_byte(status: CheckpointRunStatus) -> u8 {
    match status {
        CheckpointRunStatus::Running => 0,
        CheckpointRunStatus::Completed => 1,
        CheckpointRunStatus::Failed => 2,
        CheckpointRunStatus::Interrupted => 3,
    }
}

fn byte_to_status(value: u8) -> io::Result<CheckpointRunStatus> {
    match value {
        0 => Ok(CheckpointRunStatus::Running),
        1 => Ok(CheckpointRunStatus::Completed),
        2 => Ok(CheckpointRunStatus::Failed),
        3 => Ok(CheckpointRunStatus::Interrupted),
        _ => Err(io::Error::new(
            io::ErrorKind::InvalidData,
            format!("invalid checkpoint status byte: {}", value),
        )),
    }
}

fn env_path(name: &str) -> Option<PathBuf> {
    let value = env::var_os(name)?;
    if value.as_os_str().is_empty() {
        return None;
    }
    Some(PathBuf::from(value))
}

fn local_fallback_dir() -> PathBuf {
    PathBuf::from(".").join(".roma").join("checkpoints")
}

fn push_unique_candidate(
    candidates: &mut Vec<(CheckpointDirSource, PathBuf)>,
    source: CheckpointDirSource,
    path: PathBuf,
) {
    if candidates.iter().any(|(_, existing)| existing == &path) {
        return;
    }

    candidates.push((source, path));
}

#[cfg(target_os = "linux")]
fn default_checkpoint_dir_for_current_os(app_name: &str) -> Option<PathBuf> {
    if let Some(base) = env_path("XDG_STATE_HOME") {
        return Some(base.join(app_name).join("checkpoints"));
    }

    let home = env_path("HOME")?;
    Some(
        home.join(".local")
            .join("state")
            .join(app_name)
            .join("checkpoints"),
    )
}

#[cfg(target_os = "macos")]
fn default_checkpoint_dir_for_current_os(app_name: &str) -> Option<PathBuf> {
    let home = env_path("HOME")?;
    Some(
        home.join("Library")
            .join("Application Support")
            .join(app_name)
            .join("checkpoints"),
    )
}

#[cfg(target_os = "windows")]
fn default_checkpoint_dir_for_current_os(app_name: &str) -> Option<PathBuf> {
    if let Some(base) = env_path("LOCALAPPDATA") {
        return Some(base.join(app_name).join("checkpoints"));
    }

    let roaming = env_path("APPDATA")?;
    Some(roaming.join(app_name).join("checkpoints"))
}

#[cfg(not(any(target_os = "linux", target_os = "macos", target_os = "windows")))]
fn default_checkpoint_dir_for_current_os(app_name: &str) -> Option<PathBuf> {
    let home = env_path("HOME")?;
    Some(home.join(format!(".{}", app_name)).join("checkpoints"))
}

fn sanitize_path_segment(value: &str) -> String {
    let mut out = String::new();
    for ch in value.chars() {
        if ch.is_ascii_alphanumeric() || ch == '-' || ch == '_' {
            out.push(ch);
        } else {
            out.push('_');
        }
    }

    let trimmed = out.trim_matches('_');
    if trimmed.is_empty() {
        return "unnamed".to_string();
    }

    trimmed.to_string()
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn sanitize_path_segment_keeps_safe_ascii() {
        let sanitized = sanitize_path_segment("run-A_01");
        assert_eq!(sanitized, "run-A_01");
    }

    #[test]
    fn sanitize_path_segment_replaces_unsafe_chars() {
        let sanitized = sanitize_path_segment("run:/ 01");
        assert_eq!(sanitized, "run___01");
    }

    #[test]
    fn sanitize_path_segment_defaults_when_empty_after_cleanup() {
        let sanitized = sanitize_path_segment("@@@");
        assert_eq!(sanitized, "unnamed");
    }

    #[test]
    fn checkpoint_file_path_uses_stable_name() {
        let path = checkpoint_file_path(Path::new("/tmp/checkpoints"), "trial 7");
        let file_name = path
            .file_name()
            .and_then(|x| x.to_str())
            .unwrap_or_default();
        assert_eq!(file_name, "run-trial_7.ckpt");
    }

    #[test]
    fn explicit_dir_has_priority() {
        let cfg = CheckpointPathConfig::default().with_explicit_dir("/my/checkpoints");
        let resolved = resolve_checkpoint_dir(&cfg);
        assert_eq!(resolved, PathBuf::from("/my/checkpoints"));
    }

    #[test]
    fn candidates_always_include_local_fallback() {
        let cfg = CheckpointPathConfig::default();
        let candidates = checkpoint_dir_candidates(&cfg);
        assert!(candidates.iter().any(|(source, path)| *source
            == CheckpointDirSource::LocalFallback
            && *path == PathBuf::from("./.roma/checkpoints")));
    }

    #[test]
    fn initialize_best_effort_can_succeed() {
        let cfg = CheckpointPathConfig::default();
        let result = initialize_checkpoint_dir(&cfg, CheckpointInitMode::BestEffort);
        assert!(result.is_ok());
    }

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
}
