use std::env;
use std::fs;
use std::io;
use std::path::{Path, PathBuf};

pub const DEFAULT_CHECKPOINT_ENV_VAR: &str = "RMETAL_CHECKPOINT_DIR";
pub const DEFAULT_APP_NAME: &str = "rmetal";

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

    fn from_str(value: &str) -> Option<Self> {
        match value {
            "running" => Some(CheckpointRunStatus::Running),
            "completed" => Some(CheckpointRunStatus::Completed),
            "failed" => Some(CheckpointRunStatus::Failed),
            "interrupted" => Some(CheckpointRunStatus::Interrupted),
            _ => None,
        }
    }
}

#[derive(Debug, Clone, PartialEq)]
pub struct CheckpointRecord {
    pub run_id: String,
    pub algorithm_name: String,
    pub problem_fingerprint: String,
    pub seq_id: u64,
    pub iteration: usize,
    pub evaluations: usize,
    pub best_fitness: f64,
    pub average_fitness: f64,
    pub worst_fitness: f64,
    pub best_solution_presentation: String,
    pub state_payload: Option<String>,
    pub status: CheckpointRunStatus,
    pub error_message: Option<String>,
}

/// Configures how checkpoint directories are resolved.
///
/// Priority order:
/// 1. `explicit_dir`
/// 2. env var named by `env_var_name`
/// 3. OS-specific default location
/// 4. `project_fallback_dir`
/// 5. `./.rmetal/checkpoints`
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
pub fn checkpoint_dir_candidates(
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

/// Creates a stable checkpoint file path for a run sequence.
pub fn checkpoint_file_path(base_dir: &Path, run_id: &str, seq_id: u64) -> PathBuf {
    let run = sanitize_path_segment(run_id);
    base_dir.join(format!("run-{}-seq-{:020}.ckpt", run, seq_id))
}

/// Writes one checkpoint payload as key-value text.
pub fn write_checkpoint_record(path: &Path, record: &CheckpointRecord) -> io::Result<()> {
    let mut body = String::new();
    body.push_str(&format!("run_id={}\n", sanitize_value(&record.run_id)));
    body.push_str(&format!(
        "algorithm_name={}\n",
        sanitize_value(&record.algorithm_name)
    ));
    body.push_str(&format!(
        "problem_fingerprint={}\n",
        sanitize_value(&record.problem_fingerprint)
    ));
    body.push_str(&format!("seq_id={}\n", record.seq_id));
    body.push_str(&format!("iteration={}\n", record.iteration));
    body.push_str(&format!("evaluations={}\n", record.evaluations));
    body.push_str(&format!("best_fitness={:.17}\n", record.best_fitness));
    body.push_str(&format!("average_fitness={:.17}\n", record.average_fitness));
    body.push_str(&format!("worst_fitness={:.17}\n", record.worst_fitness));
    body.push_str(&format!(
        "best_solution_presentation={}\n",
        sanitize_value(&record.best_solution_presentation)
    ));
    if let Some(payload) = &record.state_payload {
        body.push_str(&format!("state_payload={}\n", sanitize_value(payload)));
    }
    body.push_str(&format!("status={}\n", record.status.as_str()));
    if let Some(message) = &record.error_message {
        body.push_str(&format!("error_message={}\n", sanitize_value(message)));
    }

    fs::write(path, body)
}

/// Reads one checkpoint payload from disk.
pub fn read_checkpoint_record(path: &Path) -> io::Result<CheckpointRecord> {
    let contents = fs::read_to_string(path)?;
    parse_checkpoint_record(&contents)
}

/// Lists available checkpoints in ascending sequence order for one run id.
pub fn list_checkpoints(base_dir: &Path, run_id: &str) -> io::Result<Vec<PathBuf>> {
    let run = sanitize_path_segment(run_id);
    let prefix = format!("run-{}-seq-", run);
    let mut entries: Vec<(u64, PathBuf)> = Vec::new();

    for entry in fs::read_dir(base_dir)? {
        let entry = entry?;
        let path = entry.path();
        if path.extension().and_then(|ext| ext.to_str()) != Some("ckpt") {
            continue;
        }

        let file_name = match path.file_name().and_then(|name| name.to_str()) {
            Some(name) => name,
            None => continue,
        };

        if !file_name.starts_with(&prefix) {
            continue;
        }

        if let Some(seq_id) = parse_seq_from_filename(file_name) {
            entries.push((seq_id, path));
        }
    }

    entries.sort_by_key(|(seq_id, _)| *seq_id);
    Ok(entries.into_iter().map(|(_, path)| path).collect())
}

/// Loads the latest checkpoint payload for one run id.
pub fn latest_checkpoint_record(
    base_dir: &Path,
    run_id: &str,
) -> io::Result<Option<CheckpointRecord>> {
    let paths = list_checkpoints(base_dir, run_id)?;
    match paths.last() {
        Some(path) => read_checkpoint_record(path).map(Some),
        None => Ok(None),
    }
}

/// Lists unique run ids that have at least one checkpoint file.
pub fn list_checkpoint_run_ids(base_dir: &Path) -> io::Result<Vec<String>> {
    let mut run_ids = std::collections::BTreeSet::new();
    for path in list_checkpoint_files(base_dir)? {
        if let Ok(record) = read_checkpoint_record(&path) {
            run_ids.insert(record.run_id);
        }
    }
    Ok(run_ids.into_iter().collect())
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

/// Loads the latest checkpoint compatible with algorithm + problem fingerprint
/// and resumable status (running/failed/interrupted).
pub fn latest_resumable_checkpoint_for(
    base_dir: &Path,
    algorithm_name: &str,
    problem_fingerprint: &str,
) -> io::Result<Option<CheckpointRecord>> {
    let mut best: Option<((u128, String, u64), CheckpointRecord)> = None;

    for path in list_checkpoint_files(base_dir)? {
        let Ok(record) = read_checkpoint_record(&path) else {
            continue;
        };

        if record.algorithm_name != algorithm_name {
            continue;
        }
        if record.problem_fingerprint != problem_fingerprint {
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

fn list_checkpoint_files(base_dir: &Path) -> io::Result<Vec<PathBuf>> {
    let mut files = Vec::new();
    for entry in fs::read_dir(base_dir)? {
        let entry = entry?;
        let path = entry.path();
        if path.extension().and_then(|ext| ext.to_str()) == Some("ckpt") {
            files.push(path);
        }
    }
    Ok(files)
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

fn parse_seq_from_filename(file_name: &str) -> Option<u64> {
    let stem = file_name.strip_suffix(".ckpt")?;
    let (_, seq) = stem.rsplit_once("-seq-")?;
    seq.parse::<u64>().ok()
}

fn parse_checkpoint_record(contents: &str) -> io::Result<CheckpointRecord> {
    let mut run_id: Option<String> = None;
    let mut algorithm_name: Option<String> = None;
    let mut problem_fingerprint: Option<String> = None;
    let mut seq_id: Option<u64> = None;
    let mut iteration: Option<usize> = None;
    let mut evaluations: Option<usize> = None;
    let mut best_fitness: Option<f64> = None;
    let mut average_fitness: Option<f64> = None;
    let mut worst_fitness: Option<f64> = None;
    let mut best_solution_presentation: Option<String> = None;
    let mut state_payload: Option<String> = None;
    let mut status: Option<CheckpointRunStatus> = None;
    let mut error_message: Option<String> = None;

    for line in contents.lines() {
        if line.trim().is_empty() {
            continue;
        }

        let (key, value) = line.split_once('=').ok_or_else(|| {
            io::Error::new(
                io::ErrorKind::InvalidData,
                format!("invalid checkpoint line: {}", line),
            )
        })?;

        match key {
            "run_id" => run_id = Some(desanitize_value(value)),
            "algorithm_name" => algorithm_name = Some(desanitize_value(value)),
            "problem_fingerprint" => problem_fingerprint = Some(desanitize_value(value)),
            "seq_id" => seq_id = Some(parse_u64_field("seq_id", value)?),
            "iteration" => iteration = Some(parse_usize_field("iteration", value)?),
            "evaluations" => evaluations = Some(parse_usize_field("evaluations", value)?),
            "best_fitness" => best_fitness = Some(parse_f64_field("best_fitness", value)?),
            "average_fitness" => average_fitness = Some(parse_f64_field("average_fitness", value)?),
            "worst_fitness" => worst_fitness = Some(parse_f64_field("worst_fitness", value)?),
            "best_solution_presentation" => {
                best_solution_presentation = Some(desanitize_value(value))
            }
            "state_payload" => state_payload = Some(desanitize_value(value)),
            "status" => {
                status = CheckpointRunStatus::from_str(value);
                if status.is_none() {
                    return Err(io::Error::new(
                        io::ErrorKind::InvalidData,
                        format!("invalid status value: {}", value),
                    ));
                }
            }
            "error_message" => error_message = Some(desanitize_value(value)),
            _ => {}
        }
    }

    let run_id = required_field("run_id", run_id)?;
    let algorithm_name = algorithm_name
        .or_else(|| derive_algorithm_name_from_run_id(&run_id))
        .ok_or_else(|| {
            io::Error::new(
                io::ErrorKind::InvalidData,
                "missing checkpoint field 'algorithm_name'",
            )
        })?;
    let problem_fingerprint = problem_fingerprint.unwrap_or_default();

    Ok(CheckpointRecord {
        run_id,
        algorithm_name,
        problem_fingerprint,
        seq_id: required_field("seq_id", seq_id)?,
        iteration: required_field("iteration", iteration)?,
        evaluations: required_field("evaluations", evaluations)?,
        best_fitness: required_field("best_fitness", best_fitness)?,
        average_fitness: required_field("average_fitness", average_fitness)?,
        worst_fitness: required_field("worst_fitness", worst_fitness)?,
        best_solution_presentation: required_field(
            "best_solution_presentation",
            best_solution_presentation,
        )?,
        state_payload,
        status: required_field("status", status)?,
        error_message,
    })
}

fn derive_algorithm_name_from_run_id(run_id: &str) -> Option<String> {
    let mut parts = run_id.rsplitn(3, '-');
    let _timestamp = parts.next()?;
    let _pid = parts.next()?;
    let algorithm = parts.next()?;
    if algorithm.is_empty() {
        None
    } else {
        Some(algorithm.to_string())
    }
}

fn required_field<T>(name: &str, value: Option<T>) -> io::Result<T> {
    value.ok_or_else(|| {
        io::Error::new(
            io::ErrorKind::InvalidData,
            format!("missing checkpoint field '{}'", name),
        )
    })
}

fn parse_usize_field(name: &str, value: &str) -> io::Result<usize> {
    value.parse::<usize>().map_err(|_| {
        io::Error::new(
            io::ErrorKind::InvalidData,
            format!("invalid usize value for '{}': {}", name, value),
        )
    })
}

fn parse_u64_field(name: &str, value: &str) -> io::Result<u64> {
    value.parse::<u64>().map_err(|_| {
        io::Error::new(
            io::ErrorKind::InvalidData,
            format!("invalid u64 value for '{}': {}", name, value),
        )
    })
}

fn parse_f64_field(name: &str, value: &str) -> io::Result<f64> {
    value.parse::<f64>().map_err(|_| {
        io::Error::new(
            io::ErrorKind::InvalidData,
            format!("invalid f64 value for '{}': {}", name, value),
        )
    })
}

fn sanitize_value(value: &str) -> String {
    value
        .replace('\\', "\\\\")
        .replace('\n', "\\n")
        .replace('\r', "\\r")
}

fn desanitize_value(value: &str) -> String {
    let mut out = String::with_capacity(value.len());
    let mut chars = value.chars();

    while let Some(ch) = chars.next() {
        if ch != '\\' {
            out.push(ch);
            continue;
        }

        match chars.next() {
            Some('n') => out.push('\n'),
            Some('r') => out.push('\r'),
            Some('\\') => out.push('\\'),
            Some(other) => {
                out.push('\\');
                out.push(other);
            }
            None => out.push('\\'),
        }
    }

    out
}

fn env_path(name: &str) -> Option<PathBuf> {
    let value = env::var_os(name)?;
    if value.as_os_str().is_empty() {
        return None;
    }
    Some(PathBuf::from(value))
}

fn local_fallback_dir() -> PathBuf {
    PathBuf::from(".").join(".rmetal").join("checkpoints")
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
        let path = checkpoint_file_path(Path::new("/tmp/checkpoints"), "trial 7", 42);
        let file_name = path
            .file_name()
            .and_then(|x| x.to_str())
            .unwrap_or_default();
        assert_eq!(file_name, "run-trial_7-seq-00000000000000000042.ckpt");
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
            && *path == PathBuf::from("./.rmetal/checkpoints")));
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
            "rmetal_checkpoint_roundtrip_test_{}",
            std::process::id()
        ));
        fs::create_dir_all(&base).expect("checkpoint test dir should be creatable");

        let record = CheckpointRecord {
            run_id: "HillClimbing-1-123".to_string(),
            algorithm_name: "HillClimbing".to_string(),
            problem_fingerprint: "demo-problem".to_string(),
            seq_id: 7,
            iteration: 12,
            evaluations: 44,
            best_fitness: 9.75,
            average_fitness: 8.2,
            worst_fitness: 3.1,
            best_solution_presentation: "selected=2/4".to_string(),
            state_payload: Some("seed=42".to_string()),
            status: CheckpointRunStatus::Failed,
            error_message: Some("synthetic error".to_string()),
        };

        let path = checkpoint_file_path(&base, &record.run_id, record.seq_id);
        write_checkpoint_record(&path, &record).expect("checkpoint should be writable");

        let loaded = read_checkpoint_record(&path).expect("checkpoint should be readable");
        assert_eq!(loaded, record);
    }

    #[test]
    fn latest_checkpoint_record_for_algorithm_uses_prefix_match() {
        let base = std::env::temp_dir().join(format!(
            "rmetal_checkpoint_latest_algorithm_test_{}",
            std::process::id()
        ));
        fs::create_dir_all(&base).expect("checkpoint test dir should be creatable");

        let run_id = format!("HillClimbing-{}-{}", std::process::id(), 1000);
        let older = CheckpointRecord {
            run_id: run_id.clone(),
            algorithm_name: "HillClimbing".to_string(),
            problem_fingerprint: "demo-problem".to_string(),
            seq_id: 1,
            iteration: 1,
            evaluations: 2,
            best_fitness: 1.0,
            average_fitness: 1.0,
            worst_fitness: 1.0,
            best_solution_presentation: "old".to_string(),
            state_payload: Some("seed=10".to_string()),
            status: CheckpointRunStatus::Running,
            error_message: None,
        };
        let newer = CheckpointRecord {
            run_id,
            algorithm_name: "HillClimbing".to_string(),
            problem_fingerprint: "demo-problem".to_string(),
            seq_id: 2,
            iteration: 2,
            evaluations: 3,
            best_fitness: 2.0,
            average_fitness: 2.0,
            worst_fitness: 2.0,
            best_solution_presentation: "new".to_string(),
            state_payload: Some("seed=10".to_string()),
            status: CheckpointRunStatus::Completed,
            error_message: None,
        };

        let older_path = checkpoint_file_path(&base, &older.run_id, older.seq_id);
        let newer_path = checkpoint_file_path(&base, &newer.run_id, newer.seq_id);
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
            "rmetal_checkpoint_resumable_test_{}",
            std::process::id()
        ));
        fs::create_dir_all(&base).expect("checkpoint test dir should be creatable");

        let run_a = format!("HillClimbing-{}-{}", std::process::id(), 2000);
        let run_b = format!("HillClimbing-{}-{}", std::process::id(), 3000);

        let completed = CheckpointRecord {
            run_id: run_a,
            algorithm_name: "HillClimbing".to_string(),
            problem_fingerprint: "problem-a".to_string(),
            seq_id: 3,
            iteration: 3,
            evaluations: 4,
            best_fitness: 3.0,
            average_fitness: 3.0,
            worst_fitness: 3.0,
            best_solution_presentation: "completed".to_string(),
            state_payload: Some("seed=33".to_string()),
            status: CheckpointRunStatus::Completed,
            error_message: None,
        };

        let failed = CheckpointRecord {
            run_id: run_b,
            algorithm_name: "HillClimbing".to_string(),
            problem_fingerprint: "problem-a".to_string(),
            seq_id: 8,
            iteration: 8,
            evaluations: 9,
            best_fitness: 8.0,
            average_fitness: 8.0,
            worst_fitness: 8.0,
            best_solution_presentation: "failed".to_string(),
            state_payload: Some("seed=44".to_string()),
            status: CheckpointRunStatus::Failed,
            error_message: Some("x".to_string()),
        };

        write_checkpoint_record(
            &checkpoint_file_path(&base, &completed.run_id, completed.seq_id),
            &completed,
        )
        .expect("completed checkpoint should be writable");
        write_checkpoint_record(
            &checkpoint_file_path(&base, &failed.run_id, failed.seq_id),
            &failed,
        )
        .expect("failed checkpoint should be writable");

        let latest = latest_resumable_checkpoint_for(&base, "HillClimbing", "problem-a")
            .expect("latest resumable should be readable")
            .expect("one resumable checkpoint should exist");

        assert_eq!(latest.status, CheckpointRunStatus::Failed);
        assert_eq!(latest.seq_id, 8);
    }
}
