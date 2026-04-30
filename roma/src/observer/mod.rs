//! Observer system for algorithm execution events.
//!
//! Observers consume [`AlgorithmEvent`] values emitted by the shared runtime.
//! Built-in observers include console output, SVG chart generation, and HTML
//! summary reports.

pub(crate) mod implementations;
pub(crate) mod traits;

use std::path::PathBuf;

use crate::algorithms::termination::{ExecutionStateSnapshot, TerminationReason};
pub use implementations::chart_observer::ChartObserver;
pub use implementations::console_observer::ConsoleObserver;
pub use implementations::html_report_observer::HtmlReportObserver;
pub use traits::{AlgorithmObserver, Observable};

/// Observer-facing execution payload with only presentation-relevant fields.
#[derive(Debug, Clone)]
pub struct ObserverState {
    pub seq_id: u64,
    pub iteration: usize,
    pub evaluations: usize,
    pub best_fitness: f64,
    pub average_fitness: f64,
    pub worst_fitness: f64,
    pub best_solution_presentation: String,
}

impl ObserverState {
    pub fn new(
        seq_id: u64,
        iteration: usize,
        evaluations: usize,
        best_fitness: f64,
        average_fitness: f64,
        worst_fitness: f64,
        best_solution_presentation: String,
    ) -> Self {
        Self {
            seq_id,
            iteration,
            evaluations,
            best_fitness,
            average_fitness,
            worst_fitness,
            best_solution_presentation,
        }
    }

    pub(crate) fn from_snapshot<T, Q>(
        snapshot: ExecutionStateSnapshot<T, Q>,
        best_solution_presentation: String,
        seq_id: u64,
    ) -> Self
    where
        T: Clone,
        Q: Clone,
    {
        Self::new(
            seq_id,
            snapshot.iteration,
            snapshot.evaluations,
            snapshot.best_fitness,
            snapshot.average_fitness,
            snapshot.worst_fitness,
            best_solution_presentation,
        )
    }
}

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
        state: ObserverState,
    },
    /// Algorithm has finished
    End {
        total_generations: usize,
        total_evaluations: usize,
        termination_reason: Option<TerminationReason>,
    },
    /// Algorithm has failed before finishing
    Failed {
        total_generations: usize,
        total_evaluations: usize,
        termination_reason: Option<TerminationReason>,
        error_message: String,
    },
    _Phantom(std::marker::PhantomData<(T, Q)>),
}
