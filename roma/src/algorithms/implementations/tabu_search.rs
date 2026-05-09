use std::collections::HashMap;
use std::fmt::{Debug, Display};
use std::str::FromStr;

use crate::algorithms::checkpoint::StepStateCheckpoint;
use crate::algorithms::runtime::ExecutionContext;
use crate::algorithms::termination::{ExecutionStateSnapshot, TerminationCriteria};
use crate::algorithms::traits::Algorithm;
use crate::experiment::traits::{CaseParameter, ExperimentalCase};
use crate::observer::traits::{AlgorithmObserver, Observable};
use crate::operator::traits::NeighborhoodOperator;
use crate::problem::traits::Problem;
use crate::solution::Solution;
use crate::solution_set::implementations::vector_solution_set::VectorSolutionSet;
use crate::solution_set::traits::SolutionSet;
use crate::utils::random::{seed_from_time, Random};

#[derive(Clone)]
pub struct TabuSearchParameters<T, N>
where
    T: Clone,
    N: NeighborhoodOperator<T>,
{
    pub neighborhood_operator: N,
    pub neighborhood_size: usize,
    pub tabu_tenure: usize,
    pub aspiration_enabled: bool,
    pub termination_criteria: TerminationCriteria,
    pub random_seed: Option<u64>,
    _phantom: std::marker::PhantomData<T>,
}

impl<T, N> TabuSearchParameters<T, N>
where
    T: Clone,
    N: NeighborhoodOperator<T>,
{
    pub fn new(
        neighborhood_operator: N,
        neighborhood_size: usize,
        tabu_tenure: usize,
        termination_criteria: TerminationCriteria,
    ) -> Self {
        Self {
            neighborhood_operator,
            neighborhood_size,
            tabu_tenure,
            aspiration_enabled: true,
            termination_criteria,
            random_seed: None,
            _phantom: std::marker::PhantomData,
        }
    }

    pub fn with_seed(mut self, seed: u64) -> Self {
        self.random_seed = Some(seed);
        self
    }

    pub fn with_aspiration(mut self, aspiration_enabled: bool) -> Self {
        self.aspiration_enabled = aspiration_enabled;
        self
    }
}

pub struct TabuSearch<T, N>
where
    T: Clone,
    N: NeighborhoodOperator<T>,
{
    parameters: TabuSearchParameters<T, N>,
    solution_set: Option<VectorSolutionSet<T>>,
    observers: Vec<Box<dyn AlgorithmObserver<T>>>,
}

pub struct TabuSearchState<T>
where
    T: Clone,
{
    current: Solution<T>,
    best: Solution<T>,
    tabu_memory: HashMap<String, usize>,
    rng: Random,
    iteration: usize,
    evaluations: usize,
}

impl<T> StepStateCheckpoint<T, f64> for TabuSearchState<T>
where
    T: Clone + Display + FromStr + Debug,
{
    fn random_seed(&self) -> u64 {
        self.rng.state()
    }

    fn to_payload(&self) -> String {
        let tabu_payload = self
            .tabu_memory
            .iter()
            .map(|(signature, expiry)| format!("{}#{}", expiry, signature))
            .collect::<Vec<_>>()
            .join("\n");

        format!(
            "iter={};eval={};seed={};curr={};best={};tabu={}",
            self.iteration,
            self.evaluations,
            self.rng.state(),
            self.current.encode(),
            self.best.encode(),
            tabu_payload
        )
    }

    fn from_payload(payload: &str) -> Self {
        let parts: HashMap<&str, &str> = payload
            .split(';')
            .filter_map(|segment| {
                let mut kv = segment.splitn(2, '=');
                Some((kv.next()?, kv.next()?))
            })
            .collect();

        let iteration = parts.get("iter").and_then(|value| value.parse().ok()).unwrap_or(0);
        let evaluations = parts.get("eval").and_then(|value| value.parse().ok()).unwrap_or(0);
        let random_seed = parts
            .get("seed")
            .and_then(|value| value.parse().ok())
            .unwrap_or_else(seed_from_time);
        let current = parts
            .get("curr")
            .and_then(|value| Solution::decode(value).ok())
            .expect("Critical error: Could not decode current state from payload");
        let best = parts
            .get("best")
            .and_then(|value| Solution::decode(value).ok())
            .expect("Critical error: Could not decode best state from payload");
        let tabu_memory = parts
            .get("tabu")
            .map(|entries| {
                entries
                    .split('\n')
                    .filter(|entry| !entry.is_empty())
                    .filter_map(|entry| {
                        let (expiry, signature) = entry.split_once('#')?;
                        Some((signature.to_string(), expiry.parse().ok()?))
                    })
                    .collect::<HashMap<_, _>>()
            })
            .unwrap_or_default();

        Self {
            current,
            best,
            tabu_memory,
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

impl<T, N> Observable<T> for TabuSearch<T, N>
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

impl<T, N> TabuSearch<T, N>
where
    T: Clone + Send + Sync + 'static + Display + FromStr + Debug,
    N: NeighborhoodOperator<T> + Send + Sync,
{
    fn purge_expired_tabu_entries(tabu_memory: &mut HashMap<String, usize>, iteration: usize) {
        tabu_memory.retain(|_, expiry| *expiry > iteration);
    }
}

impl<T, N> Algorithm<T> for TabuSearch<T, N>
where
    T: Clone + Send + Sync + 'static + Display + FromStr + Debug,
    N: NeighborhoodOperator<T> + Send + Sync,
{
    type SolutionSet = VectorSolutionSet<T>;
    type Parameters = TabuSearchParameters<T, N>;
    type StepState = TabuSearchState<T>;

    fn new(parameters: Self::Parameters) -> Self {
        Self {
            parameters,
            solution_set: None,
            observers: Vec::new(),
        }
    }

    fn algorithm_name(&self) -> &str {
        "TabuSearch"
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
        if self.parameters.neighborhood_size == 0 {
            return Err("neighborhood_size must be > 0".to_string());
        }

        if self.parameters.tabu_tenure == 0 {
            return Err("tabu_tenure must be > 0".to_string());
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
        let current_signature = current.encode();
        let mut tabu_memory = HashMap::new();
        tabu_memory.insert(current_signature, self.parameters.tabu_tenure);

        Self::StepState {
            best: current.copy(),
            current,
            tabu_memory,
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
        Self::purge_expired_tabu_entries(&mut state.tabu_memory, state.iteration);

        let mut best_any_candidate: Option<Solution<T>> = None;
        let mut best_admissible_candidate: Option<Solution<T>> = None;
        let best_quality = state.best.quality_value();

        let neighbors = self.parameters.neighborhood_operator.generate_neighbors(
            &state.current,
            self.parameters.neighborhood_size,
            &mut state.rng,
        );

        for mut candidate in neighbors {
            problem.evaluate(&mut candidate);
            state.evaluations += 1;

            let candidate_signature = candidate.encode();
            let is_tabu = state
                .tabu_memory
                .get(&candidate_signature)
                .is_some_and(|expiry| *expiry > state.iteration);
            let aspiration = self.parameters.aspiration_enabled
                && problem.is_better_fitness(candidate.quality_value(), best_quality);
            let admissible = !is_tabu || aspiration;

            if best_any_candidate.as_ref().is_none_or(|best| {
                problem.is_better_fitness(candidate.quality_value(), best.quality_value())
            }) {
                best_any_candidate = Some(candidate.copy());
            }

            if admissible
                && best_admissible_candidate.as_ref().is_none_or(|best| {
                    problem.is_better_fitness(candidate.quality_value(), best.quality_value())
                })
            {
                best_admissible_candidate = Some(candidate);
            }
        }

        let selected = best_admissible_candidate
            .or(best_any_candidate)
            .expect("neighborhood_size > 0 guarantees at least one candidate");

        state.current = selected;
        let selected_signature = state.current.encode();
        state.tabu_memory.insert(
            selected_signature,
            state.iteration + self.parameters.tabu_tenure,
        );

        if problem.is_better_fitness(state.current.quality_value(), state.best.quality_value()) {
            state.best = state.current.copy();
        }
    }

    fn build_snapshot(
        &self,
        _problem: &(impl Problem<T> + Sync),
        state: &Self::StepState,
    ) -> ExecutionStateSnapshot<T> {
        let fitness = state.best.quality_value();
        ExecutionStateSnapshot {
            iteration: state.iteration,
            evaluations: state.evaluations,
            best_solution: state.best.copy(),
            best_fitness: fitness,
            average_fitness: fitness,
            worst_fitness: fitness,
        }
    }

    fn finalize_step_state(&self, state: Self::StepState) -> Self::SolutionSet {
        let mut result = VectorSolutionSet::new();
        result.add_solution(state.best);
        result
    }

    fn checkpoint_algorithm_parameters(&self) -> String {
        format!(
            "neighborhood={};neighborhood_size={};tabu_tenure={};aspiration={};termination={:?}",
            self.parameters.neighborhood_operator.name(),
            self.parameters.neighborhood_size,
            self.parameters.tabu_tenure,
            self.parameters.aspiration_enabled,
            self.parameters.termination_criteria
        )
    }
}

impl<T, N, P> ExperimentalCase<T, f64, P> for TabuSearchParameters<T, N>
where
    T: Clone + Send + Sync + 'static + Display + FromStr + Debug,
    N: NeighborhoodOperator<T> + Clone + Send + Sync + 'static,
    P: Problem<T, f64> + Sync,
{
    fn algorithm_name(&self) -> &str {
        "TabuSearch"
    }

    fn case_name(&self) -> String {
        format!(
            "TabuSearch(neighbors={}, tenure={})",
            self.neighborhood_size, self.tabu_tenure
        )
    }

    fn parameters(&self) -> Vec<CaseParameter> {
        vec![
            CaseParameter::new("neighborhood_operator", self.neighborhood_operator.name()),
            CaseParameter::new("neighborhood_size", self.neighborhood_size.to_string()),
            CaseParameter::new("tabu_tenure", self.tabu_tenure.to_string()),
            CaseParameter::new("aspiration_enabled", self.aspiration_enabled.to_string()),
            CaseParameter::new(
                "termination_criteria",
                format!("{:?}", self.termination_criteria),
            ),
        ]
    }

    fn run(&self, problem: &P) -> Result<Box<dyn SolutionSet<T, f64>>, String> {
        let result = TabuSearch::new(self.clone()).run(problem)?;
        Ok(Box::new(result))
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::operator::PermutationSwapNeighborhood;
    use crate::problem::QapProblem;
    use crate::solution_set::traits::SolutionSet;
    use crate::TerminationCriterion;

    #[test]
    fn tabu_search_rejects_zero_tenure() {
        let parameters = TabuSearchParameters::new(
            PermutationSwapNeighborhood::new(1),
            8,
            0,
            TerminationCriteria::new(vec![TerminationCriterion::MaxIterations(5)]),
        );
        let algorithm = TabuSearch::new(parameters);

        assert_eq!(
            algorithm.validate_parameters(),
            Err("tabu_tenure must be > 0".to_string())
        );
    }

    #[test]
    fn tabu_search_runs_on_qap() {
        let problem = QapProblem::with_matrices(
            vec![
                vec![0.0, 3.0, 1.0, 2.0],
                vec![3.0, 0.0, 4.0, 1.0],
                vec![1.0, 4.0, 0.0, 5.0],
                vec![2.0, 1.0, 5.0, 0.0],
            ],
            vec![
                vec![0.0, 8.0, 6.0, 4.0],
                vec![8.0, 0.0, 7.0, 5.0],
                vec![6.0, 7.0, 0.0, 3.0],
                vec![4.0, 5.0, 3.0, 0.0],
            ],
        );

        let parameters = TabuSearchParameters::new(
            PermutationSwapNeighborhood::new(1),
            10,
            4,
            TerminationCriteria::new(vec![TerminationCriterion::MaxIterations(20)]),
        )
        .with_seed(23);

        let mut algorithm = TabuSearch::new(parameters);
        let result = algorithm.run(&problem).expect("Tabu Search on QAP should succeed");

        assert_eq!(result.size(), 1);
        let best = result.get(0).expect("Expected one solution");
        let mut assignment = best.variables().to_vec();
        assignment.sort_unstable();

        assert_eq!(assignment, vec![0, 1, 2, 3]);
        assert!(best.quality_value().is_finite());
    }
}