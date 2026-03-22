use crate::algorithms::termination::{
    ExecutionStateSnapshot,
    TerminationCriteria,
};
use crate::algorithms::traits::Algorithm;
use crate::algorithms::runtime::{
    ExecutionContext,
    ImprovementDirection
};
use crate::experiment::traits::{CaseParameter, ExperimentalCase};
use crate::observer::traits::{AlgorithmObserver, Observable};
use crate::operator::traits::MutationOperator;
use crate::problem::traits::Problem;
use crate::solution::Solution;
use crate::solution_set::implementations::vector_solution_set::VectorSolutionSet;
use crate::solution_set::traits::SolutionSet;
use crate::utils::random::{Random, seed_from_time};

/// Configuration parameters for the Hill Climbing algorithm.
///
/// # Type Parameters
/// - `T`: decision variable type of the solution.
/// - `M`: mutation operator used to generate neighbor solutions.
#[derive(Clone)]
pub struct HillClimbingParameters<T, M>
where
    T: Clone,
    M: MutationOperator<T>,
{
    pub mutation_operator: M,
    pub mutation_probability: f64,
    pub termination_criteria: TerminationCriteria,
    pub random_seed: Option<u64>,
    _phantom: std::marker::PhantomData<T>,
}

impl<T, M> HillClimbingParameters<T, M>
where
    T: Clone,
    M: MutationOperator<T>,
{
    /// Creates a new parameter set.
    ///
    /// # Arguments
    /// - `mutation_operator`: operator used to mutate the current solution.
    /// - `mutation_probability`: per-variable mutation probability in the range `[0.0, 1.0]`.
    /// - `termination_criteria`: criteria to stop the algorithm.
    pub fn new(
        mutation_operator: M,
        mutation_probability: f64,
        termination_criteria: TerminationCriteria,
    ) -> Self {
        Self {
            mutation_operator,
            mutation_probability,
            termination_criteria,
            random_seed: None,
            _phantom: std::marker::PhantomData,
        }
    }

    /// Sets a deterministic random seed for reproducible runs.
    pub fn with_seed(mut self, seed: u64) -> Self {
        self.random_seed = Some(seed);
        self
    }
}



/// Hill Climbing optimization algorithm.
///
/// This implementation keeps one current solution, mutates it to generate a
/// neighbor, and replaces the current solution only when the neighbor is
/// strictly better according to the optimization direction.
///
/// The final result is a `VectorSolutionSet` containing one solution: the best
/// solution found during the run.
///
/// # Type Parameters
/// - `T`: decision variable type of the solution.
/// - `M`: mutation operator used to generate neighbor solutions.
pub struct HillClimbing<T, M>
where
    T: Clone,
    M: MutationOperator<T>,
{
    parameters: HillClimbingParameters<T, M>,
    solution_set: Option<VectorSolutionSet<T>>,
    observers: Vec<Box<dyn AlgorithmObserver<T>>>,
}

impl<T, M> HillClimbing<T, M>
where
    T: Clone,
    M: MutationOperator<T>,
{
    
}

pub struct HillClimbingState<T>
where
    T: Clone,
{
    current: Solution<T>,
    rng: Random,
    iteration: usize,
    evaluations: usize,
}

impl<T, M> Observable<T> for HillClimbing<T, M>
where
    T: Clone + Send + 'static,
    M: MutationOperator<T>,
{
    fn add_observer(&mut self, observer: Box<dyn AlgorithmObserver<T>>) {
        self.observers.push(observer);
    }

    fn clear_observers(&mut self) {
        self.observers.clear();
    }
}

impl<T, M> Algorithm<T> for HillClimbing<T, M>
where
    T: Clone + Send + Sync + 'static,
    M: MutationOperator<T> + Send + Sync,
{
    type SolutionSet = VectorSolutionSet<T>;
    type Parameters = HillClimbingParameters<T, M>;
    type StepState = HillClimbingState<T>;

    /// Creates a new Hill Climbing instance.    /// Creates a new Hill Climbing instance.

    ///
    /// # Arguments
    /// - `parameters`: algorithm configuration.
    /// - `maximization`: `true` for maximization, `false` for minimization.
    fn new(parameters: HillClimbingParameters<T, M>) -> Self {
        Self {
            parameters,
            solution_set: None,
            observers: Vec::new(),
        }
    }

    fn algorithm_name(&self) -> &str {
        "HillClimbing"
    }

    fn termination_criteria(&self) -> TerminationCriteria {
        self.parameters.termination_criteria.clone()
    }

    fn observers_mut(&mut self) -> &mut Vec<Box<dyn AlgorithmObserver<T>>> {
        &mut self.observers
    }

    fn set_solution_set(&mut self, solution_set: Self::SolutionSet) {
        self.solution_set = Some(solution_set);
    }

    /// Validates algorithm parameters.
    ///
    fn validate_parameters(&self) -> Result<(), String> {
        if self.parameters.termination_criteria.is_empty() {
            return Err("termination_criteria must not be empty".to_string());
        }

        if !(0.0..=1.0).contains(&self.parameters.mutation_probability) {
            return Err("mutation_probability must be in [0,1]".to_string());
        }

        Ok(())
    }

    /// Returns the last computed solution set, if the algorithm has been run.
    fn get_solution_set(&self) -> Option<&Self::SolutionSet> {
        self.solution_set.as_ref()
    }

    fn initialize_step_state(
        &self,
        problem: &(impl Problem<T> + Sync),
        _context: &ExecutionContext<T>,
    ) -> Self::StepState {
        let mut rng = Random::new(self.parameters.random_seed.unwrap_or_else(seed_from_time));
        let mut current = problem.create_solution(&mut rng);
        problem.evaluate(&mut current);

        HillClimbingState {
            current,
            rng,
            iteration: 0,
            evaluations: 1,
        }
    }

    fn step(
        &self,
        problem: &(impl Problem<T> + Sync),
        state: &mut Self::StepState,
        _context: &ExecutionContext<T>,
    ) {
        state.iteration += 1;

        let mut neighbor = state.current.copy();
        self.parameters.mutation_operator.execute(
            &mut neighbor,
            self.parameters.mutation_probability,
            &mut state.rng,
        );
        problem.evaluate(&mut neighbor);
        state.evaluations += 1;

        let improved = match problem.get_improvement_direction() {
            ImprovementDirection::Maximize => {
                neighbor.quality_value() > state.current.quality_value()
            }
            ImprovementDirection::Minimize => {
                neighbor.quality_value() < state.current.quality_value()
            }
        };

        if improved {
            state.current = neighbor;
        }
    }

    fn snapshot(&self, state: &Self::StepState) -> ExecutionStateSnapshot<T> {
        let fit = state.current.quality_value();
        ExecutionStateSnapshot::new(
            0,
            state.iteration,
            state.evaluations,
            state.current.copy(),
            fit,
            fit,
            fit,
        )
    }

    fn finalize_step_state(&self, state: Self::StepState) -> Self::SolutionSet {
        let mut result = VectorSolutionSet::new();
        result.add_solution(state.current);
        result
    }
}

impl<T, M, P> ExperimentalCase<T, f64, P> for HillClimbingParameters<T, M>
where
    T: Clone + Send + Sync + 'static,
    M: MutationOperator<T> + Clone + Send + Sync + 'static,
    P: Problem<T, f64> + Sync,
{
    fn algorithm_name(&self) -> &str {
        "HillClimbing"
    }

    fn case_name(&self) -> String {
        format!(
            "{}(mutation_prob={:.4})",
            "HillClimbing",
            self.mutation_probability,
        )
    }

    fn parameters(&self) -> Vec<CaseParameter> {
        vec![
            CaseParameter::new("mutation_operator", self.mutation_operator.name()),
            CaseParameter::new(
                "mutation_probability",
                format!("{:.6}", self.mutation_probability),
            ),
            CaseParameter::new(
                "termination_criteria",
                format!("{:?}", self.termination_criteria),
            ),
        ]
    }

    fn run(&self, problem: &P) -> Result<Box<dyn SolutionSet<T, f64>>, String> {
        let result = HillClimbing::new(self.clone()).run(problem)?;
        Ok(Box::new(result))
    }
}