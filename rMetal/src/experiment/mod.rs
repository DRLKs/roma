//! Experiment execution and reporting APIs.
//!
//! [`Experiment`] runs multiple algorithm cases on the same problem for repeated
//! trials, then aggregates best/mean/worst metrics and failure details into an
//! [`ExperimentReport`].

mod executor;
mod parallel;
mod report;
pub mod traits;
mod utils;

pub use executor::Experiment;
pub use report::{ExperimentFailure, ExperimentReport, ExperimentRunResult, ExperimentSummary};
