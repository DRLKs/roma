use crate::algorithms::objective::{is_better, ImprovementDirection};
use crate::algorithms::runtime::ExecutionContext;
use crate::algorithms::termination::{ExecutionStateSnapshot, TerminationCriteria};
use crate::algorithms::traits::Algorithm;
use crate::experiment::traits::{CaseParameter, ExperimentalCase};
use crate::Observable;
use crate::observer::traits::AlgorithmObserver;
use crate::operator::traits::{CrossoverOperator, MutationOperator, SelectionOperator};
use crate::problem::traits::Problem;
use crate::solution::Solution;
use crate::solution_set::implementations::vector_solution_set::VectorSolutionSet;
use crate::solution_set::traits::SolutionSet;
use crate::utils::parallel::parallel_map_indexed;
use crate::utils::random::{seed_from_time, Random};
use crate::utils::statistics::calculate_statistics;

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
    direction: ImprovementDirection,
    generation: usize,
    evaluations: usize,
    run_seed: u64,
}

impl<T, C, M, Sel> GeneticAlgorithm<T, C, M, Sel>
where
    T: Clone + Send + Sync + 'static,
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
        let thread_count = Self::resolve_num_threads(requested_threads);

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
        direction: ImprovementDirection,
        generation: usize,
        run_seed: u64,
        evaluations: &mut usize,
    ) -> Vec<Solution<T>> {
        let generation_seed = Self::derive_seed(run_seed, generation as u64);
        let mut offspring = Self::create_offspring(
            parameters,
            problem,
            current_population,
            direction,
            generation_seed,
            evaluations,
        );
        Self::apply_elitism(parameters, current_population, &mut offspring, direction);
        Self::sort_population(&mut offspring, direction);
        offspring.truncate(parameters.population_size);
        offspring
    }

    fn create_offspring(
        parameters: &GeneticAlgorithmParameters<T, C, M, Sel>,
        problem: &(impl Problem<T> + Sync),
        population: &[Solution<T>],
        direction: ImprovementDirection,
        generation_seed: u64,
        evaluations: &mut usize,
    ) -> Vec<Solution<T>> {
        let requested_threads = Self::resolve_num_threads(parameters.num_threads);
        let thread_count = requested_threads.min(parameters.population_size.max(1));

        let (mut offspring_population, generation_evaluations) = if thread_count <= 1 {
            Self::create_offspring_sequential(
                parameters,
                problem,
                population,
                direction,
                generation_seed,
            )
        } else {
            Self::create_offspring_parallel(
                parameters,
                problem,
                population,
                direction,
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
        direction: ImprovementDirection,
        generation_seed: u64,
    ) -> (Vec<Solution<T>>, usize) {
        let mut offspring_population = Vec::with_capacity(parameters.population_size);
        let mut generation_evaluations = 0usize;
        let mut rng = Random::new(generation_seed);

        while offspring_population.len() < parameters.population_size {
            let parent1 = parameters
                .selection_operator
                .execute(population, &mut rng, direction);
            let parent2 = parameters
                .selection_operator
                .execute(population, &mut rng, direction);

            let mut offspring = if rng.next_f64() < parameters.crossover_probability {
                parameters
                    .crossover_operator
                    .execute(&parent1, &parent2, &mut rng)
            } else {
                vec![parent1.copy(), parent2.copy()]
            };

            for child in &mut offspring {
                parameters.mutation_operator.execute(
                    child,
                    parameters.mutation_probability,
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
        direction: ImprovementDirection,
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

                    let worker_seed = Self::derive_seed(generation_seed, worker_id as u64 + 1);
                    let mut local_rng = Random::new(worker_seed);
                    let mut local_offspring = Vec::with_capacity(worker_target);
                    let mut local_evaluations = 0usize;

                    while local_offspring.len() < worker_target {
                        let parent1 = parameters.selection_operator.execute(
                            population,
                            &mut local_rng,
                            direction,
                        );
                        let parent2 = parameters.selection_operator.execute(
                            population,
                            &mut local_rng,
                            direction,
                        );

                        let mut children =
                            if local_rng.next_f64() < parameters.crossover_probability {
                                parameters.crossover_operator.execute(
                                    &parent1,
                                    &parent2,
                                    &mut local_rng,
                                )
                            } else {
                                vec![parent1.copy(), parent2.copy()]
                            };

                        for child in &mut children {
                            parameters.mutation_operator.execute(
                                child,
                                parameters.mutation_probability,
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

    fn resolve_num_threads(num_threads: Option<usize>) -> usize {
        match num_threads {
            Some(n) => n.max(1),
            None => std::thread::available_parallelism()
                .map(|n| n.get())
                .unwrap_or(1),
        }
    }

    fn resolve_seed(parameters: &GeneticAlgorithmParameters<T, C, M, Sel>) -> u64 {
        parameters.random_seed.unwrap_or_else(seed_from_time)
    }

    fn derive_seed(base_seed: u64, stream: u64) -> u64 {
        let mut z = base_seed ^ stream.wrapping_mul(0x9E37_79B9_7F4A_7C15);
        z = (z ^ (z >> 30)).wrapping_mul(0xBF58_476D_1CE4_E5B9);
        z = (z ^ (z >> 27)).wrapping_mul(0x94D0_49BB_1331_11EB);
        z ^ (z >> 31)
    }

    fn apply_elitism(
        parameters: &GeneticAlgorithmParameters<T, C, M, Sel>,
        current_population: &[Solution<T>],
        next_population: &mut Vec<Solution<T>>,
        direction: ImprovementDirection,
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
            let a_val = a.quality_value();
            let b_val = b.quality_value();

            if is_better(a_val, b_val, direction) {
                std::cmp::Ordering::Less
            } else if is_better(b_val, a_val, direction) {
                std::cmp::Ordering::Greater
            } else {
                std::cmp::Ordering::Equal
            }
        });
        elite_indices.truncate(elite_count);

        Self::sort_population(next_population, direction);
        next_population.truncate(parameters.population_size.saturating_sub(elite_count));
        next_population.extend(
            elite_indices
                .into_iter()
                .map(|idx| current_population[idx].copy()),
        );
    }

    fn sort_population(population: &mut [Solution<T>], direction: ImprovementDirection) {
        population.sort_by(|a, b| {
            let a_val = a.quality_value();
            let b_val = b.quality_value();
            let ordering = if is_better(a_val, b_val, direction) {
                std::cmp::Ordering::Less
            } else if is_better(b_val, a_val, direction) {
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
    T: Clone + Send + Sync + 'static,
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

    fn initialize_step_state(
        &self,
        problem: &(impl Problem<T> + Sync),
        _context: &ExecutionContext<T>,
    ) -> Self::StepState {
        let run_seed = Self::resolve_seed(&self.parameters);
        let mut init_rng = Random::new(Self::derive_seed(run_seed, 0));
        let population = Self::initialize_population(&self.parameters, problem, &mut init_rng);
        let direction = problem.get_improvement_direction();

        GeneticAlgorithmState {
            population,
            direction,
            generation: 0,
            evaluations: self.parameters.population_size,
            run_seed,
        }
    }

    fn step(
        &self,
        problem: &(impl Problem<T> + Sync),
        state: &mut Self::StepState,
        _context: &ExecutionContext<T>,
    ) {
        state.generation += 1;
        state.population = Self::next_generation(
            &self.parameters,
            problem,
            &state.population,
            state.direction,
            state.generation,
            state.run_seed,
            &mut state.evaluations,
        );
    }

    fn snapshot(&self, state: &Self::StepState) -> ExecutionStateSnapshot<T> {
        let (_best, avg, worst) = calculate_statistics(&state.population, state.direction);
        let best_solution = state
            .population
            .iter()
            .cloned()
            .reduce(|best, candidate| {
                if is_better(
                    candidate.quality_value(),
                    best.quality_value(),
                    state.direction,
                ) {
                    candidate
                } else {
                    best
                }
            })
            .expect("population should not be empty when reporting progress");

        let best_fitness = best_solution.quality_value();
        ExecutionStateSnapshot::new(
            0,
            state.generation,
            state.evaluations,
            best_solution,
            best_fitness,
            avg,
            worst,
        )
    }

    fn finalize_step_state(&self, state: Self::StepState) -> Self::SolutionSet {
        VectorSolutionSet::from_vec(state.population)
    }
}


impl<T, C, M, Sel, P> ExperimentalCase<T, f64, P> for GeneticAlgorithmParameters<T, C, M, Sel>
where
    T: Clone + Send + Sync + 'static,
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
            CaseParameter::new(
                "population_size",
                self.population_size.to_string(),
            ),
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
            CaseParameter::new(
                "crossover_operator",
                self.crossover_operator.name(),
            ),
            CaseParameter::new(
                "mutation_operator",
                self.mutation_operator.name(),
            ),
            CaseParameter::new(
                "selection_operator",
                self.selection_operator.name(),
            ),
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
