use std::env;
use std::fs;
use std::io;
use std::path::{Path, PathBuf};

use super::{DEFAULT_APP_NAME, DEFAULT_CHECKPOINT_ENV_VAR};

// Checkpoint files are stored with this extension under scoped directories.
const CHECKPOINT_FILE_EXTENSION: &str = "ckpt";
const RUN_ID_TIMESTAMP_SEPARATOR: char = '-';
const LOCAL_FALLBACK_RELATIVE_PATH: &str = "./.roma/checkpoints";
const CHECKPOINTS_DIR_NAME: &str = "checkpoints";
const DEFAULT_SANITIZED_SEGMENT: &str = "unnamed";
const SANITIZED_REPLACEMENT_CHAR: char = '_';

const ERR_CHECKPOINT_DIR_REQUIRED_UNAVAILABLE: &str =
    "checkpoint directory unavailable in required mode";

#[cfg(target_os = "linux")]
const ENV_XDG_STATE_HOME: &str = "XDG_STATE_HOME";
const ENV_HOME: &str = "HOME";
#[cfg(target_os = "windows")]
const ENV_LOCALAPPDATA: &str = "LOCALAPPDATA";
#[cfg(target_os = "windows")]
const ENV_APPDATA: &str = "APPDATA";
#[cfg(target_os = "linux")]
const LINUX_LOCAL_STATE_SEGMENT: &str = ".local";
#[cfg(target_os = "linux")]
const LINUX_STATE_SEGMENT: &str = "state";
#[cfg(target_os = "macos")]
const MACOS_LIBRARY_SEGMENT: &str = "Library";
#[cfg(target_os = "macos")]
const MACOS_APP_SUPPORT_SEGMENT: &str = "Application Support";

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
        .ok_or_else(|| io::Error::other(ERR_CHECKPOINT_DIR_REQUIRED_UNAVAILABLE))
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
    base_dir.join(format!("run-{}.{}", run, CHECKPOINT_FILE_EXTENSION))
}

pub(crate) fn list_checkpoint_files(base_dir: &Path) -> io::Result<Vec<PathBuf>> {
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
            if path.extension().and_then(|ext| ext.to_str()) == Some(CHECKPOINT_FILE_EXTENSION) {
                files.push(path);
            }
        }
    }
    Ok(files)
}

pub(crate) fn checkpoint_scope_dir(
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

pub(crate) fn run_id_timestamp_ms(run_id: &str) -> Option<u128> {
    let (_, timestamp) = run_id.rsplit_once(RUN_ID_TIMESTAMP_SEPARATOR)?;
    timestamp.parse::<u128>().ok()
}

fn env_path(name: &str) -> Option<PathBuf> {
    let value = env::var_os(name)?;
    if value.as_os_str().is_empty() {
        return None;
    }
    Some(PathBuf::from(value))
}

fn local_fallback_dir() -> PathBuf {
    PathBuf::from(LOCAL_FALLBACK_RELATIVE_PATH)
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
    if let Some(base) = env_path(ENV_XDG_STATE_HOME) {
        return Some(base.join(app_name).join(CHECKPOINTS_DIR_NAME));
    }

    let home = env_path(ENV_HOME)?;
    Some(
        home.join(LINUX_LOCAL_STATE_SEGMENT)
            .join(LINUX_STATE_SEGMENT)
            .join(app_name)
            .join(CHECKPOINTS_DIR_NAME),
    )
}

#[cfg(target_os = "macos")]
fn default_checkpoint_dir_for_current_os(app_name: &str) -> Option<PathBuf> {
    let home = env_path(ENV_HOME)?;
    Some(
        home.join(MACOS_LIBRARY_SEGMENT)
            .join(MACOS_APP_SUPPORT_SEGMENT)
            .join(app_name)
            .join(CHECKPOINTS_DIR_NAME),
    )
}

#[cfg(target_os = "windows")]
fn default_checkpoint_dir_for_current_os(app_name: &str) -> Option<PathBuf> {
    if let Some(base) = env_path(ENV_LOCALAPPDATA) {
        return Some(base.join(app_name).join(CHECKPOINTS_DIR_NAME));
    }

    let roaming = env_path(ENV_APPDATA)?;
    Some(roaming.join(app_name).join(CHECKPOINTS_DIR_NAME))
}

#[cfg(not(any(target_os = "linux", target_os = "macos", target_os = "windows")))]
fn default_checkpoint_dir_for_current_os(app_name: &str) -> Option<PathBuf> {
    let home = env_path(ENV_HOME)?;
    Some(
        home.join(format!(".{}", app_name))
            .join(CHECKPOINTS_DIR_NAME),
    )
}

fn sanitize_path_segment(value: &str) -> String {
    let mut out = String::new();
    for ch in value.chars() {
        if ch.is_ascii_alphanumeric() || ch == '-' || ch == '_' {
            out.push(ch);
        } else {
            out.push(SANITIZED_REPLACEMENT_CHAR);
        }
    }

    let trimmed = out.trim_matches('_');
    if trimmed.is_empty() {
        return DEFAULT_SANITIZED_SEGMENT.to_string();
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
            && *path == PathBuf::from(LOCAL_FALLBACK_RELATIVE_PATH)));
    }

    #[test]
    fn initialize_best_effort_can_succeed() {
        let cfg = CheckpointPathConfig::default();
        let result = initialize_checkpoint_dir(&cfg, CheckpointInitMode::BestEffort);
        assert!(result.is_ok());
    }
}
