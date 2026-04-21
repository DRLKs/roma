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
    CheckpointPathConfig, CheckpointRecord, resolve_checkpoint_dir, select_resume_checkpoint,
};
use crate::utils::cli::{has_flag, resolve_path_from_flag_or_default};

const RESUME_FLAG: &str = "--resume";
const CHECKPOINT_DIR_FLAG: &str = "--checkpoint-dir";

/// Basic interface for all optimization algorithms.
///
/// All algorithms execute with the same step-based lifecycle:
/// initialize state -> report initial snapshot -> loop step/snapshot until
/// termination -> finalize to a solution set.
pub trait Algorithm<T, Q = f64>
where
    T: Clone + Send + 'static,
    Q: Clone + Default + Dominance + Send + 'static,
{
    type SolutionSet: SolutionSet<T, Q>;
    type Parameters;
    type StepState;

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
        let algorithm_name = self.algorithm_name().to_string();
        let criteria = self.termination_criteria();
        let direction: ImprovementDirection = problem.get_improvement_direction();
        let algorithm = &*self;

        // Configuration for checkpoint resumption
        let resume_checkpoint = self.get_resume_checkpoint(problem);

        let result = run_with_observer_runtime(
            &mut observers,
            criteria,
            direction,
            algorithm_name,
            move |context| {
                let mut state = if let Some(checkpoint) = resume_checkpoint.as_ref() {
                    algorithm.initialize_step_state_with_resume(
                        problem,
                        checkpoint,
                    )
                } else {
                    algorithm.initialize_step_state(problem)
                };

                let initial_snapshot = context.snapshot_with_seq(algorithm.snapshot(&state));
                let initial_presentation = problem.format_solution(&initial_snapshot.best_solution);
                let mut last_iteration = initial_snapshot.iteration;
                let mut last_evaluations = initial_snapshot.evaluations;
                context.report_progress(ObserverState::from_snapshot(
                    initial_snapshot,
                    initial_presentation,
                ));

                while !context.should_terminate() {
                    algorithm.step(problem, &mut state, context);

                    let step_snapshot = algorithm.snapshot(&state);
                    let step_snapshot = context.snapshot_with_seq(step_snapshot);
                    let step_presentation = problem.format_solution(&step_snapshot.best_solution);
                    last_iteration = step_snapshot.iteration;
                    last_evaluations = step_snapshot.evaluations;
                    context.report_progress(ObserverState::from_snapshot(
                        step_snapshot,
                        step_presentation,
                    ));
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

    fn snapshot(&self, state: &Self::StepState) -> ExecutionStateSnapshot<T, Q>;

    fn finalize_step_state(&self, state: Self::StepState) -> Self::SolutionSet;

    fn initialize_step_state_with_resume(
        &self,
        problem: &(impl Problem<T, Q> + Sync),
        _checkpoint: &CheckpointRecord,
    ) -> Self::StepState {
        self.initialize_step_state(problem)
    }

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
    fn get_resume_checkpoint(&self, problem: &(impl Problem<T, Q> + Sync)) -> Option<CheckpointRecord> {
        let resume = has_flag(RESUME_FLAG);
        if !resume {
            return None;
        }
        
        let default_dir = resolve_checkpoint_dir(&CheckpointPathConfig::default());
        let checkpoint_dir =
                resolve_path_from_flag_or_default(CHECKPOINT_DIR_FLAG, default_dir.clone());

        let algorithm_parameters = self.checkpoint_algorithm_parameters();
        let problem_description = problem.get_problem_description();
        let problem_parameters = problem.get_problem_description(); // TODO: Obtener parámetros específicos de cada problema

        match select_resume_checkpoint(
            checkpoint_dir.as_path(),
            self.algorithm_name(),
            &algorithm_parameters,
            &problem_description,
            &problem_parameters,
        ){
            Ok(Some(record)) => Some(record),
            Ok(None) => {
                eprintln!("No matching checkpoint found for resumption.");
                None
            }
            Err(err) => {
                eprintln!("Error while searching for checkpoint: {}", err);
                None
            }
        }
        
    }
}
