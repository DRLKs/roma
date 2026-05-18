use std::fmt::Display;
use std::str::FromStr;

use crate::algorithms::checkpoint::{ExecutionStateSnapshot, StepStateCheckpoint};
use crate::algorithms::termination::{TerminationCriteria};
use crate::algorithms::traits::Algorithm;
use crate::experiment::traits::{CaseParameter, ExperimentalCase};
use crate::observer::traits::AlgorithmObserver;
use crate::operator::traits::{CrossoverOperator, MutationOperator, SelectionOperator};
use crate::problem::traits::Problem;
use crate::solution::Solution;
use crate::solution_set::implementations::vector_solution_set::VectorSolutionSet;
use crate::solution_set::traits::SolutionSet;
use crate::utils::parallel::parallel_map_indexed;
use crate::utils::parallel::resolve_num_threads;
use crate::utils::random::{seed_from_time, Random};
use crate::utils::statistics::calculate_population_statistics;
use crate::Observable;

#[derive(Clone)]
pub struct GeneticAlgorithmParameters<T, C, M, Sel>
where
    T: Clone,
    C: CrossoverOperator<T>,
    M: MutationOperator<T>,
    Sel: SelectionOperator<T>,
{
    pub population_size: usize,
    pub crossover_probability: f64,
    pub mutation_probability: f64,
    pub elite_size: usize,
    pub crossover_operator: C,
    pub mutation_operator: M,
    pub selection_operator: Sel,
    pub num_threads: Option<usize>,
    pub random_seed: Option<u64>,
    pub termination_criteria: TerminationCriteria,
    _phantom: std::marker::PhantomData<T>,
}

impl<T, C, M, Sel> GeneticAlgorithmParameters<T, C, M, Sel>
where
    T: Clone,
    C: CrossoverOperator<T>,
    M: MutationOperator<T>,
    Sel: SelectionOperator<T>,
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
        GeneticAlgorithmParameters {
            population_size,
            crossover_probability,
            mutation_probability,
            elite_size: 0,
            crossover_operator,
            mutation_operator,
            selection_operator,
            num_threads: None,
            random_seed: None,
            termination_criteria,
            _phantom: std::marker::PhantomData,
        }
    }

    pub fn show_parameters_information(&self) {
        println!("Starting Genetic Algorithm");
        println!("  Population size: {}", self.population_size);
        println!("  Termination criteria: {:?}", self.termination_criteria);
        println!("  Crossover: {}", self.crossover_operator.name());
        println!("  Mutation: {}", self.mutation_operator.name());
        println!("  Selection: {}", self.selection_operator.name());
        println!("  Elitism: {}", self.elite_size);
    }

    pub fn with_elite_size(mut self, elite_size: usize) -> Self {
        self.elite_size = elite_size;
        self
    }

    pub fn with_threads(mut self, num_threads: usize) -> Self {
        self.num_threads = Some(num_threads);
        self
    }

    pub fn with_parallel(mut self) -> Self {
        self.num_threads = None;
        self
    }

    pub fn with_seed(mut self, seed: u64) -> Self {
        self.random_seed = Some(seed);
        self
    }

    pub fn sequential(mut self) -> Self {
        self.num_threads = Some(1);
        self
    }
}

pub struct GeneticAlgorithm<T, C, M, Sel>
where
    T: Clone,
    C: CrossoverOperator<T>,
    M: MutationOperator<T>,
    Sel: SelectionOperator<T>,
{
    parameters: GeneticAlgorithmParameters<T, C, M, Sel>,
    solution_set: Option<VectorSolutionSet<T>>,
    observers: Vec<Box<dyn AlgorithmObserver<T>>>,
}

pub struct GeneticAlgorithmState<T>
where
    T: Clone,
{
    population: Vec<Solution<T>>,
    generation: usize,
    evaluations: usize,
    run_seed: u64,
}

impl<T> StepStateCheckpoint<T> for GeneticAlgorithmState<T>
where
    T: Clone + Display + FromStr,
{
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
        let encoded_pop = self
            .population
            .iter()
            .map(|sol| sol.encode())
            .collect::<Vec<String>>()
            .join(",");

        format!(
            "iter={};eval={};seed={};pop=[{}]",
            self.generation, self.evaluations, self.run_seed, encoded_pop
        )
    }

    fn from_payload(payload: &str) -> Self {
        let parts: std::collections::HashMap<&str, &str> = payload
            .split(';')
            .filter_map(|s| {
                let mut kv = s.splitn(2, '=');
                Some((kv.next()?, kv.next()?))
            })
            .collect();

        let generation = parts.get("iter").and_then(|s| s.parse().ok()).unwrap_or(0);
        let evaluations = parts.get("eval").and_then(|s| s.parse().ok()).unwrap_or(0);
        let run_seed = parts
            .get("seed")
            .and_then(|s| s.parse().ok())
            .unwrap_or_else(seed_from_time);

        let population = parts
            .get("pop")
            .map(|pop_str| {
                pop_str
                    .trim_matches(|c| c == '[' || c == ']')
                    .split(',')
                    .filter(|s| !s.is_empty())
                    .filter_map(|sol_str| Solution::decode(sol_str).ok())
                    .collect::<Vec<_>>()
            })
            .unwrap_or_default();

        Self {
            population,
            generation,
            evaluations,
            run_seed,
        }
    }
}

impl<T, C, M, Sel> GeneticAlgorithm<T, C, M, Sel>
where
    T: Clone + Send + Sync + 'static + Display,
    C: CrossoverOperator<T> + Send + Sync,
    M: MutationOperator<T> + Send + Sync,
    Sel: SelectionOperator<T> + Send + Sync,
{
    fn initialize_population(
        parameters: &GeneticAlgorithmParameters<T, C, M, Sel>,
        problem: &(impl Problem<T> + Sync),
        rng: &mut Random,
    ) -> Vec<Solution<T>> {
        let initial_population: Vec<Solution<T>> = (0..parameters.population_size)
            .map(|_| problem.create_solution(rng))
            .collect();

        Self::evaluate_population(initial_population, problem, parameters.num_threads)
    }

    fn evaluate_population(
        population: Vec<Solution<T>>,
        problem: &(impl Problem<T> + Sync),
        requested_threads: Option<usize>,
    ) -> Vec<Solution<T>> {
        let thread_count = resolve_num_threads(requested_threads);

        if thread_count <= 1 {
            let mut evaluated = population;
            for solution in &mut evaluated {
                problem.evaluate(solution);
            }
            return evaluated;
        }

        parallel_map_indexed(&population, Some(thread_count), 1, |_, solution| {
            let mut evaluated = solution.copy();
            problem.evaluate(&mut evaluated);
            evaluated
        })
    }

    fn next_generation(
        parameters: &GeneticAlgorithmParameters<T, C, M, Sel>,
        problem: &(impl Problem<T> + Sync),
        current_population: &[Solution<T>],
        generation: usize,
        run_seed: u64,
        evaluations: &mut usize,
    ) -> Vec<Solution<T>> {
        let generation_seed = Random::derive_seed(run_seed, generation as u64);
        let mut offspring = Self::create_offspring(
            parameters,
            problem,
            current_population,
            generation_seed,
            evaluations,
        );
        Self::apply_elitism(parameters, problem, current_population, &mut offspring);
        Self::sort_population(problem, &mut offspring);
        offspring.truncate(parameters.population_size);
        offspring
    }

    fn create_offspring(
        parameters: &GeneticAlgorithmParameters<T, C, M, Sel>,
        problem: &(impl Problem<T> + Sync),
        population: &[Solution<T>],
        generation_seed: u64,
        evaluations: &mut usize,
    ) -> Vec<Solution<T>> {
        let requested_threads = resolve_num_threads(parameters.num_threads);
        let thread_count = requested_threads.min(parameters.population_size.max(1));

        let (mut offspring_population, generation_evaluations) = if thread_count <= 1 {
            Self::create_offspring_sequential(
                parameters,
                problem,
                population,
                generation_seed,
            )
        } else {
            Self::create_offspring_parallel(
                parameters,
                problem,
                population,
                thread_count,
                generation_seed,
            )
        };

        *evaluations += generation_evaluations;
        offspring_population.truncate(parameters.population_size);
        offspring_population
    }

    fn create_offspring_sequential(
        parameters: &GeneticAlgorithmParameters<T, C, M, Sel>,
        problem: &(impl Problem<T> + Sync),
        population: &[Solution<T>],
        generation_seed: u64,
    ) -> (Vec<Solution<T>>, usize) {
        let mut offspring_population = Vec::with_capacity(parameters.population_size);
        let mut generation_evaluations = 0usize;
        let mut rng = Random::new(generation_seed);
        let real_bounds = problem.real_bounds();

        while offspring_population.len() < parameters.population_size {
            let parent1 = parameters
                .selection_operator
                .execute(population, &mut rng, &|a, b| problem.dominates(a, b));
            let parent2 = parameters
                .selection_operator
                .execute(population, &mut rng, &|a, b| problem.dominates(a, b));

            let mut offspring = if rng.next_f64() < parameters.crossover_probability {
                parameters
                    .crossover_operator
                    .execute(
                        &parent1,
                        &parent2,
                        real_bounds,
                        &mut rng,
                    )
            } else {
                vec![parent1.copy(), parent2.copy()]
            };

            for child in &mut offspring {
                parameters.mutation_operator.execute(
                    child,
                    parameters.mutation_probability,
                    real_bounds,
                    &mut rng,
                );
                problem.evaluate(child);
                generation_evaluations += 1;
            }

            offspring_population.extend(offspring);
        }

        (offspring_population, generation_evaluations)
    }

    /// IMPORTANT:
    ///
    ///  Parallelism breaks determinism if you work on machines with different numbers of cores.
    /// - An 8-core machine will generate a different output than a 16-core machine.
    fn create_offspring_parallel(
        parameters: &GeneticAlgorithmParameters<T, C, M, Sel>,
        problem: &(impl Problem<T> + Sync),
        population: &[Solution<T>],
        thread_count: usize,
        generation_seed: u64,
    ) -> (Vec<Solution<T>>, usize) {
        let target_size = parameters.population_size;
        let mut all_offspring = Vec::with_capacity(target_size);
        let mut total_evaluations = 0usize;

        std::thread::scope(|scope| {
            let mut handles = Vec::with_capacity(thread_count);

            for worker_id in 0..thread_count {
                let start = worker_id * target_size / thread_count;
                let end = (worker_id + 1) * target_size / thread_count;
                let worker_target = end.saturating_sub(start);

                handles.push(scope.spawn(move || {
                    if worker_target == 0 {
                        return (Vec::new(), 0usize);
                    }

                    let worker_seed = Random::derive_seed(generation_seed, worker_id as u64 + 1);
                    let mut local_rng = Random::new(worker_seed);
                    let mut local_offspring = Vec::with_capacity(worker_target);
                    let mut local_evaluations = 0usize;
                    let real_bounds = problem.real_bounds();

                    while local_offspring.len() < worker_target {
                        let parent1 = parameters.selection_operator.execute(
                            population,
                            &mut local_rng,
                            &|a, b| problem.dominates(a, b),
                        );
                        let parent2 = parameters.selection_operator.execute(
                            population,
                            &mut local_rng,
                            &|a, b| problem.dominates(a, b),
                        );

                        let mut children =
                            if local_rng.next_f64() < parameters.crossover_probability {
                                parameters.crossover_operator.execute(
                                    &parent1,
                                    &parent2,
                                    real_bounds,
                                    &mut local_rng,
                                )
                            } else {
                                vec![parent1.copy(), parent2.copy()]
                            };

                        for child in &mut children {
                            parameters.mutation_operator.execute(
                                child,
                                parameters.mutation_probability,
                                real_bounds,
                                &mut local_rng,
                            );
                            problem.evaluate(child);
                            local_evaluations += 1;
                        }

                        local_offspring.extend(children);
                    }

                    local_offspring.truncate(worker_target);
                    (local_offspring, local_evaluations)
                }));
            }

            for handle in handles {
                if let Ok((mut worker_offspring, worker_evaluations)) = handle.join() {
                    total_evaluations += worker_evaluations;
                    all_offspring.append(&mut worker_offspring);
                }
            }
        });

        (all_offspring, total_evaluations)
    }

    fn apply_elitism(
        parameters: &GeneticAlgorithmParameters<T, C, M, Sel>,
        problem: &(impl Problem<T> + Sync),
        current_population: &[Solution<T>],
        next_population: &mut Vec<Solution<T>>,
    ) {
        if parameters.elite_size == 0 || current_population.is_empty() {
            return;
        }

        let elite_count = parameters
            .elite_size
            .min(parameters.population_size)
            .min(current_population.len());

        let mut elite_indices: Vec<usize> = (0..current_population.len()).collect();
        elite_indices.sort_by(|&ia, &ib| {
            let a = &current_population[ia];
            let b = &current_population[ib];
            if problem.dominates(a, b) {
                std::cmp::Ordering::Less
            } else if problem.dominates(b, a) {
                std::cmp::Ordering::Greater
            } else {
                std::cmp::Ordering::Equal
            }
        });
        elite_indices.truncate(elite_count);

        Self::sort_population(problem, next_population);
        next_population.truncate(parameters.population_size.saturating_sub(elite_count));
        next_population.extend(
            elite_indices
                .into_iter()
                .map(|idx| current_population[idx].copy()),
        );
    }

    fn sort_population(problem: &(impl Problem<T> + Sync), population: &mut [Solution<T>]) {
        population.sort_by(|a, b| {
            let ordering = if problem.dominates(a, b) {
                std::cmp::Ordering::Less
            } else if problem.dominates(b, a) {
                std::cmp::Ordering::Greater
            } else {
                std::cmp::Ordering::Equal
            };

            ordering
        });
    }
}

impl<T, C, M, Sel> Observable<T> for GeneticAlgorithm<T, C, M, Sel>
where
    T: Clone + Send + 'static,
    C: CrossoverOperator<T>,
    M: MutationOperator<T>,
    Sel: SelectionOperator<T>,
{
    fn add_observer(&mut self, observer: Box<dyn AlgorithmObserver<T>>) {
        self.observers.push(observer);
    }

    fn clear_observers(&mut self) {
        self.observers.clear();
    }
}

impl<T, C, M, Sel> Algorithm<T> for GeneticAlgorithm<T, C, M, Sel>
where
    T: Clone + Send + Sync + 'static + Display + Display + FromStr,
    C: CrossoverOperator<T> + Send + Sync,
    M: MutationOperator<T> + Send + Sync,
    Sel: SelectionOperator<T> + Send + Sync,
{
    type SolutionSet = VectorSolutionSet<T>;
    type Parameters = GeneticAlgorithmParameters<T, C, M, Sel>;
    type StepState = GeneticAlgorithmState<T>;

    fn new(parameters: GeneticAlgorithmParameters<T, C, M, Sel>) -> Self {
        GeneticAlgorithm {
            parameters,
            solution_set: None,
            observers: Vec::new(),
        }
    }

    fn algorithm_name(&self) -> &str {
        "GeneticAlgorithm"
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

        if self.parameters.elite_size > self.parameters.population_size {
            return Err("elite_size must be <= population_size".to_string());
        }

        Ok(())
    }

    fn get_solution_set(&self) -> Option<&Self::SolutionSet> {
        self.solution_set.as_ref()
    }

    fn initialize_step_state(&self, problem: &(impl Problem<T> + Sync)) -> Self::StepState {
        let run_seed = Random::resolve_seed(self.parameters.random_seed);
        let mut init_rng = Random::new(Random::derive_seed(run_seed, 0));
        let population = Self::initialize_population(&self.parameters, problem, &mut init_rng);

        GeneticAlgorithmState {
            population,
            generation: 0,
            evaluations: self.parameters.population_size,
            run_seed,
        }
    }

    fn step(
        &self,
        problem: &(impl Problem<T> + Sync),
        state: &mut Self::StepState,
    ) {
        state.generation += 1;
        state.population = Self::next_generation(
            &self.parameters,
            problem,
            &state.population,
            state.generation,
            state.run_seed,
            &mut state.evaluations,
        );
    }

    fn build_snapshot(
        &self,
        problem: &(impl Problem<T> + Sync),
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
}

impl<T, C, M, Sel, P> ExperimentalCase<T, f64, P> for GeneticAlgorithmParameters<T, C, M, Sel>
where
    T: Clone + Send + Sync + 'static + Display + FromStr,
    C: CrossoverOperator<T> + Clone + Send + Sync + 'static,
    M: MutationOperator<T> + Clone + Send + Sync + 'static,
    Sel: SelectionOperator<T> + Clone + Send + Sync + 'static,
    P: Problem<T, f64> + Sync,
{
    fn algorithm_name(&self) -> &str {
        "GeneticAlgorithm"
    }

    fn case_name(&self) -> String {
        format!(
            "{}(pop={}, cx={:.3}, mut={:.3}, elite={})",
            "GeneticAlgorithm",
            self.population_size,
            self.crossover_probability,
            self.mutation_probability,
            self.elite_size,
        )
    }

    fn parameters(&self) -> Vec<CaseParameter> {
        let threads_text = match self.num_threads {
            Some(v) => v.to_string(),
            None => "auto".to_string(),
        };

        vec![
            CaseParameter::new("population_size", self.population_size.to_string()),
            CaseParameter::new(
                "crossover_probability",
                format!("{:.6}", self.crossover_probability),
            ),
            CaseParameter::new(
                "mutation_probability",
                format!("{:.6}", self.mutation_probability),
            ),
            CaseParameter::new("elite_size", self.elite_size.to_string()),
            CaseParameter::new("threads", threads_text),
            CaseParameter::new("crossover_operator", self.crossover_operator.name()),
            CaseParameter::new("mutation_operator", self.mutation_operator.name()),
            CaseParameter::new("selection_operator", self.selection_operator.name()),
            CaseParameter::new(
                "termination_criteria",
                format!("{:?}", self.termination_criteria),
            ),
        ]
    }

    fn run(&self, problem: &P) -> Result<Box<dyn SolutionSet<T, f64>>, String> {
        let mut algorithm = GeneticAlgorithm::new(self.clone());
        let result = algorithm.run(problem)?;
        Ok(Box::new(result))
    }
}
