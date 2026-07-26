use std::fmt::{Debug, Display};
use std::str::FromStr;

use crate::algorithms::termination::TerminationCriteria;
use crate::algorithms::traits::Algorithm;
use crate::experiment::traits::{CaseParameter, ExperimentalCase};
use crate::observer::traits::{AlgorithmObserver, Observable};
use crate::operator::traits::NeighborhoodOperator;
use crate::problem::traits::Problem;
use crate::solution::Solution;
use crate::solution_set::implementations::vector_solution_set::VectorSolutionSet;
use crate::solution_set::traits::SolutionSet;
use crate::utils::checkpoint::{
    ExecutionStateSnapshot, StatePayloadDecoder, StatePayloadEncoder, StepStateCheckpoint,
};
use crate::utils::random::{Random, seed_from_time};

/// Configuration for [`VNS`].
#[derive(Clone)]
pub struct VNSParameters<T, N>
where
    T: Clone,
    N: NeighborhoodOperator<T>,
{
    pub neighborhoods: Vec<N>,
    pub local_search_trials: usize,
    pub termination_criteria: TerminationCriteria,
    pub random_seed: Option<u64>,
    _phantom: std::marker::PhantomData<T>,
}

impl<T, N> VNSParameters<T, N>
where
    T: Clone,
    N: NeighborhoodOperator<T>,
{
    /// Creates a new Variable Neighborhood Search parameter set.
    pub fn new(
        neighborhoods: Vec<N>,
        local_search_trials: usize,
        termination_criteria: TerminationCriteria,
    ) -> Self {
        Self {
            neighborhoods,
            local_search_trials,
            termination_criteria,
            random_seed: None,
            _phantom: std::marker::PhantomData,
        }
    }

    /// Uses a fixed RNG seed for reproducible executions.
    pub fn with_seed(mut self, seed: u64) -> Self {
        self.random_seed = Some(seed);
        self
    }
}

/// Variable Neighborhood Search optimizer parameterized by explicit neighborhood operators.
pub struct VNS<T, N>
where
    T: Clone,
    N: NeighborhoodOperator<T>,
{
    parameters: VNSParameters<T, N>,
    solution_set: Option<VectorSolutionSet<T>>,
    observers: Vec<Box<dyn AlgorithmObserver<T>>>,
}

/// Serializable execution state used by checkpoint and resume flows.
pub struct VNSState<T>
where
    T: Clone,
{
    current: Solution<T>,
    best: Solution<T>,
    neighborhood_index: usize,
    rng: Random,
    iteration: usize,
    evaluations: usize,
}

impl<T> StepStateCheckpoint<T, f64> for VNSState<T>
where
    T: Clone + Display + FromStr + Debug,
{
    fn random_seed(&self) -> u64 {
        self.rng.state()
    }

    fn to_payload(&self) -> Vec<u8> {
        let mut payload = StatePayloadEncoder::new();
        payload
            .write_usize(self.iteration)
            .expect("iteration should serialize into checkpoint payload");
        payload
            .write_usize(self.evaluations)
            .expect("evaluations should serialize into checkpoint payload");
        payload.write_u64(self.rng.state());
        payload
            .write_usize(self.neighborhood_index)
            .expect("neighborhood index should serialize into checkpoint payload");
        payload
            .write_solution(&self.current)
            .expect("current solution should serialize into checkpoint payload");
        payload
            .write_solution(&self.best)
            .expect("best solution should serialize into checkpoint payload");
        payload.finish()
    }

    fn from_payload(payload: &[u8]) -> Self {
        let mut payload = StatePayloadDecoder::new(payload)
            .expect("critical error: invalid VNS checkpoint payload");
        let iteration = payload
            .read_usize()
            .expect("critical error: missing checkpoint iteration");
        let evaluations = payload
            .read_usize()
            .expect("critical error: missing checkpoint evaluations");
        let random_seed = payload
            .read_u64()
            .expect("critical error: missing checkpoint RNG state");
        let neighborhood_index = payload
            .read_usize()
            .expect("critical error: missing checkpoint neighborhood index");
        let current = payload
            .read_solution()
            .expect("critical error: could not decode current solution");
        let best = payload
            .read_solution()
            .expect("critical error: could not decode best solution");
        payload
            .ensure_finished()
            .expect("critical error: trailing bytes in VNS checkpoint payload");

        Self {
            current,
            best,
            neighborhood_index,
            rng: Random::new(random_seed),
            iteration,
            evaluations,
        }
    }

    fn iteration(&self) -> usize {
        self.iteration
    }

    fn evaluations(&self) -> usize {
        self.evaluations
    }
}

impl<T, N> Observable<T> for VNS<T, N>
where
    T: Clone + Send + 'static,
    N: NeighborhoodOperator<T>,
{
    fn add_observer(&mut self, observer: Box<dyn AlgorithmObserver<T>>) {
        self.observers.push(observer);
    }

    fn clear_observers(&mut self) {
        self.observers.clear();
    }
}

impl<T, N> Algorithm<T> for VNS<T, N>
where
    T: Clone + Send + Sync + 'static + Display + FromStr + Debug,
    N: NeighborhoodOperator<T> + Send + Sync,
{
    type SolutionSet = VectorSolutionSet<T>;
    type Parameters = VNSParameters<T, N>;
    type StepState = VNSState<T>;

    fn new(parameters: Self::Parameters) -> Self {
        Self {
            parameters,
            solution_set: None,
            observers: Vec::new(),
        }
    }

    fn algorithm_name(&self) -> &str {
        "VNS"
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

    fn validate_parameters(&self) -> Result<(), String> {
        if self.parameters.neighborhoods.is_empty() {
            return Err("neighborhoods must not be empty".to_string());
        }

        if self.parameters.local_search_trials == 0 {
            return Err("local_search_trials must be > 0".to_string());
        }

        if self.parameters.termination_criteria.is_empty() {
            return Err("termination_criteria must not be empty".to_string());
        }

        Ok(())
    }

    fn get_solution_set(&self) -> Option<&Self::SolutionSet> {
        self.solution_set.as_ref()
    }

    fn initialize_step_state(&self, problem: &(impl Problem<T> + Sync)) -> Self::StepState {
        let mut rng = Random::new(self.parameters.random_seed.unwrap_or_else(seed_from_time));
        let mut current = problem.create_solution(&mut rng);
        problem.evaluate(&mut current);

        Self::StepState {
            best: current.copy(),
            current,
            neighborhood_index: 0,
            rng,
            iteration: 0,
            evaluations: 1,
        }
    }

    fn step(&self, problem: &(impl Problem<T> + Sync), state: &mut Self::StepState) {
        state.iteration += 1;
        let real_bounds = problem.real_bounds();

        let neighborhood = &self.parameters.neighborhoods[state.neighborhood_index];
        let mut candidate =
            neighborhood.random_neighbor(&state.current, real_bounds, &mut state.rng);
        problem.evaluate(&mut candidate);
        state.evaluations += 1;

        let mut local_best = candidate;
        for _ in 0..self.parameters.local_search_trials {
            let mut improved_candidate =
                neighborhood.random_neighbor(&local_best, real_bounds, &mut state.rng);
            problem.evaluate(&mut improved_candidate);
            state.evaluations += 1;

            if problem.is_better_fitness(
                improved_candidate.quality_value(),
                local_best.quality_value(),
            ) {
                local_best = improved_candidate;
            }
        }

        if problem.is_better_fitness(local_best.quality_value(), state.current.quality_value()) {
            state.current = local_best;
            if problem.is_better_fitness(state.current.quality_value(), state.best.quality_value())
            {
                state.best = state.current.copy();
            }
            state.neighborhood_index = 0;
        } else {
            state.neighborhood_index =
                (state.neighborhood_index + 1) % self.parameters.neighborhoods.len();
        }
    }

    fn build_snapshot(
        &self,
        problem: &(impl Problem<T> + Sync),
        state: &Self::StepState,
    ) -> ExecutionStateSnapshot {
        let fitness = state.best.quality_value();
        ExecutionStateSnapshot {
            iteration: state.iteration,
            evaluations: state.evaluations,
            best_fitness: fitness,
            average_fitness: fitness,
            worst_fitness: fitness,
            best_solution_presentation: problem.format_solution(&state.best),
        }
    }

    fn finalize_step_state(&self, state: Self::StepState) -> Self::SolutionSet {
        let mut result = VectorSolutionSet::new();
        result.add_solution(state.best);
        result
    }

    fn checkpoint_algorithm_parameters(&self) -> String {
        let neighborhood_names = self
            .parameters
            .neighborhoods
            .iter()
            .map(|n| n.name().to_string())
            .collect::<Vec<_>>()
            .join(",");

        format!(
            "neighborhoods=[{}];local_search_trials={};termination={:?}",
            neighborhood_names,
            self.parameters.local_search_trials,
            self.parameters.termination_criteria
        )
    }
}

impl<T, N, P> ExperimentalCase<T, f64, P> for VNSParameters<T, N>
where
    T: Clone + Send + Sync + 'static + Display + FromStr + Debug,
    N: NeighborhoodOperator<T> + Clone + Send + Sync + 'static,
    P: Problem<T, f64> + Sync,
{
    fn algorithm_name(&self) -> &str {
        "VNS"
    }

    fn case_name(&self) -> String {
        format!(
            "VNS(neighborhoods={}, local_trials={})",
            self.neighborhoods.len(),
            self.local_search_trials
        )
    }

    fn parameters(&self) -> Vec<CaseParameter> {
        vec![
            CaseParameter::new("neighborhood_count", self.neighborhoods.len().to_string()),
            CaseParameter::new("local_search_trials", self.local_search_trials.to_string()),
            CaseParameter::new(
                "termination_criteria",
                format!("{:?}", self.termination_criteria),
            ),
        ]
    }

    fn run(&self, problem: &P) -> Result<Box<dyn SolutionSet<T, f64>>, String> {
        let result = VNS::new(self.clone()).run(problem)?;
        Ok(Box::new(result))
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::TerminationCriterion;
    use crate::operator::neighborhood_operator_implementations::gaussian_neighborhood::GaussianNeighborhood;
    use crate::problem::AckleyProblem;
    use crate::solution_set::traits::SolutionSet;

    #[test]
    fn vns_rejects_empty_neighborhoods() {
        let parameters: VNSParameters<f64, GaussianNeighborhood> = VNSParameters::new(
            Vec::new(),
            3,
            TerminationCriteria::new(vec![TerminationCriterion::MaxIterations(5)]),
        );
        let algorithm = VNS::new(parameters);

        assert_eq!(
            algorithm.validate_parameters(),
            Err("neighborhoods must not be empty".to_string())
        );
    }

    #[test]
    fn vns_runs_on_ackley_with_multiple_neighborhoods() {
        let problem = AckleyProblem::new(6, -5.0, 5.0);
        let parameters = VNSParameters::new(
            vec![
                GaussianNeighborhood::new(0.1),
                GaussianNeighborhood::new(0.5),
            ],
            4,
            TerminationCriteria::new(vec![TerminationCriterion::MaxIterations(15)]),
        )
        .with_seed(31);

        let mut algorithm = VNS::new(parameters);
        let result = algorithm
            .run(&problem)
            .expect("VNS on Ackley should succeed");

        assert_eq!(result.size(), 1);
        let best = result.get(0).expect("Expected one solution");
        assert_eq!(best.num_variables(), 6);
        assert!(
            best.variables()
                .iter()
                .all(|value| (-5.0..=5.0).contains(value))
        );
        assert!(best.quality_value().is_finite());
    }
}
