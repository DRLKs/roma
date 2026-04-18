use crate::algorithms::objective::ImprovementDirection;
use crate::algorithms::termination::{
    ExecutionStateSnapshot, TerminationController, TerminationCriteria, TerminationReason,
};
use crate::algorithms::traits::Algorithm;
use crate::observer::traits::AlgorithmObserver;
use crate::observer::{AlgorithmEvent, ObserverState};
use crate::problem::traits::Problem;
use crate::solution::Solution;
use crate::utils::checkpoint::{
    checkpoint_file_path, initialize_checkpoint_dir, write_checkpoint_record, CheckpointInitMode,
    CheckpointPathConfig, CheckpointRecord, CheckpointRunStatus,
};
use std::any::Any;
use std::cell::RefCell;
use std::panic::{self, AssertUnwindSafe};
use std::path::PathBuf;
use std::sync::mpsc::{self, Sender};
use std::sync::Arc;
use std::thread::{self, JoinHandle};
use std::time::{SystemTime, UNIX_EPOCH};

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
    checkpoint_dir: Option<PathBuf>,
    checkpoint_run_id: String,
    checkpoint_algorithm_name: String,
    checkpoint_problem_fingerprint: String,
    checkpoint_state_payload: RefCell<Option<String>>,
    last_observer_state: RefCell<Option<ObserverState>>,
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
        checkpoint_run_id: String,
        checkpoint_algorithm_name: String,
        checkpoint_problem_fingerprint: String,
    ) -> Self {
        let checkpoint_dir = initialize_checkpoint_dir(
            &CheckpointPathConfig::default(),
            CheckpointInitMode::BestEffort,
        )
        .ok()
        .and_then(|result| result.directory);

        Self {
            sender,
            termination: RefCell::new(TerminationController::new(criteria, direction)),
            next_snapshot_seq: RefCell::new(0),
            checkpoint_dir,
            checkpoint_run_id,
            checkpoint_algorithm_name,
            checkpoint_problem_fingerprint,
            checkpoint_state_payload: RefCell::new(None),
            last_observer_state: RefCell::new(None),
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

        self.write_terminal_checkpoint(
            total_generations,
            total_evaluations,
            CheckpointRunStatus::Completed,
            None,
        );
    }

    pub fn fail(
        &self,
        total_generations: usize,
        total_evaluations: usize,
        error_message: impl Into<String>,
    ) {
        let error_message = error_message.into();
        emit_event(
            &self.sender,
            AlgorithmEvent::Failed {
                total_generations,
                total_evaluations,
                termination_reason: self.termination_reason(),
                error_message: error_message.clone(),
            },
        );

        self.write_terminal_checkpoint(
            total_generations,
            total_evaluations,
            CheckpointRunStatus::Failed,
            Some(error_message),
        );
    }

    pub fn interrupted(&self, total_generations: usize, total_evaluations: usize) {
        self.write_terminal_checkpoint(
            total_generations,
            total_evaluations,
            CheckpointRunStatus::Interrupted,
            Some("run ended without explicit End event".to_string()),
        );
    }

    /// Applies one execution snapshot and emits events accordingly.
    pub fn report_progress(&self, observer_state: ObserverState) {
        self.last_observer_state
            .borrow_mut()
            .replace(observer_state.clone());
        self.write_checkpoint(&observer_state, CheckpointRunStatus::Running, None);

        emit_event(
            &self.sender,
            AlgorithmEvent::ExecutionStateUpdated {
                state: observer_state,
            },
        );
    }

    pub fn set_checkpoint_payload(&self, payload: Option<String>) {
        self.checkpoint_state_payload
            .borrow_mut()
            .clone_from(&payload);
    }

    fn next_snapshot_seq_id(&self) -> u64 {
        let mut next = self.next_snapshot_seq.borrow_mut();
        let id = *next;
        *next = next.saturating_add(1);
        id
    }

    pub fn snapshot_with_seq(
        &self,
        mut snapshot: ExecutionStateSnapshot<T, Q>,
    ) -> ExecutionStateSnapshot<T, Q> {
        snapshot.seq_id = self.next_snapshot_seq_id();
        self.termination.borrow_mut().on_snapshot(&snapshot);
        snapshot
    }

    /// Returns `true` when any configured termination criterion has been met.
    pub fn should_terminate(&self) -> bool {
        self.termination.borrow_mut().should_terminate()
    }

    /// Returns the terminal reason if a criterion has already been triggered.
    pub fn termination_reason(&self) -> Option<TerminationReason> {
        self.termination.borrow().reason().cloned()
    }

    pub fn current_progress(&self) -> (usize, usize) {
        let termination = self.termination.borrow();
        (
            termination.current_iterations(),
            termination.current_evaluations(),
        )
    }

    fn write_terminal_checkpoint(
        &self,
        total_generations: usize,
        total_evaluations: usize,
        status: CheckpointRunStatus,
        error_message: Option<String>,
    ) {
        let state = self
            .last_observer_state
            .borrow()
            .clone()
            .unwrap_or_else(|| {
                ObserverState::new(
                    0,
                    total_generations,
                    total_evaluations,
                    0.0,
                    0.0,
                    0.0,
                    String::new(),
                )
            });

        self.write_checkpoint(&state, status, error_message);
    }

    fn write_checkpoint(
        &self,
        state: &ObserverState,
        status: CheckpointRunStatus,
        error_message: Option<String>,
    ) {
        let Some(checkpoint_dir) = &self.checkpoint_dir else {
            return;
        };

        let path = checkpoint_file_path(checkpoint_dir, &self.checkpoint_run_id, state.seq_id);
        let record = CheckpointRecord {
            run_id: self.checkpoint_run_id.clone(),
            algorithm_name: self.checkpoint_algorithm_name.clone(),
            problem_fingerprint: self.checkpoint_problem_fingerprint.clone(),
            seq_id: state.seq_id,
            iteration: state.iteration,
            evaluations: state.evaluations,
            best_fitness: state.best_fitness,
            average_fitness: state.average_fitness,
            worst_fitness: state.worst_fitness,
            best_solution_presentation: state.best_solution_presentation.clone(),
            state_payload: self.checkpoint_state_payload.borrow().clone(),
            status,
            error_message,
        };

        let _ = write_checkpoint_record(&path, &record);
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
/// - `End` on success,
/// - `Failed` if the task panics.
pub fn run_with_observer_runtime<T, Q, R, F>(
    observers: &mut Vec<Box<dyn AlgorithmObserver<T, Q>>>,
    criteria: TerminationCriteria,
    direction: ImprovementDirection,
    algorithm_name: impl Into<String>,
    problem_fingerprint: impl Into<String>,
    task: F,
) -> R
where
    T: Clone + Send + 'static,
    Q: Clone + Send + 'static,
    F: FnOnce(&ExecutionContext<T, Q>) -> RuntimeExecutionOutput<R>,
{
    let algorithm_name = algorithm_name.into();
    let problem_fingerprint = problem_fingerprint.into();
    let checkpoint_run_id = format!(
        "{}-{}-{}",
        algorithm_name,
        std::process::id(),
        SystemTime::now()
            .duration_since(UNIX_EPOCH)
            .map(|duration| duration.as_millis())
            .unwrap_or(0)
    );

    if observers.is_empty() {
        let context = ExecutionContext::new(
            None,
            criteria,
            direction,
            checkpoint_run_id,
            algorithm_name.clone(),
            problem_fingerprint.clone(),
        );
        context.start(algorithm_name);

        let task_result = panic::catch_unwind(AssertUnwindSafe(|| task(&context)));
        match task_result {
            Ok(output) => {
                context.end(output.total_generations, output.total_evaluations);
                return output.result;
            }
            Err(payload) => {
                let (iterations, evaluations) = context.current_progress();
                context.fail(
                    iterations,
                    evaluations,
                    panic_payload_message(payload.as_ref()),
                );
                panic::resume_unwind(payload);
            }
        }
    }

    let runtime = ObserverRuntime::new(std::mem::take(observers));
    let context = ExecutionContext::new(
        runtime.sender(),
        criteria,
        direction,
        checkpoint_run_id,
        algorithm_name.clone(),
        problem_fingerprint,
    );
    context.start(algorithm_name);

    let task_result = panic::catch_unwind(AssertUnwindSafe(|| task(&context)));
    let mut panic_payload: Option<Box<dyn Any + Send>> = None;
    let result = match task_result {
        Ok(output) => {
            context.end(output.total_generations, output.total_evaluations);
            Ok(output.result)
        }
        Err(payload) => {
            let (iterations, evaluations) = context.current_progress();
            context.fail(
                iterations,
                evaluations,
                panic_payload_message(payload.as_ref()),
            );
            panic_payload = Some(payload);
            Err(())
        }
    };

    // Must be dropped before joining runtime so the channel closes and observer
    // thread can drain and terminate.
    drop(context);

    let observers_after = runtime.finish();
    *observers = observers_after;

    if let Some(payload) = panic_payload {
        panic::resume_unwind(payload);
    }

    match result {
        Ok(value) => value,
        Err(()) => unreachable!("panic payload should have been resumed"),
    }
}

fn panic_payload_message(payload: &(dyn std::any::Any + Send)) -> String {
    if let Some(message) = payload.downcast_ref::<&str>() {
        return (*message).to_string();
    }
    if let Some(message) = payload.downcast_ref::<String>() {
        return message.clone();
    }
    "panic without message".to_string()
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
    T: Clone + Send + 'static,
    Q: Clone + Default + crate::solution::traits::Dominance + Send + 'static,
    A: Algorithm<T, Q> + Send + 'static,
    A::SolutionSet: Clone + Send + 'static,
    P: Problem<T, Q> + Sync + Send + 'static,
{
    thread::spawn(move || {
        let result = algorithm.run(problem.as_ref());
        (algorithm, result)
    })
}

/// Executes the common step-based algorithm lifecycle using runtime observers.
pub(crate) fn run_algorithm<
    T,
    Q,
    S,
    R,
    Initialize,
    Step,
    Snapshot,
    Finalize,
    CheckpointPayload,
    RenderSolution,
>(
    observers: &mut Vec<Box<dyn AlgorithmObserver<T, Q>>>,
    criteria: TerminationCriteria,
    direction: ImprovementDirection,
    algorithm_name: impl Into<String>,
    problem_fingerprint: impl Into<String>,
    initialize: Initialize,
    mut step: Step,
    snapshot: Snapshot,
    finalize: Finalize,
    checkpoint_payload: CheckpointPayload,
    render_solution: RenderSolution,
) -> R
where
    T: Clone + Send + 'static,
    Q: Clone + Send + 'static,
    Initialize: FnOnce(&ExecutionContext<T, Q>) -> S,
    Step: FnMut(&mut S, &ExecutionContext<T, Q>),
    Snapshot: Fn(&S) -> ExecutionStateSnapshot<T, Q>,
    Finalize: FnOnce(S) -> R,
    CheckpointPayload: Fn(&S) -> Option<String>,
    RenderSolution: Fn(&Solution<T, Q>) -> String,
{
    run_with_observer_runtime(
        observers,
        criteria,
        direction,
        algorithm_name,
        problem_fingerprint,
        move |context| {
            let mut state = initialize(context);

            let initial_snapshot = context.snapshot_with_seq(snapshot(&state));
            let initial_presentation = render_solution(&initial_snapshot.best_solution);
            let mut last_iteration = initial_snapshot.iteration;
            let mut last_evaluations = initial_snapshot.evaluations;
            context.set_checkpoint_payload(checkpoint_payload(&state));
            context.report_progress(ObserverState::from_snapshot(
                initial_snapshot,
                initial_presentation,
            ));

            while !context.should_terminate() {
                step(&mut state, context);

                let step_snapshot = snapshot(&state);
                let step_snapshot = context.snapshot_with_seq(step_snapshot);
                let step_presentation = render_solution(&step_snapshot.best_solution);
                last_iteration = step_snapshot.iteration;
                last_evaluations = step_snapshot.evaluations;
                context.set_checkpoint_payload(checkpoint_payload(&state));
                context.report_progress(ObserverState::from_snapshot(
                    step_snapshot,
                    step_presentation,
                ));
            }

            RuntimeExecutionOutput::new(finalize(state), last_iteration, last_evaluations)
        },
    )
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::algorithms::termination::TerminationCriterion;
    use crate::observer::traits::AlgorithmObserver;
    use crate::observer::AlgorithmEvent;
    use crate::solution::RealSolutionBuilder;
    use std::sync::{Arc, Mutex};

    fn snapshot(
        iteration: usize,
        evaluations: usize,
        best_fitness: f64,
    ) -> ExecutionStateSnapshot<f64> {
        let best_solution = RealSolutionBuilder::new(1)
            .set_variable(0, 0.0)
            .with_quality(best_fitness)
            .build();

        ExecutionStateSnapshot::new(
            0,
            iteration,
            evaluations,
            best_solution,
            best_fitness,
            best_fitness,
            best_fitness,
        )
    }

    #[test]
    fn snapshot_with_seq_updates_termination_state() {
        let criteria = TerminationCriteria::new(vec![TerminationCriterion::MaxIterations(2)]);
        let context: ExecutionContext<f64> = ExecutionContext::new(
            None,
            criteria,
            ImprovementDirection::Maximize,
            "runtime-test-run".to_string(),
            "RuntimeTest".to_string(),
            "runtime-problem".to_string(),
        );

        let initial = context.snapshot_with_seq(snapshot(0, 1, 1.0));
        assert_eq!(initial.seq_id, 0);
        assert!(!context.should_terminate());

        let progressed = context.snapshot_with_seq(snapshot(2, 3, 1.2));
        assert_eq!(progressed.seq_id, 1);

        assert!(context.should_terminate());
    }

    struct CaptureObserver {
        events: Arc<Mutex<Vec<String>>>,
    }

    impl AlgorithmObserver<f64> for CaptureObserver {
        fn update(&mut self, event: &AlgorithmEvent<f64>) {
            let mut events = self.events.lock().expect("events lock should be available");
            match event {
                AlgorithmEvent::Start { .. } => events.push("Start".to_string()),
                AlgorithmEvent::ExecutionStateUpdated { .. } => {
                    events.push("ExecutionStateUpdated".to_string())
                }
                AlgorithmEvent::End { .. } => events.push("End".to_string()),
                AlgorithmEvent::Failed { error_message, .. } => {
                    events.push(format!("Failed:{}", error_message))
                }
                AlgorithmEvent::_Phantom(_) => {}
            }
        }

        fn name(&self) -> &str {
            "CaptureObserver"
        }
    }

    #[test]
    fn runtime_emits_failed_event_when_task_panics() {
        let events = Arc::new(Mutex::new(Vec::<String>::new()));
        let observer = CaptureObserver {
            events: Arc::clone(&events),
        };
        let mut observers: Vec<Box<dyn AlgorithmObserver<f64>>> = vec![Box::new(observer)];

        let criteria = TerminationCriteria::new(vec![TerminationCriterion::MaxIterations(10)]);

        let result: Result<(), Box<dyn Any + Send>> = panic::catch_unwind(AssertUnwindSafe(|| {
            run_with_observer_runtime::<f64, f64, (), _>(
                &mut observers,
                criteria,
                ImprovementDirection::Maximize,
                "PanickingDemo",
                "runtime-test-problem",
                |_context| -> RuntimeExecutionOutput<()> {
                    panic!("injected panic for runtime failure event");
                },
            )
        }));

        assert!(result.is_err());

        let events = events.lock().expect("events lock should be available");
        assert!(events.iter().any(|event| event == "Start"));
        assert!(events
            .iter()
            .any(|event| event.contains("Failed:injected panic for runtime failure event")));
        assert!(!events.iter().any(|event| event == "End"));
    }
}
