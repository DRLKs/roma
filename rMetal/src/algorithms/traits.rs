use crate::algorithms::runtime::{run_with_observer_runtime, ExecutionContext, RuntimeExecutionOutput};
use crate::algorithms::termination::{ExecutionStateSnapshot, ImprovementDirection, TerminationCriteria};
use crate::observer::traits::AlgorithmObserver;
use crate::problem::traits::Problem;
use crate::solution::traits::Dominance;
use crate::solution_set::traits::SolutionSet;

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

    /// Human-readable algorithm name used by observers and runtime reports.
    fn algorithm_name(&self) -> &str;

    /// Termination criteria configured for this algorithm instance.
    fn termination_criteria(&self) -> TerminationCriteria;

    /// Objective direction used by termination logic.
    fn improvement_direction(&self) -> ImprovementDirection;

    /// Mutable access to registered observers.
    fn observers_mut(&mut self) -> &mut Vec<Box<dyn AlgorithmObserver<T, Q>>>;

    /// Stores the last solution set produced by `run`.
    fn set_solution_set(&mut self, solution_set: Self::SolutionSet);

    /// Runs the optimization algorithm on the given problem.
    ///
    /// Default implementation shared by all algorithms.
    fn run(&mut self, problem: &(impl Problem<T, Q> + Sync)) -> Result<Self::SolutionSet, String>
    where
        Self: Sized,
        Self::SolutionSet: Clone,
    {
        self.validate_parameters()?;

        let mut observers = std::mem::take(self.observers_mut());
        let algorithm_name = self.algorithm_name().to_string();
        let criteria = self.termination_criteria();
        let direction = self.improvement_direction();
        let algorithm = &*self;

        let result = run_with_observer_runtime(
            &mut observers,
            criteria,
            direction,
            algorithm_name,
            |context| {
                let mut state = algorithm.initialize_step_state(problem, context);

                let initial_snapshot = algorithm.snapshot(&state);
                let mut last_iteration = initial_snapshot.iteration;
                let mut last_evaluations = initial_snapshot.evaluations;
                context.report_progress(initial_snapshot);

                while !context.should_terminate() {
                    algorithm.step(problem, &mut state, context);

                    let step_snapshot = algorithm.snapshot(&state);
                    last_iteration = step_snapshot.iteration;
                    last_evaluations = step_snapshot.evaluations;
                    context.report_progress(step_snapshot);
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
        context: &ExecutionContext<T, Q>,
    ) -> Self::StepState;

    fn step(
        &self,
        problem: &(impl Problem<T, Q> + Sync),
        state: &mut Self::StepState,
        context: &ExecutionContext<T, Q>,
    );

    fn snapshot(&self, state: &Self::StepState) -> ExecutionStateSnapshot<T, Q>;

    fn finalize_step_state(&self, state: Self::StepState) -> Self::SolutionSet;
}