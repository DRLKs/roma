pub(crate) mod implementations;
pub(crate) mod traits;

use std::path::PathBuf;

use crate::algorithms::termination::{ExecutionStateSnapshot, TerminationReason};
pub use implementations::chart_observer::ChartObserver;
pub use implementations::console_observer::ConsoleObserver;
pub use implementations::html_report_observer::HtmlReportObserver;
pub use traits::{AlgorithmObserver, Observable};

pub(crate) fn default_observers_output_path() -> PathBuf {
    PathBuf::from(env!("CARGO_MANIFEST_DIR"))
        .join("target")
        .join("observers_outputs")
}

/// Events that can be observed during algorithm execution
#[derive(Debug, Clone)]
pub enum AlgorithmEvent<T, Q = f64>
where
    T: Clone,
    Q: Clone,
{
    /// Algorithm has started
    Start {
        algorithm_name: String,
    },
    /// Shared execution snapshot update
    ExecutionStateUpdated {
        state: ExecutionStateSnapshot<T, Q>,
    },
    /// Algorithm has finished
    End {
        total_generations: usize,
        total_evaluations: usize,
        termination_reason: Option<TerminationReason>,
    },
    _Phantom(std::marker::PhantomData<T>),
}
