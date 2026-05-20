use crate::algorithms::checkpoint::{ExecutionStateSnapshot, StepStateCheckpoint};
use crate::algorithms::termination::TerminationCriteria;
use crate::algorithms::traits::Algorithm;
use crate::experiment::traits::{CaseParameter, ExperimentalCase};
use crate::observer::traits::{AlgorithmObserver, Observable};
use crate::problem::Problem;
use crate::solution::{RealBounds, Solution};
use crate::solution_set::implementations::vector_solution_set::VectorSolutionSet;
use crate::solution_set::traits::SolutionSet;
use crate::utils::random::Random;
use crate::utils::statistics::calculate_population_statistics;

/// Configuration for [`DifferentialEvolution`].
///
/// The algorithm keeps a fixed-size population and combines individuals using
/// the classic DE/rand/1/bin style trial generation used by this implementation.
#[derive(Clone)]
pub struct DifferentialEvolutionParameters {
    pub population_size: usize,
    pub crossover_rate: f64,
    pub differential_weight: f64,
    pub termination_criteria: TerminationCriteria,
    pub random_seed: Option<u64>,
}

impl DifferentialEvolutionParameters {
    /// Creates a new Differential Evolution parameter set.
    pub fn new(
        population_size: usize,
        crossover_rate: f64,
        differential_weight: f64,
        termination_criteria: TerminationCriteria,
    ) -> Self {
        Self {
            population_size,
            crossover_rate,
            differential_weight,
            termination_criteria,
            random_seed: None,
        }
    }

    /// Uses a fixed RNG seed for reproducible executions.
    pub fn with_seed(mut self, seed: u64) -> Self {
        self.random_seed = Some(seed);
        self
    }
}

/// Differential Evolution optimizer for bounded real-valued problems.
pub struct DifferentialEvolution {
    parameters: DifferentialEvolutionParameters,
    solution_set: Option<VectorSolutionSet<f64>>,
    observers: Vec<Box<dyn AlgorithmObserver<f64>>>,
}

/// Serializable execution state used by checkpoint and resume flows.
pub struct DifferentialEvolutionState {
    population: Vec<Solution<f64>>,
    generation: usize,
    evaluations: usize,
    run_seed: u64,
}

impl StepStateCheckpoint<f64> for DifferentialEvolutionState {
    fn iteration(&self) -> usize {
        self.generation
    }

    fn evaluations(&self) -> usize {
        self.evaluations
    }

    fn random_seed(&self) -> u64 {
        self.run_seed
    }

    fn to_payload(&self) -> String {
        let encoded_population = self
            .population
            .iter()
            .map(|solution| solution.encode())
            .collect::<Vec<_>>()
            .join("\n");

        format!(
            "iter={};eval={};seed={};pop={}",
            self.generation, self.evaluations, self.run_seed, encoded_population
        )
    }

    fn from_payload(payload: &str) -> Self {
        let mut iteration = 0usize;
        let mut evaluations = 0usize;
        let mut run_seed = 0u64;
        let mut population = Vec::new();

        for part in payload.split(';') {
            if let Some(value) = part.strip_prefix("iter=") {
                iteration = value.parse().unwrap_or(0);
            } else if let Some(value) = part.strip_prefix("eval=") {
                evaluations = value.parse().unwrap_or(0);
            } else if let Some(value) = part.strip_prefix("seed=") {
                run_seed = value.parse().unwrap_or(0);
            } else if let Some(value) = part.strip_prefix("pop=") {
                population = value
                    .split('\n')
                    .filter(|encoded| !encoded.is_empty())
                    .filter_map(|encoded| Solution::decode(encoded).ok())
                    .collect();
            }
        }

        Self {
            population,
            generation: iteration,
            evaluations,
            run_seed,
        }
    }
}

impl DifferentialEvolution {
    fn initialize_population(
        parameters: &DifferentialEvolutionParameters,
        problem: &(impl Problem<f64> + Sync),
        rng: &mut Random,
    ) -> Vec<Solution<f64>> {
        let mut population = Vec::with_capacity(parameters.population_size);
        for _ in 0..parameters.population_size {
            let mut solution = problem.create_solution(rng);
            problem.evaluate(&mut solution);
            population.push(solution);
        }
        population
    }

    fn sample_distinct_indices(
        population_len: usize,
        target_index: usize,
        rng: &mut Random,
    ) -> [usize; 3] {
        let mut indices = Vec::with_capacity(3);
        while indices.len() < 3 {
            let candidate = rng.range(population_len as u64) as usize;
            if candidate != target_index && !indices.contains(&candidate) {
                indices.push(candidate);
            }
        }

        [indices[0], indices[1], indices[2]]
    }

    fn build_trial_solution(
        parameters: &DifferentialEvolutionParameters,
        bounds: Option<&RealBounds>,
        target: &Solution<f64>,
        donor_a: &Solution<f64>,
        donor_b: &Solution<f64>,
        donor_c: &Solution<f64>,
        rng: &mut Random,
    ) -> Solution<f64> {
        let mut trial = target.copy();
        let variable_count = target.num_variables();
        if variable_count == 0 {
            return trial;
        }

        let forced_index = rng.range(variable_count as u64) as usize;
        let trial_variables = trial.variables_mut();

        match bounds {
            None => {
                for index in 0..variable_count {
                    if index == forced_index || rng.next_f64() < parameters.crossover_rate {
                        let mutant_value = donor_a.variables()[index]
                            + parameters.differential_weight
                                * (donor_b.variables()[index] - donor_c.variables()[index]);
                        trial_variables[index] = mutant_value;
                    }
                }
            }
            Some(RealBounds::Uniform {
                lower,
                upper,
                dimensions,
            }) => {
                let lower = *lower;
                let upper = *upper;
                let dimensions = *dimensions;
                for index in 0..variable_count {
                    if index == forced_index || rng.next_f64() < parameters.crossover_rate {
                        let mutant_value = donor_a.variables()[index]
                            + parameters.differential_weight
                                * (donor_b.variables()[index] - donor_c.variables()[index]);
                        trial_variables[index] = if index < dimensions {
                            mutant_value.clamp(lower, upper)
                        } else {
                            mutant_value
                        };
                    }
                }
            }
            Some(RealBounds::PerVariable {
                lower_bounds,
                upper_bounds,
            }) => {
                for index in 0..variable_count {
                    if index == forced_index || rng.next_f64() < parameters.crossover_rate {
                        let mutant_value = donor_a.variables()[index]
                            + parameters.differential_weight
                                * (donor_b.variables()[index] - donor_c.variables()[index]);
                        trial_variables[index] = match (lower_bounds.get(index), upper_bounds.get(index)) {
                            (Some(&lower), Some(&upper)) => mutant_value.clamp(lower, upper),
                            _ => mutant_value,
                        };
                    }
                }
            }
        }

        trial
    }
}

impl Observable<f64> for DifferentialEvolution {
    fn add_observer(&mut self, observer: Box<dyn AlgorithmObserver<f64>>) {
        self.observers.push(observer);
    }

    fn clear_observers(&mut self) {
        self.observers.clear();
    }
}

impl Algorithm<f64> for DifferentialEvolution {
    type SolutionSet = VectorSolutionSet<f64>;
    type Parameters = DifferentialEvolutionParameters;
    type StepState = DifferentialEvolutionState;

    fn new(parameters: Self::Parameters) -> Self {
        Self {
            parameters,
            solution_set: None,
            observers: Vec::new(),
        }
    }

    fn algorithm_name(&self) -> &str {
        "DifferentialEvolution"
    }

    fn termination_criteria(&self) -> TerminationCriteria {
        self.parameters.termination_criteria.clone()
    }

    fn observers_mut(&mut self) -> &mut Vec<Box<dyn AlgorithmObserver<f64>>> {
        &mut self.observers
    }

    fn set_solution_set(&mut self, solution_set: Self::SolutionSet) {
        self.solution_set = Some(solution_set);
    }

    fn validate_parameters(&self) -> Result<(), String> {
        if self.parameters.population_size < 4 {
            return Err("population_size must be >= 4".to_string());
        }

        if self.parameters.termination_criteria.is_empty() {
            return Err("termination_criteria must not be empty".to_string());
        }

        if !(0.0..=1.0).contains(&self.parameters.crossover_rate) {
            return Err("crossover_rate must be in [0,1]".to_string());
        }

        if self.parameters.differential_weight <= 0.0 {
            return Err("differential_weight must be > 0".to_string());
        }

        Ok(())
    }

    fn get_solution_set(&self) -> Option<&Self::SolutionSet> {
        self.solution_set.as_ref()
    }

    fn initialize_step_state(&self, problem: &(impl Problem<f64> + Sync)) -> Self::StepState {
        let run_seed = Random::resolve_seed(self.parameters.random_seed);
        let mut rng = Random::new(Random::derive_seed(run_seed, 0));
        let population = Self::initialize_population(&self.parameters, problem, &mut rng);

        DifferentialEvolutionState {
            population,
            generation: 0,
            evaluations: self.parameters.population_size,
            run_seed,
        }
    }

    fn step(
        &self,
        problem: &(impl Problem<f64> + Sync),
        state: &mut Self::StepState,
    ) {
        state.generation += 1;
        let mut rng = Random::new(Random::derive_seed(state.run_seed, state.generation as u64));
        let real_bounds = problem.real_bounds();

        let current_population = state.population.clone();
        let mut next_population = Vec::with_capacity(current_population.len());

        for (target_index, target) in current_population.iter().enumerate() {
            let [a, b, c] = Self::sample_distinct_indices(current_population.len(), target_index, &mut rng);
            let mut trial = Self::build_trial_solution(
                &self.parameters,
                real_bounds,
                target,
                &current_population[a],
                &current_population[b],
                &current_population[c],
                &mut rng,
            );
            problem.evaluate(&mut trial);
            state.evaluations += 1;

            if problem.is_better_fitness(trial.quality_value(), target.quality_value()) {
                next_population.push(trial);
            } else {
                next_population.push(target.copy());
            }
        }

        state.population = next_population;
    }

    fn build_snapshot(
        &self,
        problem: &(impl Problem<f64> + Sync),
        state: &Self::StepState,
    ) -> ExecutionStateSnapshot {
        let stats = calculate_population_statistics(&state.population, problem);
        let best_solution = &state.population[stats.best_index.expect(
            "population should not be empty when reporting progress",
        )];

        ExecutionStateSnapshot {
            iteration: state.generation,
            evaluations: state.evaluations,
            best_fitness: stats.best_fitness,
            average_fitness: stats.average_fitness,
            worst_fitness: stats.worst_fitness,
            best_solution_presentation: problem.format_solution(best_solution),
        }
    }

    fn finalize_step_state(&self, state: Self::StepState) -> Self::SolutionSet {
        VectorSolutionSet::from_vec(state.population)
    }

    fn checkpoint_algorithm_parameters(&self) -> String {
        format!(
            "population_size={};crossover_rate={:.6};differential_weight={:.6};termination={:?}",
            self.parameters.population_size,
            self.parameters.crossover_rate,
            self.parameters.differential_weight,
            self.parameters.termination_criteria
        )
    }
}

impl<P> ExperimentalCase<f64, f64, P> for DifferentialEvolutionParameters
where
    P: Problem<f64, f64> + Sync,
{
    fn algorithm_name(&self) -> &str {
        "DifferentialEvolution"
    }

    fn case_name(&self) -> String {
        format!(
            "DifferentialEvolution(pop={}, cr={:.4}, f={:.4})",
            self.population_size, self.crossover_rate, self.differential_weight
        )
    }

    fn parameters(&self) -> Vec<CaseParameter> {
        vec![
            CaseParameter::new("population_size", self.population_size.to_string()),
            CaseParameter::new("crossover_rate", format!("{:.6}", self.crossover_rate)),
            CaseParameter::new(
                "differential_weight",
                format!("{:.6}", self.differential_weight),
            ),
            CaseParameter::new(
                "termination_criteria",
                format!("{:?}", self.termination_criteria),
            ),
        ]
    }

    fn run(&self, problem: &P) -> Result<Box<dyn SolutionSet<f64, f64>>, String> {
        let result = DifferentialEvolution::new(self.clone()).run(problem)?;
        Ok(Box::new(result))
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::problem::AckleyProblem;
    use crate::solution_set::traits::SolutionSet;
    use crate::TerminationCriterion;

    #[test]
    fn de_rejects_too_small_population() {
        let parameters = DifferentialEvolutionParameters::new(
            3,
            0.9,
            0.7,
            TerminationCriteria::new(vec![TerminationCriterion::MaxIterations(5)]),
        );
        let algorithm = DifferentialEvolution::new(parameters);

        assert_eq!(
            algorithm.validate_parameters(),
            Err("population_size must be >= 4".to_string())
        );
    }

    #[test]
    fn de_runs_on_ackley_and_returns_bounded_population() {
        let problem = AckleyProblem::new(8, -5.0, 5.0);
        let parameters = DifferentialEvolutionParameters::new(
            16,
            0.9,
            0.8,
            TerminationCriteria::new(vec![TerminationCriterion::MaxIterations(15)]),
        )
        .with_seed(19);

        let mut algorithm = DifferentialEvolution::new(parameters);
        let result = algorithm.run(&problem).expect("DE on Ackley should succeed");

        assert_eq!(result.size(), 16);
        for solution in result.iter() {
            assert_eq!(solution.num_variables(), 8);
            assert!(solution.variables().iter().all(|value| (-5.0..=5.0).contains(value)));
            assert!(solution.quality_value().is_finite());
        }
    }
}