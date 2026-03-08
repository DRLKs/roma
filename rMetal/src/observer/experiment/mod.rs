pub mod traits;
pub mod implementations;

pub use traits::{ExperimentObservable, ExperimentObserver};
pub use implementations::console_observer::ExperimentConsoleObserver;

use crate::experiment::{ExperimentReport, Objective};

/// Events emitted during experiment execution.
#[derive(Debug, Clone)]
pub enum ExperimentEvent {
    Start {
        name: String,
        objective: Objective,
        runs_per_case: usize,
        total_cases: usize,
    },
    CaseStarted {
        algorithm: String,
        configuration: String,
        problem: String,
    },
    RunCompleted {
        algorithm: String,
        configuration: String,
        problem: String,
        run_index: usize,
        seed: u64,
        best_value: f64,
    },
    End {
        report: ExperimentReport,
    },
    Error {
        algorithm: String,
        configuration: String,
        problem: String,
        message: String,
    },
}
