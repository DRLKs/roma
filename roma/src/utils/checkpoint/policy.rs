use std::fs;
use std::path::{Path, PathBuf};

use super::{CheckpointRecord, write_snapshot};
use crate::utils::cli::CliArgs;
use crate::utils::path::{
    CheckpointInitMode, CheckpointPathConfig, initialize_checkpoint_dir, resolve_checkpoint_dir,
};

pub const DEFAULT_FREQUENCY_OF_CHECKPOINT_WRITES: usize = 10;

/// Resolves checkpoint settings once for a single algorithm execution.
pub struct CheckpointPolicy {
    checkpoint_dir: PathBuf,
    pub(crate) storage_ready: bool,
    pub(crate) writes_requested: bool,
    resume_requested: bool,
}

impl CheckpointPolicy {
    pub fn from_cli(checkpoint_cfg: &CheckpointPathConfig) -> Self {
        Self::resolve(&CliArgs::from_env(), checkpoint_cfg)
    }

    pub fn resolve(args: &CliArgs, checkpoint_cfg: &CheckpointPathConfig) -> Self {
        if args.has_checkpoint_dir_override() {
            let checkpoint_dir = args.checkpoint_dir_or(resolve_checkpoint_dir(checkpoint_cfg));
            return Self {
                storage_ready: fs::create_dir_all(&checkpoint_dir).is_ok(),
                checkpoint_dir,
                writes_requested: !args.checkpoints_disabled(),
                resume_requested: args.resume_requested(),
            };
        }

        let prepared_dir = resolve_checkpoint_dir_from_config_for_writes(checkpoint_cfg).ok();
        let checkpoint_dir = prepared_dir
            .clone()
            .unwrap_or_else(|| resolve_checkpoint_dir(checkpoint_cfg));

        Self {
            checkpoint_dir,
            storage_ready: prepared_dir.is_some(),
            writes_requested: !args.checkpoints_disabled(),
            resume_requested: args.resume_requested(),
        }
    }

    pub fn checkpoint_dir(&self) -> &Path {
        &self.checkpoint_dir
    }
    pub fn resume_requested(&self) -> bool {
        self.resume_requested
    }
    pub fn should_write(&self) -> bool {
        self.storage_ready && self.writes_requested
    }

    pub fn persist_record(
        &self,
        last_checkpoint_path: &mut Option<PathBuf>,
        record: &CheckpointRecord,
    ) {
        if self.should_write() {
            if let Ok(path) = write_snapshot(self.checkpoint_dir(), record) {
                *last_checkpoint_path = Some(path);
            }
        }
    }
}

pub(crate) fn resolve_checkpoint_dir_from_config_for_writes(
    config: &CheckpointPathConfig,
) -> Result<PathBuf, String> {
    initialize_checkpoint_dir(config, CheckpointInitMode::BestEffort)
        .map_err(|err| format!("failed to initialize checkpoint directory: {err}"))
        .and_then(|result| {
            result.directory.ok_or_else(|| {
                "no writable checkpoint directory available; checkpoint writes disabled".to_string()
            })
        })
}
