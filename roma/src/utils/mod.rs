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
pub use checkpoint::{delete_snapshot_on_success, read_snapshot, write_snapshot};
pub use cli::seed_from_cli_or;
pub use parallel::resolve_num_threads;
pub use random::{seed_from_time, Random};
