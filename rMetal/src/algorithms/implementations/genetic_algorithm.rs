use crate::algorithms::traits::Algorithm;
use crate::observer::AlgorithmEvent;
use crate::operator::traits::{CrossoverOperator, MutationOperator, SelectionOperator};
use crate::observer::traits::{AlgorithmObserver, ThreadSafeObserverCollection};
use crate::problem::traits::Problem;
use crate::solution_set::implementations::vector_solution_set::VectorSolutionSet;
use crate::solution_set::traits::SolutionSet;
use crate::solutions::traits::Solution;

/// Parameters for the Genetic Algorithm.
/// Uses generics to allow any combination of operators.
pub struct GeneticAlgorithmParameters<T, S, C, M, Sel>
where
    S: Solution<T>,
    T: Clone,
    C: CrossoverOperator<T, S>,
    M: MutationOperator<T, S>,
    Sel: SelectionOperator<T, S>,
{
    pub population_size: usize,
    pub max_generations: usize,
    pub crossover_probability: f64,
    pub mutation_probability: f64,
    pub crossover_operator: C,
    pub mutation_operator: M,
    pub selection_operator: Sel,
    pub num_threads: Option<usize>,
    _phantom: std::marker::PhantomData<(T, S)>,
}

impl<T, S, C, M, Sel> GeneticAlgorithmParameters<T, S, C, M, Sel>
where
    S: Solution<T>,
    T: Clone,
    C: CrossoverOperator<T, S>,
    M: MutationOperator<T, S>,
    Sel: SelectionOperator<T, S>,
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
            crossover_operator,
            mutation_operator,
            selection_operator,
            num_threads: None,
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
    }

    pub fn with_threads(mut self, num_threads: usize) -> Self {
        self.num_threads = Some(num_threads);
        self
    }

    pub fn with_parallel(mut self) -> Self {
        self.num_threads = None;
        self
    }

    pub fn sequential(mut self) -> Self {
        self.num_threads = Some(1);
        self
    }
}

/// Genetic Algorithm implementation with configurable operators.
/// 
/// This design allows you to plug in different operators without changing the algorithm code.
/// 
/// # Type Parameters
/// * `T` - Type of solution variables
/// * `S` - Solution type
/// * `C` - Crossover operator type
/// * `M` - Mutation operator type
/// * `Sel` - Selection operator type
pub struct GeneticAlgorithm<T, S, C, M, Sel>
where
    S: Solution<T> + Clone,
    T: Clone,
    C: CrossoverOperator<T, S>,
    M: MutationOperator<T, S>,
    Sel: SelectionOperator<T, S>,
{
    parameters: GeneticAlgorithmParameters<T, S, C, M, Sel>,
    solution_set: Option<VectorSolutionSet<T, S>>,
    observers: Vec<Box<dyn AlgorithmObserver<T, S>>>,
}

impl<T, S, C, M, Sel> GeneticAlgorithm<T, S, C, M, Sel>
where
    S: Solution<T> + Clone,
    T: Clone,
    C: CrossoverOperator<T, S>,
    M: MutationOperator<T, S>,
    Sel: SelectionOperator<T, S>,
{
    pub fn new(parameters: GeneticAlgorithmParameters<T, S, C, M, Sel>) -> Self {
        GeneticAlgorithm {
            parameters,
            solution_set: None,
            observers: Vec::new(),
        }
    }

    /// Adds an observer to monitor algorithm execution
    pub fn add_observer(&mut self, observer: Box<dyn AlgorithmObserver<T, S>>) {
        self.observers.push(observer);
    }

    /// Notifies all observers of an event
    fn notify_observers(&mut self, event: &AlgorithmEvent<T, S>) {
        for observer in &mut self.observers {
            observer.update(event);
        }
    }
}

impl<T, S, P, C, M, Sel> Algorithm<T, S, P> for GeneticAlgorithm<T, S, C, M, Sel>
where
    S: Solution<T> + Clone + Send,
    T: Clone + Send,
    P: Problem<T, S> + Sync,
    C: CrossoverOperator<T, S> + Sync + Clone + Send,
    M: MutationOperator<T, S> + Sync + Clone + Send,
    Sel: SelectionOperator<T, S> + Sync + Clone + Send,
{
    type SolutionSet = VectorSolutionSet<T, S>;
    type Parameters = GeneticAlgorithmParameters<T, S, C, M, Sel>;

    fn run(&mut self, problem: &P, verbose: u8) -> Self::SolutionSet {

        use crate::utils::random::{Random, seed_from_time};
        use crate::utils::statistics::calculate_statistics;

        // Validate parameters before starting
        if !<Self as Algorithm<T, S, P>>::validate_parameters(self) {
            let error_msg = "Invalid parameters: population_size and max_generations must be > 0, probabilities must be in [0, 1]".to_string();
            
            self.notify_observers(&AlgorithmEvent::Error {
                message: error_msg.clone(),
            });
            
            if verbose > 0 {
                eprintln!("Error: {}", error_msg);
            }
            
            panic!("{}", error_msg);
        }
        
        // Notify start
        self.notify_observers(&AlgorithmEvent::Start {
            algorithm_name: "GeneticAlgorithm".to_string(),
        });

        if verbose > 0 {
            self.parameters.show_parameters_information();
        }

        // Determine number of threads to use
        let num_threads = self.parameters.num_threads.unwrap_or_else(|| {
            std::thread::available_parallelism()
                .map(|n| n.get())
                .unwrap_or(1)
        });

        if verbose > 0 && num_threads > 1 {
            println!("Running {} parallel threads...", num_threads);
        }

        // Pre-allocate slots for solutions from each thread
        use std::sync::{Arc, Mutex};
        let solution_slots: Arc<Mutex<Vec<Option<S>>>> = Arc::new(Mutex::new(vec![None; num_threads]));
        
        // Create thread-safe observer collection
        let observers = ThreadSafeObserverCollection::new(std::mem::take(&mut self.observers));

        // Get references to parameters before scope
        let population_size = self.parameters.population_size;
        let max_generations = self.parameters.max_generations;
        let crossover_prob = self.parameters.crossover_probability;
        let mutation_prob = self.parameters.mutation_probability;
        let crossover_op = &self.parameters.crossover_operator;
        let mutation_op = &self.parameters.mutation_operator;
        let selection_op = &self.parameters.selection_operator;

        // Always use scoped threads, even for 1 thread (unified code path)
        std::thread::scope(|scope| {
            let mut handles = Vec::new();

            for thread_id in 0..num_threads {
                let solution_slots = Arc::clone(&solution_slots);
                let thread_observers = observers.clone_handle();

                let handle = scope.spawn(move || {
                    // Each thread has its own population
                    let mut population: Vec<S> = (0..population_size)
                        .map(|_| {
                            let mut solution = problem.create_solution();
                            problem.evaluate(&mut solution);
                            solution
                        })
                        .collect();

                    // Notify initial statistics
                    let (best_fit, avg_fit, worst_fit) = calculate_statistics(&population);
                    thread_observers.notify(&AlgorithmEvent::GenerationCompleted {
                        generation: 0,
                        evaluations: population_size,
                        best_fitness: best_fit,
                        average_fitness: avg_fit,
                        worst_fitness: worst_fit,
                    });

                    // Evolution loop for this thread
                    for generation in 1..=max_generations {
                        let mut offspring_population = Vec::new();

                        // Generate offspring
                        while offspring_population.len() < population_size {
                            let parent1 = selection_op.execute(&population).copy();
                            let parent2 = selection_op.execute(&population).copy();

                            let mut offspring = if Random::new(seed_from_time())
                            .next_f64()
                                < crossover_prob
                            {
                                crossover_op.execute(&parent1, &parent2)
                            } else {
                                vec![parent1, parent2]
                            };

                            for child in &mut offspring {
                                mutation_op.execute(child, mutation_prob);
                                problem.evaluate(child);
                            }

                            offspring_population.extend(offspring);
                        }

                        offspring_population.truncate(population_size);
                        population = offspring_population;
                        
                        // Calculate and notify statistics after each generation
                        let (best_fitness, avg_fitness, worst_fitness) = calculate_statistics(&population);
                        
                        thread_observers.notify(&AlgorithmEvent::GenerationCompleted {
                            generation,
                            evaluations: population_size * generation,
                            best_fitness,
                            average_fitness: avg_fitness,
                            worst_fitness,
                        });
                    }

                    // Find best solution from this thread
                    let best_solution = population
                        .into_iter()
                        .max_by(|a, b| a.value().partial_cmp(&b.value()).unwrap())
                        .unwrap();

                    // Write to designated slot
                    solution_slots.lock().unwrap()[thread_id] = Some(best_solution);
                });

                handles.push(handle);
            }

            // Wait for all threads to complete
            for handle in handles {
                handle.join().unwrap();
            }
        });

        // Collect all solutions from slots into SolutionSet
        let solutions = solution_slots.lock().unwrap();
        let mut result = VectorSolutionSet::new();
        
        for solution_opt in solutions.iter() {
            if let Some(solution) = solution_opt {
                result.add_solution(solution.clone());
            }
        }

        if verbose > 0 {
            if num_threads > 1 {
                println!("Parallel execution finished. Collected {} solutions.", result.size());
            }
            if let Some(best) = result.solutions().iter().max_by(|a, b| a.value().partial_cmp(&b.value()).unwrap()) {
                println!("Genetic Algorithm finished. Best fitness: {}", best.value());
            }
        }

        // Notify end and finalize
        observers.notify(&AlgorithmEvent::End {
            total_generations: self.parameters.max_generations,
            total_evaluations: self.parameters.population_size * num_threads * self.parameters.max_generations,
        });
        observers.finalize();

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
    }

    fn get_solution_set(&self) -> Option<&Self::SolutionSet> {
        self.solution_set.as_ref()
    }

    fn get_parameters(&self) -> &Self::Parameters {
        &self.parameters
    }

    fn set_parameters(&mut self, params: Self::Parameters) {
        self.parameters = params;
    }
}

impl<T, S, C, M, Sel> Clone for GeneticAlgorithm<T, S, C, M, Sel>
where
    S: Solution<T> + Clone,
    T: Clone,
    C: CrossoverOperator<T, S> + Clone,
    M: MutationOperator<T, S> + Clone,
    Sel: SelectionOperator<T, S> + Clone,
{
    fn clone(&self) -> Self {
        GeneticAlgorithm {
            parameters: GeneticAlgorithmParameters {
                population_size: self.parameters.population_size,
                max_generations: self.parameters.max_generations,
                crossover_probability: self.parameters.crossover_probability,
                mutation_probability: self.parameters.mutation_probability,
                crossover_operator: self.parameters.crossover_operator.clone(),
                mutation_operator: self.parameters.mutation_operator.clone(),
                selection_operator: self.parameters.selection_operator.clone(),
                num_threads: self.parameters.num_threads,
                _phantom: std::marker::PhantomData,
            },
            solution_set: self.solution_set.clone(),
            observers: Vec::new(),
        }
    }
}

unsafe impl<T, S, C, M, Sel> Send for GeneticAlgorithm<T, S, C, M, Sel>
where
    S: Solution<T> + Clone + Send,
    T: Clone + Send,
    C: CrossoverOperator<T, S> + Send,
    M: MutationOperator<T, S> + Send,
    Sel: SelectionOperator<T, S> + Send,
{
}
