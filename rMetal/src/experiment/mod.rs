pub mod traits;
mod utils;
mod report;
mod executor;
mod parallel;

pub use report::{
    ExperimentFailure,
    ExperimentReport,
    ExperimentRunResult,
    ExperimentSummary,
};
pub use executor::Experiment;