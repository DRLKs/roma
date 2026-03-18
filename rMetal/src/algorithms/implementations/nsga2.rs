use crate::algorithms::termination::{
    ExecutionStateSnapshot,
    TerminationCriteria,
};
use crate::algorithms::traits::Algorithm;
use crate::algorithms::runtime::ExecutionContext;
use crate::observer::traits::{AlgorithmObserver, Observable};
use crate::operator::traits::{CrossoverOperator, MutationOperator, SelectionOperator};
use crate::problem::traits::Problem;
use crate::solution::ParetoCrowdingDistanceQuality;
use crate::solution_set::implementations::vector_solution_set::VectorSolutionSet;
use crate::utils::random::{seed_from_time, Random};

pub struct NSGAIIParameters<C, M, Sel>
where
    C: CrossoverOperator<f64, ParetoCrowdingDistanceQuality>,
    M: MutationOperator<f64, ParetoCrowdingDistanceQuality>,
    Sel: SelectionOperator<f64, ParetoCrowdingDistanceQuality>,
{
    pub population_size: usize,
    pub crossover_probability: f64,
    pub mutation_probability: f64,
    pub crossover_operator: C,
    pub mutation_operator: M,
    pub selection_operator: Sel,
    pub random_seed: Option<u64>,
    pub termination_criteria: TerminationCriteria,
}

impl<C, M, Sel> NSGAIIParameters<C, M, Sel>
where
    C: CrossoverOperator<f64, ParetoCrowdingDistanceQuality>,
    M: MutationOperator<f64, ParetoCrowdingDistanceQuality>,
    Sel: SelectionOperator<f64, ParetoCrowdingDistanceQuality>,
{
    pub fn new(
        population_size: usize,
        crossover_probability: f64,
        mutation_probability: f64,
        crossover_operator: C,
        mutation_operator: M,
        selection_operator: Sel,
        termination_criteria: TerminationCriteria,
    ) -> Self {
        Self {
            population_size,
            crossover_probability,
            mutation_probability,
            crossover_operator,
            mutation_operator,
            selection_operator,
            random_seed: None,
            termination_criteria,
        }
    }

    pub fn with_seed(mut self, seed: u64) -> Self {
        self.random_seed = Some(seed);
        self
    }
}

pub struct NSGAII<C, M, Sel>
where
    C: CrossoverOperator<f64, ParetoCrowdingDistanceQuality>,
    M: MutationOperator<f64, ParetoCrowdingDistanceQuality>,
    Sel: SelectionOperator<f64, ParetoCrowdingDistanceQuality>,
{
    parameters: NSGAIIParameters<C, M, Sel>,
    solution_set: Option<VectorSolutionSet<f64, ParetoCrowdingDistanceQuality>>,
    observers: Vec<Box<dyn AlgorithmObserver<f64, ParetoCrowdingDistanceQuality>>>,
}

pub struct NSGAIIState {
    population: Vec<crate::solution::Solution<f64, ParetoCrowdingDistanceQuality>>,
    rng: Random,
    generation: usize,
    evaluations: usize,
}

impl<C, M, Sel> Observable<f64, ParetoCrowdingDistanceQuality> for NSGAII<C, M, Sel>
where
    C: CrossoverOperator<f64, ParetoCrowdingDistanceQuality>,
    M: MutationOperator<f64, ParetoCrowdingDistanceQuality>,
    Sel: SelectionOperator<f64, ParetoCrowdingDistanceQuality>,
{
    fn add_observer(&mut self, observer: Box<dyn AlgorithmObserver<f64, ParetoCrowdingDistanceQuality>>) {
        self.observers.push(observer);
    }

    fn clear_observers(&mut self) {
        self.observers.clear();
    }
}

impl<C, M, Sel> Algorithm<f64, ParetoCrowdingDistanceQuality> for NSGAII<C, M, Sel>
where
    C: CrossoverOperator<f64, ParetoCrowdingDistanceQuality>,
    M: MutationOperator<f64, ParetoCrowdingDistanceQuality>,
    Sel: SelectionOperator<f64, ParetoCrowdingDistanceQuality>,
{
    type SolutionSet = VectorSolutionSet<f64, ParetoCrowdingDistanceQuality>;
    type Parameters = NSGAIIParameters<C, M, Sel>;
    type StepState = NSGAIIState;

    fn new(parameters: NSGAIIParameters<C, M, Sel>) -> Self {
        Self {
            parameters,
            solution_set: None,
            observers: Vec::new(),
        }
    }
    
    fn algorithm_name(&self) -> &str {
        "NSGA-II"
    }

    fn termination_criteria(&self) -> TerminationCriteria {
        self.parameters.termination_criteria.clone()
    }

    fn observers_mut(&mut self) -> &mut Vec<Box<dyn AlgorithmObserver<f64, ParetoCrowdingDistanceQuality>>> {
        &mut self.observers
    }

    fn set_solution_set(&mut self, solution_set: Self::SolutionSet) {
        self.solution_set = Some(solution_set);
    }

    fn validate_parameters(&self) -> Result<(), String> {
        if self.parameters.population_size == 0 {
            return Err("population_size must be > 0".to_string());
        }

        if self.parameters.termination_criteria.is_empty() {
            return Err("termination_criteria must not be empty".to_string());
        }

        if !(0.0..=1.0).contains(&self.parameters.crossover_probability) {
            return Err("crossover_probability must be in [0,1]".to_string());
        }

        if !(0.0..=1.0).contains(&self.parameters.mutation_probability) {
            return Err("mutation_probability must be in [0,1]".to_string());
        }

        Ok(())
    }

    fn get_solution_set(&self) -> Option<&Self::SolutionSet> {
        self.solution_set.as_ref()
    }

    fn initialize_step_state(
        &self,
        problem: &(impl Problem<f64, ParetoCrowdingDistanceQuality> + Sync),
        _context: &ExecutionContext<f64, ParetoCrowdingDistanceQuality>,
    ) -> Self::StepState {
        let mut rng = Random::new(self.parameters.random_seed.unwrap_or_else(seed_from_time));

        let population: Vec<_> = (0..self.parameters.population_size)
            .map(|_| {
                let mut solution = problem.create_solution(&mut rng);
                problem.evaluate(&mut solution);
                solution
            })
            .collect();

        NSGAIIState {
            population,
            rng,
            generation: 0,
            evaluations: self.parameters.population_size,
        }
    }

    fn step(
        &self,
        problem: &(impl Problem<f64, ParetoCrowdingDistanceQuality> + Sync),
        state: &mut Self::StepState,
        _context: &ExecutionContext<f64, ParetoCrowdingDistanceQuality>,
    ) {
        state.generation += 1;
        let mut offspring = Vec::with_capacity(self.parameters.population_size);

        while offspring.len() < self.parameters.population_size {
            let parent1 = self
                .parameters
                .selection_operator
                .execute(&state.population, &mut state.rng)
                .copy();
            let parent2 = self
                .parameters
                .selection_operator
                .execute(&state.population, &mut state.rng)
                .copy();

            let mut children = if state.rng.next_f64() < self.parameters.crossover_probability {
                self.parameters
                    .crossover_operator
                    .execute(&parent1, &parent2, &mut state.rng)
            } else {
                vec![parent1, parent2]
            };

            for child in &mut children {
                self.parameters.mutation_operator.execute(
                    child,
                    self.parameters.mutation_probability,
                    &mut state.rng,
                );
                problem.evaluate(child);
                state.evaluations += 1;
            }

            offspring.extend(children);
        }

        offspring.truncate(self.parameters.population_size);
        state.population.extend(offspring);

        state.population.sort_by(|a, b| {
            a.get_objective(0)
                .unwrap_or(f64::INFINITY)
                .partial_cmp(&b.get_objective(0).unwrap_or(f64::INFINITY))
                .unwrap_or(std::cmp::Ordering::Equal)
        });
        state.population.truncate(self.parameters.population_size);
    }

    fn snapshot(&self, state: &Self::StepState) -> ExecutionStateSnapshot<f64, ParetoCrowdingDistanceQuality> {
        let worst = state
            .population
            .last()
            .and_then(|s| s.get_objective(0))
            .unwrap_or(0.0);
        let avg = if state.population.is_empty() {
            0.0
        } else {
            let values: Vec<f64> = state.population.iter().filter_map(|s| s.get_objective(0)).collect();
            if values.is_empty() {
                0.0
            } else {
                values.iter().sum::<f64>() / values.len() as f64
            }
        };

        let best_solution = state
            .population
            .first()
            .map(|solution| solution.copy())
            .expect("population should not be empty when reporting progress");
        let best = best_solution.get_objective(0).unwrap_or(0.0);

        ExecutionStateSnapshot::new(
            0,
            state.generation,
            state.evaluations,
            best_solution,
            best,
            avg,
            worst,
        )
    }

    fn finalize_step_state(&self, state: Self::StepState) -> Self::SolutionSet {
        VectorSolutionSet::from_vec(state.population)
    }
}
