use crate::algorithms::termination::{
    ExecutionStateSnapshot,
    ImprovementDirection,
    TerminationCriteria,
};
use crate::algorithms::traits::Algorithm;
use crate::algorithms::runtime::ExecutionContext;
use crate::experiment::traits::{CaseParameter, ExperimentalCase};
use crate::observer::traits::{AlgorithmObserver, Observable};
use crate::operator::traits::{CrossoverOperator, MutationOperator, SelectionOperator};
use crate::problem::traits::Problem;
use crate::solution::Solution;
use crate::solution_set::implementations::vector_solution_set::VectorSolutionSet;
use crate::solution_set::traits::SolutionSet;
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

    pub fn show_parameters_information(&self){
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
        let requested_threads = Self::resolve_num_threads(parameters.num_threads);
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

    fn sort_population_desc(population: &mut [Solution<T>]) {
        population.sort_by(|a, b| {
            b.quality_value()
                .partial_cmp(&a.quality_value())
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

    fn improvement_direction(&self) -> ImprovementDirection {
        ImprovementDirection::Maximize
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
        _context: &ExecutionContext<T>,
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

    fn snapshot(&self, state: &Self::StepState) -> ExecutionStateSnapshot<T> {
        let (_best, avg, worst) = calculate_statistics(&state.population);
        let best_solution = state
            .population
            .iter()
            .max_by(|a, b| {
                a.quality_value()
                    .partial_cmp(&b.quality_value())
                    .unwrap_or(std::cmp::Ordering::Equal)
            })
            .map(|solution| solution.copy())
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

/// Executable experiment case for the Genetic Algorithm.
#[derive(Clone)]
pub struct GeneticAlgorithmExperiment<T, C, M, Sel>
where
    T: Clone,
    C: CrossoverOperator<T> + Clone,
    M: MutationOperator<T> + Clone,
    Sel: SelectionOperator<T> + Clone,
{
    pub parameters: GeneticAlgorithmParameters<T, C, M, Sel>,
}

impl<T, C, M, Sel> GeneticAlgorithmExperiment<T, C, M, Sel>
where
    T: Clone,
    C: CrossoverOperator<T> + Clone,
    M: MutationOperator<T> + Clone,
    Sel: SelectionOperator<T> + Clone,
{
    pub fn new(parameters: GeneticAlgorithmParameters<T, C, M, Sel>) -> Self {
        Self { parameters }
    }
}

impl<T, C, M, Sel, P> ExperimentalCase<T, f64, P> for GeneticAlgorithmExperiment<T, C, M, Sel>
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
            self.parameters.population_size,
            self.parameters.crossover_probability,
            self.parameters.mutation_probability,
            self.parameters.elite_size,
        )
    }

    fn parameters(&self) -> Vec<CaseParameter> {
        let threads_text = match self.parameters.num_threads {
            Some(v) => v.to_string(),
            None => "auto".to_string(),
        };

        vec![
            CaseParameter::new("population_size", self.parameters.population_size.to_string()),
            CaseParameter::new(
                "crossover_probability",
                format!("{:.6}", self.parameters.crossover_probability),
            ),
            CaseParameter::new(
                "mutation_probability",
                format!("{:.6}", self.parameters.mutation_probability),
            ),
            CaseParameter::new("elite_size", self.parameters.elite_size.to_string()),
            CaseParameter::new("threads", threads_text),
            CaseParameter::new("crossover_operator", self.parameters.crossover_operator.name()),
            CaseParameter::new("mutation_operator", self.parameters.mutation_operator.name()),
            CaseParameter::new("selection_operator", self.parameters.selection_operator.name()),
            CaseParameter::new(
                "termination_criteria",
                format!("{:?}", self.parameters.termination_criteria),
            ),
        ]
    }

    fn run(&self, problem: &P) -> Result<Box<dyn SolutionSet<T, f64>>, String> {
        let mut algorithm = GeneticAlgorithm::new(self.parameters.clone());
        let result = algorithm.run(problem)?;
        Ok(Box::new(result))
    }
}
