use std::fmt::Display;
use std::sync::{LazyLock, Mutex};

use crate::algorithms::checkpoint::{
    delete_snapshot_on_success, generate_run_id, select_resume_checkpoint_for_metadata,
    CheckpointPolicy, CheckpointRecord, CheckpointRuntimeMetadata, ExecutionStateSnapshot,
    StepStateCheckpoint, DEFAULT_FREQUENCY_OF_CHECKPOINT_WRITES,
};
use crate::algorithms::runtime::{
    run_with_observer_runtime, RuntimeExecutionOutput,
};
use crate::algorithms::termination::TerminationCriteria;
use crate::observer::traits::AlgorithmObserver;
use crate::observer::ObserverState;
use crate::problem::traits::Problem;
use crate::solution_set::traits::SolutionSet;
use crate::utils::path::CheckpointPathConfig;

pub static CONSOLE_LOCK: LazyLock<Mutex<()>> = LazyLock::new(|| Mutex::new(()));

/// Basic interface for all optimization algorithms.
///
/// All algorithms execute with the same step-based lifecycle:
/// initialize state -> report initial snapshot -> loop step/snapshot until
/// termination -> finalize to a solution set.
pub trait Algorithm<T, Q = f64>
where
    T: Clone + Send + 'static + Display,
    Q: Clone + Default + Send + 'static + Display,
{
    type SolutionSet: SolutionSet<T, Q>;
    type Parameters;
    type StepState: StepStateCheckpoint<T, Q>;

    fn new(parameters: Self::Parameters) -> Self;

    /// Human-readable algorithm name used by observers and runtime reports.
    fn algorithm_name(&self) -> &str;

    /// Termination criteria configured for this algorithm instance.
    fn termination_criteria(&self) -> TerminationCriteria;

    /// Mutable access to registered observers.
    fn observers_mut(&mut self) -> &mut Vec<Box<dyn AlgorithmObserver<T, Q>>>;

    /// Stores the last solution set produced by `run`.
    fn set_solution_set(&mut self, solution_set: Self::SolutionSet);

    /// Runs the optimization algorithm on the given problem.
    ///
    /// Default implementation shared by all algorithms.
    fn run<P>(&mut self, problem: &P) -> Result<Self::SolutionSet, String>
    where
        Self: Sized,
        Self::SolutionSet: Clone,
        P: Problem<T, Q> + Sync,
    {
        self.validate_parameters()?;

        let mut observers = std::mem::take(self.observers_mut());
        let checkpoint_policy = CheckpointPolicy::from_cli(&CheckpointPathConfig::default());

        let algorithm_name = self.algorithm_name().to_string();
        let algorithm_parameters = self.checkpoint_algorithm_parameters();
        let criteria = self.termination_criteria();
        let better_fitness = problem.better_fitness_fn();
        let algorithm = &self;
        let problem_description = problem.get_problem_description();
        let problem_parameters = problem.get_problem_parameters_payload();
        let checkpoint_metadata = CheckpointRuntimeMetadata::new(
            &algorithm_name,
            &algorithm_parameters,
            &problem_description,
            &problem_parameters,
        );

        let resume_checkpoint = self.get_resume_checkpoint(&checkpoint_metadata, &checkpoint_policy);
        let run_id = resume_checkpoint
            .as_ref()
            .map(|record| record.run_id.clone())
            .unwrap_or_else(|| generate_run_id(&algorithm_name));

        let mut state: <Self as Algorithm<T, Q>>::StepState;

        if let Some(checkpoint) = resume_checkpoint.as_ref() {
            state = StepStateCheckpoint::from_payload(&checkpoint.step_state_payload)
        } else {
            state = algorithm.initialize_step_state(problem)
        };

        let mut last_checkpoint_path: Option<std::path::PathBuf> = None;

        let output = run_with_observer_runtime(
            &mut observers,
            criteria,
            better_fitness,
            algorithm_name.clone(),
            move |context| {
                let initial_snapshot = algorithm.build_snapshot(problem, &state);
                context.update_execution_state(&initial_snapshot);

                let mut last_iteration = initial_snapshot.iteration;
                let mut last_evaluations = initial_snapshot.evaluations;

                let initial_record = state.build_checkpoint_record(
                    &run_id,
                    &checkpoint_metadata,
                    context.time_elapsed(),
                );
                checkpoint_policy.persist_record(&mut last_checkpoint_path, &initial_record);

                context.report_progress(ObserverState::from_snapshot(&initial_snapshot, context.seq_id()));


                while !context.should_terminate() {
                    algorithm.step(problem, &mut state);

                    let step_snapshot = algorithm.build_snapshot(problem, &state);
                    context.update_execution_state(&step_snapshot);
                    last_iteration = step_snapshot.iteration;
                    last_evaluations = step_snapshot.evaluations;

                    if DEFAULT_FREQUENCY_OF_CHECKPOINT_WRITES > 0
                        && step_snapshot.iteration % DEFAULT_FREQUENCY_OF_CHECKPOINT_WRITES == 0
                    {
                        let record = state.build_checkpoint_record(
                            &run_id,
                            &checkpoint_metadata,
                            context.time_elapsed(),
                        );
                        checkpoint_policy.persist_record(&mut last_checkpoint_path, &record);
                    }

                    context.report_progress(ObserverState::from_snapshot(&step_snapshot, context.seq_id()));
                }

                if let Some(path) = last_checkpoint_path.as_ref() {
                    let _ = delete_snapshot_on_success(path);
                }

                RuntimeExecutionOutput::new(
                    algorithm.finalize_step_state(state),
                    last_iteration,
                    last_evaluations,
                )
            },
        );
        *self.observers_mut() = observers;

        self.set_solution_set(output.clone());
        Ok(output)
    }

    /// Returns `Ok(())` when parameters are valid, or `Err(message)` otherwise.
    fn validate_parameters(&self) -> Result<(), String> {
        Ok(())
    }

    fn get_solution_set(&self) -> Option<&Self::SolutionSet>;

    fn initialize_step_state(&self, problem: &(impl Problem<T, Q> + Sync)) -> Self::StepState;

    fn step(
        &self,
        problem: &(impl Problem<T, Q> + Sync),
        state: &mut Self::StepState,
    );

    fn build_snapshot(
        &self,
        problem: &(impl Problem<T, Q> + Sync),
        state: &Self::StepState,
    ) -> ExecutionStateSnapshot;

    fn finalize_step_state(&self, state: Self::StepState) -> Self::SolutionSet;

    /// Returns a string used to fingerprint algorithm checkpoint compatibility.
    fn checkpoint_algorithm_parameters(&self) -> String {
        String::new()
    }

    /// Resolves a resumable checkpoint when `--resume` is present.
    fn get_resume_checkpoint(
        &self,
        runtime_metadata: &CheckpointRuntimeMetadata<'_>,
        checkpoint_policy: &CheckpointPolicy,
    ) -> Option<CheckpointRecord> {
        if !checkpoint_policy.resume_requested() {
            return None;
        }

        match select_resume_checkpoint_for_metadata(checkpoint_policy.checkpoint_dir(), runtime_metadata)
        {
            Ok(Some(record)) => Some(record),
            Ok(None) => {
                if let Ok(_lock) = CONSOLE_LOCK.lock() {
                    // In parallel experiments, sometimes dont show this message (Dont a problem, just a UX detail)
                    eprintln!("No resumable checkpoints found for this algorithm and problem.");
                }
                None
            }
            Err(err) => {
                if let Ok(_lock) = CONSOLE_LOCK.lock() {
                    // In parallel experiments, sometimes dont show this message (Dont a problem, just a UX detail)
                    eprintln!("Error while searching for checkpoint: {}", err);
                }
                None
            }
        }
    }
}