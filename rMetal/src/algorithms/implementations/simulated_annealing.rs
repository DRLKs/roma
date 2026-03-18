use crate::algorithms::runtime::ExecutionContext;
use crate::algorithms::termination::{
    ExecutionStateSnapshot,
    TerminationCriteria,
};
use crate::algorithms::traits::Algorithm;
use crate::experiment::traits::{CaseParameter, ExperimentalCase};
use crate::observer::traits::{AlgorithmObserver, Observable};
use crate::operator::traits::MutationOperator;
use crate::problem::traits::Problem;
use crate::solution::Solution;
use crate::solution_set::implementations::vector_solution_set::VectorSolutionSet;
use crate::solution_set::traits::SolutionSet;
use crate::utils::random::{seed_from_time, Random};

#[derive(Clone)]
pub struct SimulatedAnnealingParameters<T, M>
where
    T: Clone,
    M: MutationOperator<T>,
{
    pub mutation_operator: M,
    pub mutation_probability: f64,
    pub initial_temperature: f64,
    pub minimum_temperature: f64,
    pub cooling_rate: f64,
    pub termination_criteria: TerminationCriteria,
    pub random_seed: Option<u64>,
    _phantom: std::marker::PhantomData<T>,
}

impl<T, M> SimulatedAnnealingParameters<T, M>
where
    T: Clone,
    M: MutationOperator<T>,
{
    pub fn new(
        mutation_operator: M,
        mutation_probability: f64,
        initial_temperature: f64,
        cooling_rate: f64,
        termination_criteria: TerminationCriteria,
    ) -> Self {
        Self {
            mutation_operator,
            mutation_probability,
            initial_temperature,
            minimum_temperature: 1e-8,
            cooling_rate,
            termination_criteria,
            random_seed: None,
            _phantom: std::marker::PhantomData,
        }
    }

    pub fn with_minimum_temperature(mut self, minimum_temperature: f64) -> Self {
        self.minimum_temperature = minimum_temperature;
        self
    }

    pub fn with_seed(mut self, seed: u64) -> Self {
        self.random_seed = Some(seed);
        self
    }
}

pub struct SimulatedAnnealing<T, M>
where
    T: Clone,
    M: MutationOperator<T>,
{
    parameters: SimulatedAnnealingParameters<T, M>,
    solution_set: Option<VectorSolutionSet<T>>,
    observers: Vec<Box<dyn AlgorithmObserver<T>>>,
}

pub struct SimulatedAnnealingState<T>
where
    T: Clone,
{
    current: Solution<T>,
    best: Solution<T>,
    temperature: f64,
    rng: Random,
    iteration: usize,
    evaluations: usize,
}

impl<T, M> Observable<T> for SimulatedAnnealing<T, M>
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

impl<T, M> Algorithm<T> for SimulatedAnnealing<T, M>
where
    T: Clone + Send + Sync + 'static,
    M: MutationOperator<T> + Send + Sync,
{
    type SolutionSet = VectorSolutionSet<T>;
    type Parameters = SimulatedAnnealingParameters<T, M>;
    type StepState = SimulatedAnnealingState<T>;

    fn new(parameters: Self::Parameters) -> Self {
        Self {
            parameters,
            solution_set: None,
            observers: Vec::new(),
        }
    }

    fn algorithm_name(&self) -> &str {
        "SimulatedAnnealing"
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
        if self.parameters.termination_criteria.is_empty() {
            return Err("termination_criteria must not be empty".to_string());
        }

        if !(0.0..=1.0).contains(&self.parameters.mutation_probability) {
            return Err("mutation_probability must be in [0,1]".to_string());
        }

        if !(0.0 < self.parameters.cooling_rate && self.parameters.cooling_rate <= 1.0) {
            return Err("cooling_rate must be in (0,1]".to_string());
        }

        if self.parameters.initial_temperature <= 0.0 {
            return Err("initial_temperature must be > 0".to_string());
        }

        if self.parameters.minimum_temperature <= 0.0 {
            return Err("minimum_temperature must be > 0".to_string());
        }

        if self.parameters.minimum_temperature > self.parameters.initial_temperature {
            return Err("minimum_temperature must be <= initial_temperature".to_string());
        }

        Ok(())
    }

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

        SimulatedAnnealingState {
            best: current.copy(),
            current,
            temperature: self.parameters.initial_temperature,
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

        let mut candidate = state.current.copy();
        self.parameters.mutation_operator.execute(
            &mut candidate,
            self.parameters.mutation_probability,
            &mut state.rng,
        );
        problem.evaluate(&mut candidate);
        state.evaluations += 1;

        let current_quality = state.current.quality_value();
        let candidate_quality = candidate.quality_value();
        let direction: crate::algorithms::termination::ImprovementDirection =
            problem.get_improvement_direction();
        let is_better = match direction {
            crate::algorithms::termination::ImprovementDirection::Maximize => {
                candidate_quality > current_quality
            }
            crate::algorithms::termination::ImprovementDirection::Minimize => {
                candidate_quality < current_quality
            }
        };

        if is_better {
            state.current = candidate;
        } else {
            let loss = match direction {
                crate::algorithms::termination::ImprovementDirection::Maximize => {
                    current_quality - candidate_quality
                }
                crate::algorithms::termination::ImprovementDirection::Minimize => {
                    candidate_quality - current_quality
                }
            };

            if state.temperature > 0.0 {
                let acceptance_probability = (-loss / state.temperature).exp().clamp(0.0, 1.0);
                if state.rng.next_f64() < acceptance_probability {
                    state.current = candidate;
                }
            }
        }

        let current_best = state.best.quality_value();
        let current_value = state.current.quality_value();
        let improved_best = match direction {
            crate::algorithms::termination::ImprovementDirection::Maximize => {
                current_value > current_best
            }
            crate::algorithms::termination::ImprovementDirection::Minimize => {
                current_value < current_best
            }
        };

        if improved_best {
            state.best = state.current.copy();
        }

        state.temperature = (state.temperature * self.parameters.cooling_rate)
            .max(self.parameters.minimum_temperature);
    }

    fn snapshot(&self, state: &Self::StepState) -> ExecutionStateSnapshot<T> {
        let fit = state.best.quality_value();
        ExecutionStateSnapshot::new(
            0,
            state.iteration,
            state.evaluations,
            state.best.copy(),
            fit,
            fit,
            fit,
        )
    }

    fn finalize_step_state(&self, state: Self::StepState) -> Self::SolutionSet {
        let mut result = VectorSolutionSet::new();
        result.add_solution(state.best);
        result
    }
}

impl<T, M, P> ExperimentalCase<T, f64, P> for SimulatedAnnealingParameters<T, M>
where
    T: Clone + Send + Sync + 'static,
    M: MutationOperator<T> + Clone + Send + Sync + 'static,
    P: Problem<T, f64> + Sync,
{
    fn algorithm_name(&self) -> &str {
        "SimulatedAnnealing"
    }

    fn case_name(&self) -> String {
        format!(
            "{}(mut={:.4}, t0={:.3}, cooling={:.4})",
            "SimulatedAnnealing",
            self.mutation_probability,
            self.initial_temperature,
            self.cooling_rate,
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
                "initial_temperature",
                format!("{:.6}", self.initial_temperature),
            ),
            CaseParameter::new(
                "minimum_temperature",
                format!("{:.6}", self.minimum_temperature),
            ),
            CaseParameter::new("cooling_rate", format!("{:.6}", self.cooling_rate)),
            CaseParameter::new(
                "termination_criteria",
                format!("{:?}", self.termination_criteria),
            ),
        ]
    }

    fn run(&self, problem: &P) -> Result<Box<dyn SolutionSet<T, f64>>, String> {
        let result = SimulatedAnnealing::new(self.clone()).run(problem)?;
        Ok(Box::new(result))
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::algorithms::termination::TerminationCriterion;
    use crate::operator::mutation_operator_implementations::swap_mutation::SwapMutation;
    use crate::problem::implementations::tsp_problem::TspProblem;
    use crate::solution_set::traits::SolutionSet;

    #[test]
    fn validates_basic_tsp_run() {
        let matrix = vec![
            vec![0.0, 10.0, 20.0, 10.0],
            vec![10.0, 0.0, 15.0, 25.0],
            vec![20.0, 15.0, 0.0, 30.0],
            vec![10.0, 25.0, 30.0, 0.0],
        ];
        let problem = TspProblem::with_distance_matrix(matrix);

        let params = SimulatedAnnealingParameters::new(
            SwapMutation::new(),
            0.4,
            100.0,
            0.99,
            TerminationCriteria::new(vec![TerminationCriterion::MaxIterations(50)]),
        )
        .with_seed(42);

        let mut algorithm = SimulatedAnnealing::new(params);
        let result = algorithm.run(&problem).expect("simulated annealing should run");
        assert_eq!(result.solutions().len(), 1);
        assert!(result.solutions()[0].quality_value().is_finite());
    }
}
