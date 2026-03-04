pub(crate) mod traits;
pub(crate) mod implementations;

pub use traits::{AlgorithmObserver, Observable};
pub use implementations::chart_observer::ChartObserver;
pub use implementations::console_observer::ConsoleObserver;
pub use implementations::html_report_observer::HtmlReportObserver;
use crate::solution::traits::{QualityValue, ScalarQuality};
use crate::algorithms::termination::{ExecutionStateSnapshot, TerminationReason};

/// Events that can be observed during algorithm execution
#[derive(Debug, Clone)]
pub enum AlgorithmEvent<T, Q = ScalarQuality>
where
    T: Clone,
    Q: Clone + QualityValue,
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
    /// An error occurred during execution
    Error {
        message: String,
    },
    _Phantom(std::marker::PhantomData<T>),
}