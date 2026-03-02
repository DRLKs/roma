use crate::algorithms::termination::{
    ExecutionStateSnapshot,
    ImprovementDirection,
    TerminationController,
    TerminationCriteria,
};
use crate::algorithms::traits::Algorithm;
use crate::observer::runtime::run_with_observers_in_worker;
use crate::observer::traits::{AlgorithmObserver, Observable};
use crate::observer::AlgorithmEvent;
use crate::operator::traits::MutationOperator;
use crate::problem::traits::Problem;
use crate::solution_set::implementations::vector_solution_set::VectorSolutionSet;
use crate::solution_set::traits::SolutionSet;
use crate::utils::random::{Random, seed_from_time};

/// Configuration parameters for the Hill Climbing algorithm.
///
/// # Type Parameters
/// - `T`: decision variable type of the solution.
/// - `M`: mutation operator used to generate neighbor solutions.

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
    is_maximization: bool,
    observers: Vec<Box<dyn AlgorithmObserver<T>>>,
}

impl<T, M> HillClimbing<T, M>
where
    T: Clone,
    M: MutationOperator<T>,
{
    /// Creates a new Hill Climbing instance.
    ///
    /// # Arguments
    /// - `parameters`: algorithm configuration.
    /// - `maximization`: `true` for maximization, `false` for minimization.
    pub fn new(parameters: HillClimbingParameters<T, M>, maximization: bool) -> Self {
        Self {
            parameters,
            solution_set: None,
            is_maximization: maximization,
            observers: Vec::new(),
        }
    }
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

    /// Runs the Hill Climbing search process.
    ///
    /// Workflow:
    /// 1. Validate parameters.
    /// 2. Create and evaluate an initial random solution.
    /// 3. Iterate until the configured termination criteria are met:
    ///    - mutate current solution to produce a neighbor,
    ///    - evaluate neighbor,
    ///    - accept neighbor if it improves current quality.
    /// 4. Return a solution set with the final best solution.
    ///
    /// Observer events are emitted for start, progress updates, best-solution
    /// improvements, and end-of-run statistics.
    fn run(&mut self, problem: &(impl Problem<T> + Sync)) -> Self::SolutionSet {
        let is_valid = self.validate_parameters();
        let parameters = &self.parameters;
        let is_maximization = self.is_maximization;
        let observers = std::mem::take(&mut self.observers);

        let (result, observers) = run_with_observers_in_worker(observers, move |context| {
            if !is_valid {
                let message = "Invalid parameters: termination_criteria must not be empty, mutation_probability must be in [0,1]".to_string();
                context.emit(AlgorithmEvent::Error {
                    message: message.clone(),
                });
                panic!("{}", message);
            }

            context.emit(AlgorithmEvent::Start {
                algorithm_name: "HillClimbing".to_string(),
            });

            let mut rng = Random::new(parameters.random_seed.unwrap_or_else(seed_from_time));
            let mut current = problem.create_solution(&mut rng);
            problem.evaluate(&mut current);
            let mut evaluations = 1;
            let mut snapshot_seq = 0u64;

            
            let mut termination =
                TerminationController::new(parameters.termination_criteria.clone(), get_improvement_direction(is_maximization));
            let initial = current.quality_value();
            let initial_snapshot =
                ExecutionStateSnapshot::new(snapshot_seq, 0, evaluations, initial, initial, initial);
            snapshot_seq += 1;
            context.emit(AlgorithmEvent::ExecutionStateUpdated {
                state: initial_snapshot.clone(),
            });
            termination.on_snapshot(&initial_snapshot);

            let mut iteration = 0;
            while !termination.should_terminate() {
                iteration += 1;
                let mut neighbor = current.copy();
                parameters
                    .mutation_operator
                    .execute(&mut neighbor, parameters.mutation_probability, &mut rng);
                problem.evaluate(&mut neighbor);
                evaluations += 1;

                let improved = if is_maximization {
                    neighbor.quality_value() > current.quality_value()
                } else {
                    neighbor.quality_value() < current.quality_value()
                };

                if improved {
                    current = neighbor;
                    context.emit(AlgorithmEvent::BestSolutionUpdate {
                        generation: iteration,
                        solution: current.copy(),
                    });
                }

                if iteration % 10 == 0 || improved {
                    let fit = current.quality_value();
                    let snapshot = ExecutionStateSnapshot::new(
                        snapshot_seq,
                        iteration,
                        evaluations,
                        fit,
                        fit,
                        fit,
                    );
                    snapshot_seq += 1;
                    context.emit(AlgorithmEvent::ExecutionStateUpdated {
                        state: snapshot.clone(),
                    });
                    termination.on_snapshot(&snapshot);
                } else {
                    termination.on_iteration(iteration);
                    termination.on_evaluations(evaluations);
                }
            }

            context.emit(AlgorithmEvent::End {
                total_generations: iteration,
                total_evaluations: evaluations,
                termination_reason: termination.reason().cloned(),
            });

            let mut result = VectorSolutionSet::new();
            result.add_solution(current);
            result
        });

        self.observers = observers;
        self.solution_set = Some(result.clone());
        result
    }

    /// Validates algorithm parameters.
    ///
    /// Returns `true` when:
    /// - `termination_criteria` is not empty
    /// - `mutation_probability` is in `[0.0, 1.0]`
    fn validate_parameters(&self) -> bool {
        !self.parameters.termination_criteria.is_empty()
            && self.parameters.mutation_probability >= 0.0
            && self.parameters.mutation_probability <= 1.0
    }

    /// Returns the last computed solution set, if the algorithm has been run.
    fn get_solution_set(&self) -> Option<&Self::SolutionSet> {
        self.solution_set.as_ref()
    }
}


fn get_improvement_direction( is_maximization: bool ) -> ImprovementDirection{
    if is_maximization {
        ImprovementDirection::Maximize
    } else {
        ImprovementDirection::Minimize
    }
}