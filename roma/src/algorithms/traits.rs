use crate::algorithms::objective::ImprovementDirection;
use crate::algorithms::runtime::{run_algorithm, ExecutionContext};
use crate::algorithms::termination::{ExecutionStateSnapshot, TerminationCriteria};
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

        let result = run_algorithm(
            &mut observers,
            criteria,
            direction,
            algorithm_name,
            |context| algorithm.initialize_step_state(problem, context),
            |state, context| algorithm.step(problem, state, context),
            |state| algorithm.snapshot(state),
            |state| algorithm.finalize_step_state(state),
            |solution| problem.format_solution(solution),
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
