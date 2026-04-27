use std::fmt::Display;

use crate::algorithms::objective::ImprovementDirection;
use crate::algorithms::runtime::{
    run_with_observer_runtime, ExecutionContext, RuntimeExecutionOutput,
};
use crate::algorithms::termination::{ExecutionStateSnapshot, TerminationCriteria};
use crate::observer::ObserverState;
use crate::observer::traits::AlgorithmObserver;
use crate::problem::traits::Problem;
use crate::solution::traits::Dominance;
use crate::solution_set::traits::SolutionSet;
use crate::utils::checkpoint::{
    checkpoint_identity_hashes, delete_snapshot_on_success, generate_run_id,
    write_snapshot, StepStateCheckpoint, CheckpointPathConfig,
    CheckpointRecord,
    DEFAULT_FREQUENCY_OF_CHECKPOINT_WRITES, resolve_checkpoint_dir, select_resume_checkpoint,
};
use crate::utils::cli::{has_flag, resolve_path_from_flag_or_default};

const RESUME_FLAG: &str = "--resume";
const NO_CHECKPOINT_FLAG: &str = "--no-checkpoint";
const NO_CHECKPOINT_FLAG_SHORT: &str = "--nc";
const CHECKPOINT_DIR_FLAG: &str = "--checkpoint-dir";

use std::sync::{Mutex, LazyLock};
pub static CONSOLE_LOCK: LazyLock<Mutex<()>> = LazyLock::new(|| Mutex::new(()));

/// Basic interface for all optimization algorithms.
///
/// All algorithms execute with the same step-based lifecycle:
/// initialize state -> report initial snapshot -> loop step/snapshot until
/// termination -> finalize to a solution set.
pub trait Algorithm<T, Q = f64>
where
    T: Clone + Send + 'static + Display,
    Q: Clone + Default + Dominance + Send + 'static + Display,
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
        let direction: ImprovementDirection = problem.get_improvement_direction();
        let algorithm = &*self;
        let problem_description = problem.get_problem_description();
        let problem_parameters = problem.get_problem_parameters_payload(); 
        let (algorithm_signature_hash, problem_signature_hash) = checkpoint_identity_hashes(
            &algorithm_name,
            &algorithm_parameters,
            &problem_description,
            &problem_parameters,
        );        

        // Configuration for checkpoint resumption
        let default_dir = resolve_checkpoint_dir(&CheckpointPathConfig::default());
        let checkpoint_dir =
                resolve_path_from_flag_or_default(CHECKPOINT_DIR_FLAG, default_dir.clone());
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
            direction,
            algorithm_name.clone(),
            move |context| {
                
                
                let initial_snapshot = algorithm.build_snapshot(&state);
                context.update_execution_state(&initial_snapshot);

                let mut last_iteration = initial_snapshot.iteration;
                let mut last_evaluations = initial_snapshot.evaluations;

                let initial_record = state.build_checkpoint_record(&run_id, &algorithm_name, &algorithm_parameters, &problem_description, &problem_parameters, algorithm_signature_hash, problem_signature_hash, context.time_elapsed());
                if !no_checkpoint {
                    if let Ok(path) = write_snapshot(&checkpoint_dir, &initial_record) {
                        last_checkpoint_path = Some(path);
                    }
                }

                // Report initial snapshot to observers before starting iterations
                let initial_presentation = problem.format_solution(&initial_snapshot.best_solution);
                context.report_progress(ObserverState::from_snapshot(
                    initial_snapshot,
                    initial_presentation,
                    context.seq_id(),
                ));

                while !context.should_terminate() {
                    algorithm.step(problem, &mut state, context);

                    let step_snapshot = algorithm.build_snapshot(&state );
                    context.update_execution_state(&step_snapshot);
                    let step_presentation = problem.format_solution(&step_snapshot.best_solution);
                    last_iteration = step_snapshot.iteration;
                    last_evaluations = step_snapshot.evaluations;

                    if DEFAULT_FREQUENCY_OF_CHECKPOINT_WRITES > 0
                        && step_snapshot.iteration % DEFAULT_FREQUENCY_OF_CHECKPOINT_WRITES == 0
                    {

                        let record = state.build_checkpoint_record(&run_id, &algorithm_name, &algorithm_parameters, &problem_description, &problem_parameters, algorithm_signature_hash, problem_signature_hash, context.time_elapsed());

                        if !no_checkpoint {
                            if let Ok(path) = write_snapshot(&checkpoint_dir, &record) {
                                last_checkpoint_path = Some(path);
                            }
                        }
                    }

                    context.report_progress(ObserverState::from_snapshot(
                        step_snapshot,
                        step_presentation,
                        context.seq_id(),
                    ));
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

    fn initialize_step_state(
        &self,
        problem: &(impl Problem<T, Q> + Sync),
    ) -> Self::StepState;

    fn step(
        &self,
        problem: &(impl Problem<T, Q> + Sync),
        state: &mut Self::StepState,
        context: &ExecutionContext<T, Q>,
    );

    fn build_snapshot(&self, state: &Self::StepState) -> ExecutionStateSnapshot<T, Q>;

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
    fn get_resume_checkpoint(&self, problem: &(impl Problem<T, Q> + Sync), checkpoint_dir: &std::path::PathBuf) -> Option<CheckpointRecord> {
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
        ){
            Ok(Some(record)) => Some(record),
            Ok(None) => {
                if let Ok(_lock) = CONSOLE_LOCK.lock() {    // In parallel experiments, sometimes dont show this message (Dont a problem, just a UX detail)
                    eprintln!("No resumable checkpoints found for this algorithm and problem.");
                }
                None
            }
            Err(err) => {
                if let Ok(_lock) = CONSOLE_LOCK.lock() {    // In parallel experiments, sometimes dont show this message (Dont a problem, just a UX detail)
                    eprintln!("Error while searching for checkpoint: {}", err);
                }
                None
            }
        }
        
    }
}
