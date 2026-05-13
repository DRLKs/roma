use std::fmt::Display;

use crate::algorithms::checkpoint::{
    checkpoint_identity_hashes, delete_snapshot_on_success, generate_run_id,
    select_resume_checkpoint, write_snapshot, CheckpointRecord,
    CheckpointRuntimeMetadata, StepStateCheckpoint, ExecutionStateSnapshot,
    DEFAULT_FREQUENCY_OF_CHECKPOINT_WRITES,
};
use crate::algorithms::runtime::{
    run_with_observer_runtime, ExecutionContext, RuntimeExecutionOutput,
};
use crate::algorithms::termination::{TerminationCriteria};
use crate::observer::traits::AlgorithmObserver;
use crate::observer::ObserverState;
use crate::problem::traits::Problem;
use crate::solution_set::traits::SolutionSet;
use crate::utils::cli::{has_flag, resolve_path_from_flag_or_default};
use crate::utils::path::{
    initialize_checkpoint_dir, resolve_checkpoint_dir, CheckpointInitMode, CheckpointPathConfig,
};

const RESUME_FLAG: &str = "--resume";
const NO_CHECKPOINT_FLAG: &str = "--no-checkpoint";
const NO_CHECKPOINT_FLAG_SHORT: &str = "--nc";
const CHECKPOINT_DIR_FLAG: &str = "--checkpoint-dir";

use std::sync::{LazyLock, Mutex};
pub static CONSOLE_LOCK: LazyLock<Mutex<()>> = LazyLock::new(|| Mutex::new(()));

fn resolve_checkpoint_dir_from_config_for_writes(
    config: &CheckpointPathConfig,
) -> Result<std::path::PathBuf, String> {
    initialize_checkpoint_dir(config, CheckpointInitMode::BestEffort)
        .map_err(|err| format!("failed to initialize checkpoint directory: {}", err))
        .and_then(|result| {
            result.directory.ok_or_else(|| {
                "no writable checkpoint directory available; checkpoint writes disabled".to_string()
            })
        })
}

fn write_checkpoint_if_enabled(
    last_checkpoint_path: &mut Option<std::path::PathBuf>,
    checkpoint_dir: &std::path::Path,
    checkpoint_writes_enabled: bool,
    no_checkpoint: bool,
    record: &CheckpointRecord,
) {
    if !no_checkpoint && checkpoint_writes_enabled {
        if let Ok(path) = write_snapshot(checkpoint_dir, record) {
            *last_checkpoint_path = Some(path);
        }
    }
}

fn report_snapshot<T, Q>(context: &ExecutionContext<T, Q>, snapshot: &ExecutionStateSnapshot)
where
    T: Clone + Send + 'static + Display,
    Q: Clone + Default + Send + 'static + Display,
{
    context.report_progress(ObserverState::from_snapshot(snapshot, context.seq_id()));
}

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

        // let resume = has_flag(RESUME_FLAG);
        let no_checkpoint = has_flag(NO_CHECKPOINT_FLAG) || has_flag(NO_CHECKPOINT_FLAG_SHORT);

        let algorithm_name = self.algorithm_name().to_string();
        let algorithm_parameters = self.checkpoint_algorithm_parameters();
        let criteria = self.termination_criteria();
        let better_fitness = problem.better_fitness_fn();
        let algorithm = &self;
        let problem_description = problem.get_problem_description();
        let problem_parameters = problem.get_problem_parameters_payload();
        let (algorithm_signature_hash, problem_signature_hash) = checkpoint_identity_hashes(
            &algorithm_name,
            &algorithm_parameters,
            &problem_description,
            &problem_parameters,
        );
        let checkpoint_metadata = CheckpointRuntimeMetadata {
            algorithm_name: &algorithm_name,
            algorithm_parameters: &algorithm_parameters,
            problem_description: &problem_description,
            problem_parameters: &problem_parameters,
            algorithm_signature_hash,
            problem_signature_hash,
        };

        // Configuration for checkpoint resumption
        let checkpoint_cfg = CheckpointPathConfig::default();
        let default_dir = resolve_checkpoint_dir(&checkpoint_cfg);
        let checkpoint_dir = resolve_path_from_flag_or_default(CHECKPOINT_DIR_FLAG, default_dir);
        let checkpoint_writes_enabled = if has_flag(CHECKPOINT_DIR_FLAG) {
            std::fs::create_dir_all(&checkpoint_dir).is_ok()
        } else {
            resolve_checkpoint_dir_from_config_for_writes(&checkpoint_cfg).is_ok()
        };
        let resume_checkpoint = self.get_resume_checkpoint(problem, &checkpoint_dir);
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

        let result = run_with_observer_runtime(
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
                write_checkpoint_if_enabled(
                    &mut last_checkpoint_path,
                    &checkpoint_dir,
                    checkpoint_writes_enabled,
                    no_checkpoint,
                    &initial_record,
                );

                // Report initial snapshot to observers before starting iterations
                report_snapshot(context, &initial_snapshot);

                while !context.should_terminate() {
                    algorithm.step(problem, &mut state, context);

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
                        write_checkpoint_if_enabled(
                            &mut last_checkpoint_path,
                            &checkpoint_dir,
                            checkpoint_writes_enabled,
                            no_checkpoint,
                            &record,
                        );
                    }

                    report_snapshot(context, &step_snapshot);
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

        self.set_solution_set(result.clone());
        Ok(result)
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
        context: &ExecutionContext<T, Q>,
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
    ///
    /// This method resolves the checkpoint directory, computes algorithm/problem
    /// identity fingerprints and delegates checkpoint selection to the checkpoint
    /// module. It returns `None` when resume is disabled, no compatible
    /// checkpoint is found, or selection fails.
    fn get_resume_checkpoint(
        &self,
        problem: &(impl Problem<T, Q> + Sync),
        checkpoint_dir: &std::path::PathBuf,
    ) -> Option<CheckpointRecord> {
        let resume = has_flag(RESUME_FLAG);
        if !resume {
            return None;
        }

        let algorithm_parameters = self.checkpoint_algorithm_parameters();
        let problem_description = problem.get_problem_description();
        let problem_parameters = problem.get_problem_parameters_payload(); // TODO: Obtener parámetros específicos de cada problema

        match select_resume_checkpoint(
            checkpoint_dir.as_path(),
            self.algorithm_name(),
            &algorithm_parameters,
            &problem_description,
            &problem_parameters,
        ) {
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

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn resolve_checkpoint_dir_for_writes_skips_unwritable_candidates() {
        let base = std::env::temp_dir().join(format!(
            "roma_traits_checkpoint_dir_test_{}_{}",
            std::process::id(),
            std::time::SystemTime::now()
                .duration_since(std::time::UNIX_EPOCH)
                .map(|d| d.as_nanos())
                .unwrap_or(0)
        ));
        let fallback = base.join("fallback");

        let config = CheckpointPathConfig {
            app_name: "/dev/null".to_string(),
            env_var_name: "ROMA_TEST_UNUSED_CHECKPOINT_ENV",
            explicit_dir: Some(std::path::PathBuf::from("/dev/null/roma")),
            project_fallback_dir: Some(fallback.clone()),
        };

        let resolved = resolve_checkpoint_dir_from_config_for_writes(&config)
            .expect("a writable fallback directory should be resolved");

        assert_eq!(resolved, fallback);
        assert!(resolved.exists());

        let _ = std::fs::remove_dir_all(base);
    }
}
