use crate::algorithms::objective::ImprovementDirection;
use crate::algorithms::termination::{
    ExecutionStateSnapshot, TerminationController, TerminationCriteria, TerminationReason,
};
use crate::algorithms::traits::Algorithm;
use crate::observer::traits::AlgorithmObserver;
use crate::observer::{AlgorithmEvent, ObserverState};
use crate::problem::traits::Problem;
use std::cell::RefCell;
use std::fmt::Display;
use std::sync::mpsc::{self, Sender};
use std::sync::Arc;
use std::thread::{self, JoinHandle};

type ObserverSender<T, Q> = Option<Sender<AlgorithmEvent<T, Q>>>;

/// Output expected from one algorithm execution.
///
/// The runtime emits `Start` and `End` events around the task and uses
/// this structure to complete end-of-run metadata consistently.
pub struct RuntimeExecutionOutput<R> {
    pub result: R,
    pub total_generations: usize,
    pub total_evaluations: usize,
}

impl<R> RuntimeExecutionOutput<R> {
    pub fn new(result: R, total_generations: usize, total_evaluations: usize) -> Self {
        Self {
            result,
            total_generations,
            total_evaluations,
        }
    }
}

/// Execution context passed to algorithm routines.
///
/// It encapsulates event emission and keeps algorithm logic decoupled from
/// channel internals.
pub struct ExecutionContext<T, Q = f64>
where
    T: Clone + Send + 'static,
    Q: Clone + Send + 'static,
{
    sender: ObserverSender<T, Q>,
    termination: RefCell<TerminationController>,
    next_snapshot_seq: RefCell<u64>,
}

impl<T, Q> ExecutionContext<T, Q>
where
    T: Clone + Send + 'static,
    Q: Clone + Send + 'static,
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

    pub fn seq_id(&self) -> u64 {
        *self.next_snapshot_seq.borrow()
    }

    /// Applies one execution snapshot and emits events accordingly.
    pub fn report_progress(&self, observer_state: ObserverState) {
        emit_event(
            &self.sender,
            AlgorithmEvent::ExecutionStateUpdated {
                state: observer_state,
            },
        );
    }

    fn next_snapshot_seq_id(&self) -> u64 {
        let mut next = self.next_snapshot_seq.borrow_mut();
        let id = *next;
        *next = next.saturating_add(1);
        id
    }

    pub fn update_execution_state(
        &self,
        snapshot: &ExecutionStateSnapshot<T, Q>,
    ) {
        self.next_snapshot_seq_id();
        self.termination.borrow_mut().on_snapshot(snapshot);
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
    Q: Clone + Send + 'static,
{
    sender: ObserverSender<T, Q>,
    handle: Option<JoinHandle<Vec<Box<dyn AlgorithmObserver<T, Q>>>>>,
}

impl<T, Q> ObserverRuntime<T, Q>
where
    T: Clone + Send + 'static,
    Q: Clone + Send + 'static,
{
    /// Creates the observer dispatcher thread if at least one observer exists.
    pub fn new(mut observers: Vec<Box<dyn AlgorithmObserver<T, Q>>>) -> Self {
        if observers.is_empty() {
            return Self {
                sender: None,
                handle: None,
            };
        }

        let (tx, rx) = mpsc::channel::<AlgorithmEvent<T, Q>>();
        let handle = thread::spawn(move || {
            while let Ok(event) = rx.recv() {
                for observer in observers.iter_mut() {
                    observer.update(&event);
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
        // Closing the sender causes receiver loop to finish after draining all
        // already queued events.
        self.sender.take();

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
    Q: Clone + Send + 'static,
{
    if let Some(tx) = sender {
        let _ = tx.send(event);
    }
}

/// Runs one algorithm execution with asynchronous observers.
///
/// The algorithm task itself runs in the current thread. If observers are
/// present, a dedicated observer thread is spawned to consume events.
///
/// When there are no observers, the task still receives a valid execution
/// context with mandatory termination configuration.
///
/// Lifecycle emitted by this runtime:
/// - `Start` before invoking task,
/// - `End` on success.
pub fn run_with_observer_runtime<T, Q, R, F>(
    observers: &mut Vec<Box<dyn AlgorithmObserver<T, Q>>>,
    criteria: TerminationCriteria,
    direction: ImprovementDirection,
    algorithm_name: impl Into<String>,
    task: F,
) -> R
where
    T: Clone + Send + 'static,
    Q: Clone + Send + 'static,
    F: FnOnce(&ExecutionContext<T, Q>) -> RuntimeExecutionOutput<R>,
{
    let algorithm_name = algorithm_name.into();

    if observers.is_empty() {
        let context = ExecutionContext::new(None, criteria, direction);
        context.start(algorithm_name);
        let output = task(&context);
        context.end(output.total_generations, output.total_evaluations);
        return output.result;
    }

    let runtime = ObserverRuntime::new(std::mem::take(observers));
    let context = ExecutionContext::new(runtime.sender(), criteria, direction);
    context.start(algorithm_name);

    let output = task(&context);
    context.end(output.total_generations, output.total_evaluations);

    // Must be dropped before joining runtime so the channel closes and observer
    // thread can drain and terminate.
    drop(context);

    let observers_after = runtime.finish();
    *observers = observers_after;

    output.result
}

/// Spawns one algorithm execution in a dedicated thread.
///
/// Returns a join handle that yields the algorithm instance (with its updated
/// internal state, observers and latest solution set) and the run result.
///
/// This helper is intended for coarse-grained asynchronous orchestration, such
/// as launching multiple algorithms or island populations concurrently.
pub fn spawn_algorithm_run<A, T, Q, P>(
    mut algorithm: A,
    problem: Arc<P>,
) -> JoinHandle<(A, Result<A::SolutionSet, String>)>
where
    T: Clone + Send + 'static + Display,
    Q: Clone + Default + crate::solution::traits::Dominance + Send + 'static + Display,
    A: Algorithm<T, Q> + Send + 'static,
    A::SolutionSet: Clone + Send + 'static,
    P: Problem<T, Q> + Sync + Send + 'static,
{
    thread::spawn(move || {
        let result = algorithm.run(problem.as_ref());
        (algorithm, result)
    })
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::algorithms::termination::TerminationCriterion;
    use crate::solution::RealSolutionBuilder;

    fn snapshot(
        iteration: usize,
        evaluations: usize,
        best_fitness: f64,
    ) -> ExecutionStateSnapshot<f64> {
        let best_solution = RealSolutionBuilder::new(1)
            .set_variable(0, 0.0)
            .with_quality(best_fitness)
            .build();

        ExecutionStateSnapshot{
            iteration,
            evaluations,
            best_solution,
            best_fitness,
            worst_fitness: best_fitness,
            average_fitness: best_fitness,
        }
    }

    #[test]
    fn snapshot_with_seq_updates_termination_state() {
        let criteria = TerminationCriteria::new(vec![TerminationCriterion::MaxIterations(2)]);
        let context: ExecutionContext<f64> =
            ExecutionContext::new(None, criteria, ImprovementDirection::Maximize);

        assert_eq!(*context.next_snapshot_seq.borrow(), 0);
        context.update_execution_state(&snapshot(0, 1, 1.0));
        assert_eq!(*context.next_snapshot_seq.borrow(), 1);
        assert!(!context.should_terminate());

        context.update_execution_state(&snapshot(2, 3, 1.2));
        assert_eq!(*context.next_snapshot_seq.borrow(), 2);
        assert!(context.should_terminate());
    }
}
