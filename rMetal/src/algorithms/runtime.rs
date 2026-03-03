use crate::algorithms::termination::{
    ImprovementDirection,
    TerminationController,
    TerminationCriteria,
    TerminationReason,
};
use crate::observer::traits::AlgorithmObserver;
use crate::observer::AlgorithmEvent;
use std::cell::RefCell;
use std::panic::{self, AssertUnwindSafe};
use std::sync::mpsc::{self, Sender};
use std::thread::{self, JoinHandle};

/// Message exchanged between algorithm workers and observer dispatcher.
enum ObserverMessage<T>
where
    T: Clone + Send + 'static,
{
    Event(AlgorithmEvent<T>),
    Shutdown,
}

/// Optional sender used by algorithms to dispatch observer events.
type ObserverSender<T> = Option<Sender<ObserverMessage<T>>>;

/// Execution context passed to algorithm routines.
///
/// It encapsulates event emission and keeps algorithm logic decoupled from
/// channel internals.
#[derive(Clone)]
pub struct ExecutionContext<T>
where
    T: Clone + Send + 'static,
{
    sender: ObserverSender<T>,
    termination: RefCell<TerminationController>,
}

impl<T> ExecutionContext<T>
where
    T: Clone + Send + 'static,
{
    /// Creates a new execution context.
    ///
    /// Termination configuration is mandatory to keep run semantics consistent
    /// across all algorithms.
    fn new(
        sender: ObserverSender<T>,
        criteria: TerminationCriteria,
        direction: ImprovementDirection,
    ) -> Self {
        Self {
            sender,
            termination: RefCell::new(TerminationController::new(criteria, direction)),
        }
    }

    pub fn emit(&self, event: AlgorithmEvent<T>) {
        if let AlgorithmEvent::ExecutionStateUpdated { state } = &event {
            self.termination.borrow_mut().on_snapshot(state);
        }

        emit_event(&self.sender, event);
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
struct ObserverRuntime<T>
where
    T: Clone + Send + 'static,
{
    sender: ObserverSender<T>,
    handle: Option<JoinHandle<Vec<Box<dyn AlgorithmObserver<T>>>>>,
}

impl<T> ObserverRuntime<T>
where
    T: Clone + Send + 'static,
{
    /// Creates the observer dispatcher thread if at least one observer exists.
    pub fn new(mut observers: Vec<Box<dyn AlgorithmObserver<T>>>) -> Self {
        if observers.is_empty() {
            return Self {
                sender: None,
                handle: None,
            };
        }

        let (tx, rx) = mpsc::channel::<ObserverMessage<T>>();
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
    pub fn sender(&self) -> ObserverSender<T> {
        self.sender.as_ref().cloned()
    }

    /// Stops the observer thread and returns the updated observers.
    pub fn finish(mut self) -> Vec<Box<dyn AlgorithmObserver<T>>> {
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
fn emit_event<T>(sender: &ObserverSender<T>, event: AlgorithmEvent<T>)
where
    T: Clone + Send + 'static,
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
pub fn run_with_observers<T, R, F>(
    observers: Vec<Box<dyn AlgorithmObserver<T>>>,
    criteria: TerminationCriteria,
    direction: ImprovementDirection,
    task: F,
) -> (R, Vec<Box<dyn AlgorithmObserver<T>>>)
where
    T: Clone + Send + 'static,
    F: FnOnce(ExecutionContext<T>) -> R,
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
