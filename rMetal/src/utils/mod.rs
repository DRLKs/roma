//! Utility modules used across Roma.
//!
//! This includes random utilities, benchmarking helpers, checkpoint management,
//! file-format adapters (CSV/JSON/YAML), and chart/statistics support.

pub mod benchmark;
pub mod chart;
pub mod checkpoint;
pub mod cli;
pub mod csv_adapter;
pub mod json_adapter;
pub(crate) mod parallel;
pub mod random;
pub mod statistics;
pub mod yaml_adapter;

pub use benchmark::{measure, measure_result, speedup};
pub use checkpoint::{
    checkpoint_dir_candidates, checkpoint_file_path, ensure_checkpoint_dir,
    initialize_checkpoint_dir, latest_checkpoint_record, latest_checkpoint_record_for_algorithm,
    latest_resumable_checkpoint_for, list_checkpoint_run_ids, list_checkpoints,
    read_checkpoint_record, resolve_checkpoint_dir, write_checkpoint_record, CheckpointDirSource,
    CheckpointInitMode, CheckpointInitResult, CheckpointPathConfig, CheckpointRecord,
    CheckpointRunStatus, DEFAULT_APP_NAME, DEFAULT_CHECKPOINT_ENV_VAR,
};
pub use cli::seed_from_cli_or;
pub use random::{seed_from_time, Random};
