use std::fmt::{Debug, Display};
use std::str::FromStr;

use crate::algorithms::termination::{TerminationCriteria, TerminationCriterion};
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
use crate::utils::statistics::calculate_population_statistics;

/// Configuration for [`BeamSearch`].
///
/// Finite neighborhoods are enumerated through
/// [`NeighborhoodOperator::all_neighbors`]. When an operator cannot enumerate
/// its neighborhood (for example, a continuous one), the search samples
/// `candidates_per_solution` neighbors from every beam member instead.
#[derive(Clone)]
pub struct BeamSearchParameters<T, N>
where
    T: Clone,
    N: NeighborhoodOperator<T>,
{
    pub neighborhood: N,
    pub beam_width: usize,
    pub candidates_per_solution: usize,
    pub termination_criteria: TerminationCriteria,
    pub random_seed: Option<u64>,
    _phantom: std::marker::PhantomData<T>,
}

impl<T, N> BeamSearchParameters<T, N>
where
    T: Clone,
    N: NeighborhoodOperator<T>,
{
    /// Creates parameters with `beam_width` initial solutions.
    ///
    /// For non-enumerable neighborhoods, the default number of sampled
    /// successors per beam member is also `beam_width`.
    pub fn new(
        neighborhood: N,
        beam_width: usize,
        termination_criteria: TerminationCriteria,
    ) -> Self {
        Self {
            neighborhood,
            beam_width,
            candidates_per_solution: beam_width,
            termination_criteria,
            random_seed: None,
            _phantom: std::marker::PhantomData,
        }
    }

    /// Sets the number of sampled successors per beam member when exhaustive
    /// neighborhood enumeration is unavailable.
    pub fn with_candidates_per_solution(mut self, candidates_per_solution: usize) -> Self {
        self.candidates_per_solution = candidates_per_solution;
        self
    }

    /// Uses a fixed RNG seed for reproducible executions.
    pub fn with_seed(mut self, seed: u64) -> Self {
        self.random_seed = Some(seed);
        self
    }
}

/// Local beam search using the framework's neighborhood abstraction.
///
/// Every step expands each solution in the current beam, evaluates the
/// successors, and retains the best `beam_width` candidates. Current beam
/// members also participate in selection, so the best known solution cannot
/// be discarded by a worse generation.
pub struct BeamSearch<T, N>
where
    T: Clone,
    N: NeighborhoodOperator<T>,
{
    parameters: BeamSearchParameters<T, N>,
    solution_set: Option<VectorSolutionSet<T>>,
    observers: Vec<Box<dyn AlgorithmObserver<T>>>,
}

/// Serializable execution state used by checkpoint and resume flows.
pub struct BeamSearchState<T>
where
    T: Clone,
{
    beam: Vec<Solution<T>>,
    rng: Random,
    iteration: usize,
    evaluations: usize,
}

impl<T> StepStateCheckpoint<T, f64> for BeamSearchState<T>
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
            .write_solution_vec(&self.beam)
            .expect("beam should serialize into checkpoint payload");
        payload.finish()
    }

    fn from_payload(payload: &[u8]) -> Self {
        let mut payload = StatePayloadDecoder::new(payload)
            .expect("critical error: invalid beam search checkpoint payload");
        let iteration = payload
            .read_usize()
            .expect("critical error: missing checkpoint iteration");
        let evaluations = payload
            .read_usize()
            .expect("critical error: missing checkpoint evaluations");
        let random_seed = payload
            .read_u64()
            .expect("critical error: missing checkpoint RNG state");
        let beam = payload
            .read_solution_vec()
            .expect("critical error: could not decode beam");
        payload
            .ensure_finished()
            .expect("critical error: trailing bytes in beam search checkpoint payload");

        Self {
            beam,
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

impl<T, N> Observable<T> for BeamSearch<T, N>
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

impl<T, N> Algorithm<T> for BeamSearch<T, N>
where
    T: Clone + Send + Sync + 'static + Display + FromStr + Debug,
    N: NeighborhoodOperator<T> + Send + Sync,
{
    type SolutionSet = VectorSolutionSet<T>;
    type Parameters = BeamSearchParameters<T, N>;
    type StepState = BeamSearchState<T>;

    fn new(parameters: Self::Parameters) -> Self {
        Self {
            parameters,
            solution_set: None,
            observers: Vec::new(),
        }
    }

    fn algorithm_name(&self) -> &str {
        "BeamSearch"
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
        if self.parameters.beam_width == 0 {
            return Err("beam_width must be > 0".to_string());
        }

        if self.parameters.candidates_per_solution == 0 {
            return Err("candidates_per_solution must be > 0".to_string());
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
        let mut beam = Vec::with_capacity(self.parameters.beam_width);

        for _ in 0..self.parameters.beam_width {
            let mut solution = problem.create_solution(&mut rng);
            problem.evaluate(&mut solution);
            beam.push(solution);
        }

        Self::StepState {
            beam,
            rng,
            iteration: 0,
            evaluations: self.parameters.beam_width,
        }
    }

    fn step(&self, problem: &(impl Problem<T> + Sync), state: &mut Self::StepState) {
        state.iteration += 1;
        let bounds = problem.real_bounds();
        let mut candidates = state.beam.clone();
        let evaluation_limit = self
            .parameters
            .termination_criteria
            .all()
            .iter()
            .filter_map(|criterion| match criterion {
                TerminationCriterion::MaxEvaluations(limit) => Some(*limit),
                _ => None,
            })
            .min()
            .unwrap_or(usize::MAX);

        'beam: for beam_member in &state.beam {
            let successors = self
                .parameters
                .neighborhood
                .all_neighbors(beam_member, bounds)
                .unwrap_or_else(|| {
                    (0..self.parameters.candidates_per_solution)
                        .map(|_| {
                            self.parameters.neighborhood.random_neighbor(
                                beam_member,
                                bounds,
                                &mut state.rng,
                            )
                        })
                        .collect()
                });

            for mut successor in successors {
                if state.evaluations >= evaluation_limit {
                    break 'beam;
                }
                problem.evaluate(&mut successor);
                state.evaluations += 1;
                candidates.push(successor);
            }
        }

        candidates.sort_by(|left, right| {
            if problem.dominates(left, right) {
                std::cmp::Ordering::Less
            } else if problem.dominates(right, left) {
                std::cmp::Ordering::Greater
            } else {
                std::cmp::Ordering::Equal
            }
        });
        candidates.truncate(self.parameters.beam_width);
        state.beam = candidates;
    }

    fn build_snapshot(
        &self,
        problem: &(impl Problem<T> + Sync),
        state: &Self::StepState,
    ) -> ExecutionStateSnapshot {
        let stats = calculate_population_statistics(&state.beam, problem);
        let best = &state.beam[stats
            .best_index
            .expect("beam_width > 0 guarantees a non-empty beam")];

        ExecutionStateSnapshot {
            iteration: state.iteration,
            evaluations: state.evaluations,
            best_fitness: stats.best_fitness,
            average_fitness: stats.average_fitness,
            worst_fitness: stats.worst_fitness,
            best_solution_presentation: problem.format_solution(best),
        }
    }

    fn finalize_step_state(&self, state: Self::StepState) -> Self::SolutionSet {
        VectorSolutionSet::from_vec(state.beam)
    }

    fn checkpoint_algorithm_parameters(&self) -> String {
        format!(
            "neighborhood={};beam_width={};candidates_per_solution={};termination={:?}",
            self.parameters.neighborhood.name(),
            self.parameters.beam_width,
            self.parameters.candidates_per_solution,
            self.parameters.termination_criteria,
        )
    }
}

impl<T, N, P> ExperimentalCase<T, f64, P> for BeamSearchParameters<T, N>
where
    T: Clone + Send + Sync + 'static + Display + FromStr + Debug,
    N: NeighborhoodOperator<T> + Clone + Send + Sync + 'static,
    P: Problem<T, f64> + Sync,
{
    fn algorithm_name(&self) -> &str {
        "BeamSearch"
    }

    fn case_name(&self) -> String {
        format!(
            "BeamSearch(beam_width={}, candidates_per_solution={})",
            self.beam_width, self.candidates_per_solution
        )
    }

    fn parameters(&self) -> Vec<CaseParameter> {
        vec![
            CaseParameter::new("neighborhood", self.neighborhood.name()),
            CaseParameter::new("beam_width", self.beam_width.to_string()),
            CaseParameter::new(
                "candidates_per_solution",
                self.candidates_per_solution.to_string(),
            ),
            CaseParameter::new(
                "termination_criteria",
                format!("{:?}", self.termination_criteria),
            ),
        ]
    }

    fn run(&self, problem: &P) -> Result<Box<dyn SolutionSet<T, f64>>, String> {
        let result = BeamSearch::new(self.clone()).run(problem)?;
        Ok(Box::new(result))
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::TerminationCriterion;
    use crate::operator::BitFlipNeighborhood;
    use crate::problem::{Problem, SolutionComparison, compare_scalar_qualities};
    use crate::solution::Solution;
    use crate::utils::random::Random;

    struct OneMax;

    impl Problem<bool> for OneMax {
        fn new() -> Self {
            Self
        }

        fn evaluate(&self, solution: &mut Solution<bool>) {
            solution
                .set_quality(solution.variables().iter().filter(|value| **value).count() as f64);
        }

        fn create_solution(&self, _rng: &mut Random) -> Solution<bool> {
            Solution::new(vec![false, false, false, false])
        }

        fn set_problem_description(&mut self, _description: String) {}

        fn get_problem_description(&self) -> String {
            "one max".to_string()
        }

        fn compare_qualities(&self, left: Option<&f64>, right: Option<&f64>) -> SolutionComparison {
            compare_scalar_qualities(left, right, |a, b| a > b)
        }

        fn better_fitness_fn(&self) -> fn(f64, f64) -> bool {
            |candidate, reference| candidate > reference
        }
    }

    #[test]
    fn rejects_zero_beam_width() {
        let parameters: BeamSearchParameters<bool, BitFlipNeighborhood> = BeamSearchParameters::new(
            BitFlipNeighborhood::new(),
            0,
            TerminationCriteria::new(vec![TerminationCriterion::MaxIterations(1)]),
        );

        assert_eq!(
            BeamSearch::new(parameters).validate_parameters(),
            Err("beam_width must be > 0".to_string())
        );
    }

    #[test]
    fn rejects_zero_fallback_candidate_count() {
        let parameters = BeamSearchParameters::new(
            BitFlipNeighborhood::new(),
            2,
            TerminationCriteria::new(vec![TerminationCriterion::MaxIterations(1)]),
        )
        .with_candidates_per_solution(0);

        assert_eq!(
            BeamSearch::new(parameters).validate_parameters(),
            Err("candidates_per_solution must be > 0".to_string())
        );
    }

    #[test]
    fn rejects_empty_termination_criteria() {
        let parameters = BeamSearchParameters::new(
            BitFlipNeighborhood::new(),
            2,
            TerminationCriteria::new(vec![]),
        );

        assert_eq!(
            BeamSearch::new(parameters).validate_parameters(),
            Err("termination_criteria must not be empty".to_string())
        );
    }

    #[test]
    fn step_does_not_overshoot_max_evaluations() {
        let search = BeamSearch::new(
            BeamSearchParameters::new(
                BitFlipNeighborhood::new(),
                2,
                TerminationCriteria::new(vec![TerminationCriterion::MaxEvaluations(3)]),
            )
            .with_seed(4),
        );
        let mut state = search.initialize_step_state(&OneMax);

        search.step(&OneMax, &mut state);

        assert_eq!(state.evaluations, 3);
        assert_eq!(state.beam.len(), 2);
        assert_eq!(state.beam[0].quality().copied(), Some(1.0));
    }

    #[test]
    fn checkpoint_roundtrip_preserves_beam_and_progress() {
        let state = BeamSearchState {
            beam: vec![
                {
                    let mut solution = Solution::new(vec![true, false]);
                    solution.set_quality(1.0);
                    solution
                },
                {
                    let mut solution = Solution::new(vec![true, true]);
                    solution.set_quality(2.0);
                    solution
                },
            ],
            rng: Random::new(91),
            iteration: 7,
            evaluations: 23,
        };

        let restored = BeamSearchState::<bool>::from_payload(&state.to_payload());

        assert_eq!(restored.iteration, 7);
        assert_eq!(restored.evaluations, 23);
        assert_eq!(restored.rng.state(), state.rng.state());
        assert_eq!(restored.beam.len(), 2);
        assert_eq!(restored.beam[0].variables(), &[true, false]);
        assert_eq!(restored.beam[0].quality().copied(), Some(1.0));
        assert_eq!(restored.beam[1].variables(), &[true, true]);
        assert_eq!(restored.beam[1].quality().copied(), Some(2.0));
    }
}
