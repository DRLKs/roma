use crate::algorithms::traits::Algorithm;
use crate::observer::traits::{AlgorithmObserver, Observable};
use crate::observer::AlgorithmEvent;
use crate::operator::traits::{CrossoverOperator, MutationOperator, SelectionOperator};
use crate::problem::traits::Problem;
use crate::solution_set::implementations::vector_solution_set::VectorSolutionSet;
use crate::solution_set::traits::SolutionSet;
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
    pub crossover_operator: C,
    pub mutation_operator: M,
    pub selection_operator: Sel,
    pub num_threads: Option<usize>,
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
    T: Clone,
    C: CrossoverOperator<T>,
    M: MutationOperator<T>,
    Sel: SelectionOperator<T>,
{
    pub fn new(parameters: GeneticAlgorithmParameters<T, C, M, Sel>) -> Self {
        GeneticAlgorithm {
            parameters,
            solution_set: None,
            observers: Vec::new(),
        }
    }
}

impl<T, C, M, Sel> Observable<T> for GeneticAlgorithm<T, C, M, Sel>
where
    T: Clone,
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

    fn notify_observers(&mut self, event: &AlgorithmEvent<T>) {
        for observer in &mut self.observers {
            observer.update(event);
        }
    }
}

impl<T, C, M, Sel> Algorithm<T> for GeneticAlgorithm<T, C, M, Sel>
where
    T: Clone,
    C: CrossoverOperator<T>,
    M: MutationOperator<T>,
    Sel: SelectionOperator<T>,
{
    type SolutionSet = VectorSolutionSet<T>;
    type Parameters = GeneticAlgorithmParameters<T, C, M, Sel>;

    fn run(&mut self, problem: &impl Problem<T>) -> Self::SolutionSet {
        if !self.validate_parameters() {
            let error_msg = "Invalid parameters: population_size and max_generations must be > 0, probabilities must be in [0, 1]".to_string();
            self.notify_observers(&AlgorithmEvent::Error {
                message: error_msg.clone(),
            });
            panic!("{}", error_msg);
        }

        self.notify_observers(&AlgorithmEvent::Start {
            algorithm_name: "GeneticAlgorithm".to_string(),
        });

        let mut population: Vec<_> = (0..self.parameters.population_size)
            .map(|_| {
                let mut solution = problem.create_solution();
                problem.evaluate(&mut solution);
                solution
            })
            .collect();

        let (best, avg, worst) = calculate_statistics(&population);
        self.notify_observers(&AlgorithmEvent::GenerationCompleted {
            generation: 0,
            evaluations: self.parameters.population_size,
            best_fitness: best,
            average_fitness: avg,
            worst_fitness: worst,
        });

        let mut rng = Random::new(seed_from_time());
        let mut evaluations = self.parameters.population_size;

        for generation in 1..=self.parameters.max_generations {
            let mut offspring_population = Vec::with_capacity(self.parameters.population_size);
            while offspring_population.len() < self.parameters.population_size {
                let parent1 = self.parameters.selection_operator.execute(&population).copy();
                let parent2 = self.parameters.selection_operator.execute(&population).copy();

                let mut offspring = if rng.next_f64() < self.parameters.crossover_probability {
                    self.parameters.crossover_operator.execute(&parent1, &parent2)
                } else {
                    vec![parent1, parent2]
                };

                for child in &mut offspring {
                    self.parameters
                        .mutation_operator
                        .execute(child, self.parameters.mutation_probability);
                    problem.evaluate(child);
                    evaluations += 1;
                }

                offspring_population.extend(offspring);
            }

            offspring_population.truncate(self.parameters.population_size);
            population = offspring_population;

            let (best, avg, worst) = calculate_statistics(&population);
            self.notify_observers(&AlgorithmEvent::GenerationCompleted {
                generation,
                evaluations,
                best_fitness: best,
                average_fitness: avg,
                worst_fitness: worst,
            });
        }

        let mut result = VectorSolutionSet::from_vec(population);
        result.solutions_mut().sort_by(|a, b| {
            b.value()
                .partial_cmp(&a.value())
                .unwrap_or(std::cmp::Ordering::Equal)
        });

        self.notify_observers(&AlgorithmEvent::End {
            total_generations: self.parameters.max_generations,
            total_evaluations: evaluations,
        });

        for observer in &mut self.observers {
            observer.finalize();
        }

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
