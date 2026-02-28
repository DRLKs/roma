use crate::algorithms::traits::Algorithm;
use crate::observer::runtime::{run_with_observers_in_worker, ExecutionContext};
use crate::observer::traits::{AlgorithmObserver, Observable};
use crate::observer::AlgorithmEvent;
use crate::operator::traits::{CrossoverOperator, MutationOperator, SelectionOperator};
use crate::problem::traits::Problem;
use crate::solution::Solution;
use crate::solution_set::implementations::vector_solution_set::VectorSolutionSet;
use crate::utils::random::{seed_from_time, Random};
use crate::utils::statistics::calculate_statistics;

pub struct GeneticAlgorithmParameters<T, C, M, Sel>
where
    T: Clone,
    C: CrossoverOperator<T>,
    M: MutationOperator<T>,
    Sel: SelectionOperator<T>,
{
    pub population_size: usize,
    pub max_generations: usize,
    pub crossover_probability: f64,
    pub mutation_probability: f64,
    pub elite_size: usize,
    pub crossover_operator: C,
    pub mutation_operator: M,
    pub selection_operator: Sel,
    pub num_threads: Option<usize>,
    pub random_seed: Option<u64>,
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
        max_generations: usize,
        crossover_probability: f64,
        mutation_probability: f64,
        crossover_operator: C,
        mutation_operator: M,
        selection_operator: Sel,
    ) -> Self {
        GeneticAlgorithmParameters {
            population_size,
            max_generations,
            crossover_probability,
            mutation_probability,
            elite_size: 0,
            crossover_operator,
            mutation_operator,
            selection_operator,
            num_threads: None,
            random_seed: None,
            _phantom: std::marker::PhantomData,
        }
    }

    pub fn show_parameters_information(&self){
        println!("Starting Genetic Algorithm");
        println!("  Population size: {}", self.population_size);
        println!("  Max generations: {}", self.max_generations);
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

impl<T, C, M, Sel> GeneticAlgorithm<T, C, M, Sel>
where
    T: Clone + Send + Sync + 'static,
    C: CrossoverOperator<T> + Send + Sync,
    M: MutationOperator<T> + Send + Sync,
    Sel: SelectionOperator<T> + Send + Sync,
{
    pub fn new(parameters: GeneticAlgorithmParameters<T, C, M, Sel>) -> Self {
        GeneticAlgorithm {
            parameters,
            solution_set: None,
            observers: Vec::new(),
        }
    }

    fn run_internal(
        parameters: &GeneticAlgorithmParameters<T, C, M, Sel>,
        problem: &(impl Problem<T> + Sync),
        context: &ExecutionContext<T>,
    ) -> VectorSolutionSet<T> {
        context.emit(AlgorithmEvent::Start {
            algorithm_name: "GeneticAlgorithm".to_string(),
        });

        let run_seed = Self::resolve_seed(parameters);
        let mut init_rng = Random::new(Self::derive_seed(run_seed, 0));
        let mut population = Self::initialize_population(parameters, problem, &mut init_rng);
        let mut evaluations = parameters.population_size;

        Self::emit_generation_metrics(context, 0, evaluations, &population);

        for generation in 1..=parameters.max_generations {
            population = Self::next_generation(
                parameters,
                problem,
                &population,
                generation,
                run_seed,
                &mut evaluations,
            );

            Self::emit_generation_metrics(context, generation, evaluations, &population);
        }

        context.emit(AlgorithmEvent::End {
            total_generations: parameters.max_generations,
            total_evaluations: evaluations,
        });

        VectorSolutionSet::from_vec(population)
    }

    fn initialize_population(
        parameters: &GeneticAlgorithmParameters<T, C, M, Sel>,
        problem: &(impl Problem<T> + Sync),
        rng: &mut Random,
    ) -> Vec<Solution<T>> {
        (0..parameters.population_size)
            .map(|_| {
                let mut solution = problem.create_solution(rng);
                problem.evaluate(&mut solution);
                solution
            })
            .collect()
    }

    fn next_generation(
        parameters: &GeneticAlgorithmParameters<T, C, M, Sel>,
        problem: &(impl Problem<T> + Sync),
        current_population: &[Solution<T>],
        generation: usize,
        run_seed: u64,
        evaluations: &mut usize,
    ) -> Vec<Solution<T>> {
        let generation_seed = Self::derive_seed(run_seed, generation as u64);
        let mut offspring = Self::create_offspring(
            parameters,
            problem,
            current_population,
            generation_seed,
            evaluations,
        );
        Self::apply_elitism(parameters, current_population, &mut offspring);
        Self::sort_population_desc(&mut offspring);
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
        let requested_threads = Self::resolve_num_threads(parameters);
        let thread_count = requested_threads.min(parameters.population_size.max(1));

        let (mut offspring_population, generation_evaluations) = if thread_count <= 1 {
            Self::create_offspring_sequential(parameters, problem, population, generation_seed)
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

        while offspring_population.len() < parameters.population_size {
            let parent1 = parameters.selection_operator.execute(population, &mut rng).copy();
            let parent2 = parameters.selection_operator.execute(population, &mut rng).copy();

            let mut offspring = if rng.next_f64() < parameters.crossover_probability {
                parameters
                    .crossover_operator
                    .execute(&parent1, &parent2, &mut rng)
            } else {
                vec![parent1, parent2]
            };

            for child in &mut offspring {
                parameters
                    .mutation_operator
                    .execute(child, parameters.mutation_probability, &mut rng);
                problem.evaluate(child);
                generation_evaluations += 1;
            }

            offspring_population.extend(offspring);
        }

        (offspring_population, generation_evaluations)
    }

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

                    let worker_seed = Self::derive_seed(generation_seed, worker_id as u64 + 1);
                    let mut local_rng = Random::new(worker_seed);
                    let mut local_offspring = Vec::with_capacity(worker_target);
                    let mut local_evaluations = 0usize;

                    while local_offspring.len() < worker_target {
                        let parent1 = parameters
                            .selection_operator
                            .execute(population, &mut local_rng)
                            .copy();
                        let parent2 = parameters
                            .selection_operator
                            .execute(population, &mut local_rng)
                            .copy();

                        let mut children = if local_rng.next_f64() < parameters.crossover_probability {
                            parameters
                                .crossover_operator
                                .execute(&parent1, &parent2, &mut local_rng)
                        } else {
                            vec![parent1, parent2]
                        };

                        for child in &mut children {
                            parameters
                                .mutation_operator
                                .execute(child, parameters.mutation_probability, &mut local_rng);
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
                let (mut worker_offspring, worker_evaluations) = handle
                    .join()
                    .expect("worker thread panicked while creating offspring");
                total_evaluations += worker_evaluations;
                all_offspring.append(&mut worker_offspring);
            }
        });

        (all_offspring, total_evaluations)
    }

    fn resolve_num_threads(parameters: &GeneticAlgorithmParameters<T, C, M, Sel>) -> usize {
        match parameters.num_threads {
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
    ) {
        if parameters.elite_size == 0 || current_population.is_empty() {
            return;
        }

        let elite_count = parameters
            .elite_size
            .min(parameters.population_size)
            .min(current_population.len());

        let mut elites = current_population.to_vec();
        Self::sort_population_desc(&mut elites);
        elites.truncate(elite_count);

        Self::sort_population_desc(next_population);
        next_population.truncate(parameters.population_size.saturating_sub(elite_count));
        next_population.extend(elites);
    }

    fn emit_generation_metrics(
        context: &ExecutionContext<T>,
        generation: usize,
        evaluations: usize,
        population: &[Solution<T>],
    ) {
        let (best, avg, worst) = calculate_statistics(population);
        context.emit(AlgorithmEvent::GenerationCompleted {
            generation,
            evaluations,
            best_fitness: best,
            average_fitness: avg,
            worst_fitness: worst,
        });
    }

    fn sort_population_desc(population: &mut [Solution<T>]) {
        population.sort_by(|a, b| {
            b.value()
                .partial_cmp(&a.value())
                .unwrap_or(std::cmp::Ordering::Equal)
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

    fn run(&mut self, problem: &(impl Problem<T> + Sync)) -> Self::SolutionSet {
        let is_valid = self.validate_parameters();
        let parameters = &self.parameters;
        let observers = std::mem::take(&mut self.observers);

        let (result, observers) = run_with_observers_in_worker(observers, move |context| {
            if !is_valid {
                let error_msg = "Invalid parameters: population_size and max_generations must be > 0, probabilities must be in [0, 1]".to_string();
                context.emit(AlgorithmEvent::Error {
                    message: error_msg.clone(),
                });
                panic!("{}", error_msg);
            }

            Self::run_internal(parameters, problem, &context)
        });

        self.observers = observers;
        self.solution_set = Some(result.clone());
        result
    }

    fn validate_parameters(&self) -> bool {
        self.parameters.population_size > 0
            && self.parameters.max_generations > 0
            && self.parameters.crossover_probability >= 0.0
            && self.parameters.crossover_probability <= 1.0
            && self.parameters.mutation_probability >= 0.0
            && self.parameters.mutation_probability <= 1.0
                && self.parameters.elite_size <= self.parameters.population_size
    }

    fn get_solution_set(&self) -> Option<&Self::SolutionSet> {
        self.solution_set.as_ref()
    }
}
