use crate::algorithms::termination::{
    ExecutionStateSnapshot,
    ImprovementDirection,
    TerminationController,
    TerminationCriteria,
    TerminationReason,
};
use crate::observer::traits::AlgorithmObserver;
use crate::observer::AlgorithmEvent;
use crate::solution::traits::{QualityValue, ScalarQuality};
use std::cell::RefCell;
use std::panic::{self, AssertUnwindSafe};
use std::sync::mpsc::{self, Sender};
use std::thread::{self, JoinHandle};

/// Message exchanged between algorithm workers and observer dispatcher.
enum ObserverMessage<T, Q>
where
    T: Clone + Send + 'static,
    Q: Clone + QualityValue + Send + 'static,
{
    Event(AlgorithmEvent<T, Q>),
    Shutdown,
}

/// Optional sender used by algorithms to dispatch observer events.
type ObserverSender<T, Q> = Option<Sender<ObserverMessage<T, Q>>>;

/// Execution context passed to algorithm routines.
///
/// It encapsulates event emission and keeps algorithm logic decoupled from
/// channel internals.
#[derive(Clone)]
pub struct ExecutionContext<T, Q = ScalarQuality>
where
    T: Clone + Send + 'static,
    Q: Clone + QualityValue + Send + 'static,
{
    sender: ObserverSender<T, Q>,
    termination: RefCell<TerminationController>,
    next_snapshot_seq: RefCell<u64>,
}

impl<T, Q> ExecutionContext<T, Q>
where
    T: Clone + Send + 'static,
    Q: Clone + QualityValue + Send + 'static,
{
    /// Creates a new execution context.
    fn new(
        sender: ObserverSender<T, Q>,
        criteria: TerminationCriteria,
        direction: ImprovementDirection,
    ) -> Self {
        Self {
            sender,
            termination: RefCell::new(TerminationController::new(criteria, direction)),
            next_snapshot_seq: RefCell::new(0),
        }
    }

    /// Emits algorithm start event.
    pub fn start(&self, algorithm_name: impl Into<String>) {
        emit_event(
            &self.sender,
            AlgorithmEvent::Start {
                algorithm_name: algorithm_name.into(),
            },
        );
    }

    /// Emits algorithm error event.
    pub fn error(&self, message: impl Into<String>) {
        emit_event(
            &self.sender,
            AlgorithmEvent::Error {
                message: message.into(),
            },
        );
    }

    /// Emits end event with currently known termination reason.
    pub fn end(&self, total_generations: usize, total_evaluations: usize) {
        emit_event(
            &self.sender,
            AlgorithmEvent::End {
                total_generations,
                total_evaluations,
                termination_reason: self.termination_reason(),
            },
        );
    }

    /// Applies one execution snapshot and emits events accordingly.
    pub fn report_progress(&self, snapshot: ExecutionStateSnapshot<T, Q>) {
        let seq_id = {
            let mut next = self.next_snapshot_seq.borrow_mut();
            let id = *next;
            *next = next.saturating_add(1);
            id
        };

        let snapshot = ExecutionStateSnapshot::new(
            seq_id,
            snapshot.iteration,
            snapshot.evaluations,
            snapshot.best_solution,
            snapshot.average_fitness,
            snapshot.worst_fitness,
        );

        self.termination.borrow_mut().on_snapshot(&snapshot);
        emit_event(
            &self.sender,
            AlgorithmEvent::ExecutionStateUpdated { state: snapshot },
        );
    }

    /// Returns `true` when any configured termination criterion has been met.
    pub fn should_terminate(&self) -> bool {
        self.termination.borrow_mut().should_terminate()
    }

    /// Returns the terminal reason if a criterion has already been triggered.
    pub fn termination_reason(&self) -> Option<TerminationReason> {
        self.termination.borrow().reason().cloned()
    }

}

/// Channel-based observer runtime.
///
/// If observers are present, this runtime spawns a dedicated listener thread
/// that receives events through a channel and updates all observers.
struct ObserverRuntime<T, Q>
where
    T: Clone + Send + 'static,
    Q: Clone + QualityValue + Send + 'static,
{
    sender: ObserverSender<T, Q>,
    handle: Option<JoinHandle<Vec<Box<dyn AlgorithmObserver<T, Q>>>>>,
}

impl<T, Q> ObserverRuntime<T, Q>
where
    T: Clone + Send + 'static,
    Q: Clone + QualityValue + Send + 'static,
{
    /// Creates the observer dispatcher thread if at least one observer exists.
    pub fn new(mut observers: Vec<Box<dyn AlgorithmObserver<T, Q>>>) -> Self {
        if observers.is_empty() {
            return Self {
                sender: None,
                handle: None,
            };
        }

        let (tx, rx) = mpsc::channel::<ObserverMessage<T, Q>>();
        let handle = thread::spawn(move || {
            while let Ok(message) = rx.recv() {
                match message {
                    ObserverMessage::Event(event) => {
                        for observer in observers.iter_mut() {
                            observer.update(&event);
                        }
                    }
                    ObserverMessage::Shutdown => break,
                }
            }

            for observer in observers.iter_mut() {
                observer.finalize();
            }

            observers
        });

        Self {
            sender: Some(tx),
            handle: Some(handle),
        }
    }

    /// Returns a cloneable sender for broadcasting observer events.
    pub fn sender(&self) -> ObserverSender<T, Q> {
        self.sender.as_ref().cloned()
    }

    /// Stops the observer thread and returns the updated observers.
    pub fn finish(mut self) -> Vec<Box<dyn AlgorithmObserver<T, Q>>> {
        if let Some(sender) = self.sender.take() {
            let _ = sender.send(ObserverMessage::Shutdown);
        }

        if let Some(handle) = self.handle.take() {
            return handle.join().unwrap_or_default();
        }

        Vec::new()
    }
}

/// Sends an event to the observer runtime if a sender exists.
fn emit_event<T, Q>(sender: &ObserverSender<T, Q>, event: AlgorithmEvent<T, Q>)
where
    T: Clone + Send + 'static,
    Q: Clone + QualityValue + Send + 'static,
{
    if let Some(tx) = sender {
        let _ = tx.send(ObserverMessage::Event(event));
    }
}

/// Runs algorithm work with observer runtime.
///
/// The algorithm task itself runs in the current thread. If observers are
/// present, a dedicated observer thread is spawned to consume events.
///
/// When there are no observers, the task still receives a valid execution
/// context with mandatory termination configuration.
pub fn run_with_observers<T, Q, R, F>(
    observers: Vec<Box<dyn AlgorithmObserver<T, Q>>>,
    criteria: TerminationCriteria,
    direction: ImprovementDirection,
    task: F,
) -> (R, Vec<Box<dyn AlgorithmObserver<T, Q>>>)
where
    T: Clone + Send + 'static,
    Q: Clone + QualityValue + Send + 'static,
    F: FnOnce(ExecutionContext<T, Q>) -> R,
{
    if observers.is_empty() {
        let result = task(ExecutionContext::new(None, criteria, direction));
        return (result, Vec::new());
    }

    let runtime = ObserverRuntime::new(observers);
    let context = ExecutionContext::new(runtime.sender(), criteria, direction);

    let worker_result = panic::catch_unwind(AssertUnwindSafe(|| task(context)));

    let observers = runtime.finish();

    match worker_result {
        Ok(result) => (result, observers),
        Err(payload) => panic::resume_unwind(payload),
    }
}
