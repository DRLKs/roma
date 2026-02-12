use crate::algorithms::traits::Algorithm;
use crate::operator::traits::{CrossoverOperator, MutationOperator, SelectionOperator};
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
            _phantom: std::marker::PhantomData,
        }
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
        }
    }
}

impl<T, S, P, C, M, Sel> Algorithm<T, S, P> for GeneticAlgorithm<T, S, C, M, Sel>
where
    S: Solution<T> + Clone,
    T: Clone,
    P: Problem<S, T>,
    C: CrossoverOperator<T, S>,
    M: MutationOperator<T, S>,
    Sel: SelectionOperator<T, S>,
{
    type SolutionSet = VectorSolutionSet<T, S>;
    type Parameters = GeneticAlgorithmParameters<T, S, C, M, Sel>;

    fn run(&mut self, problem: &P, verbose: u8) -> Self::SolutionSet {
        if verbose > 0 {
            println!("Starting Genetic Algorithm");
            println!("  Population size: {}", self.parameters.population_size);
            println!("  Max generations: {}", self.parameters.max_generations);
            println!("  Crossover: {}", self.parameters.crossover_operator.name());
            println!("  Mutation: {}", self.parameters.mutation_operator.name());
            println!("  Selection: {}", self.parameters.selection_operator.name());
        }

        // Initialize population
        let mut population: Vec<S> = (0..self.parameters.population_size)
            .map(|_| {
                let mut solution = problem.create_solution();
                problem.evaluate(&mut solution);
                solution
            })
            .collect();

        let mut best_fitness = population
            .iter()
            .map(|s| s.value())
            .max_by(|a, b| a.partial_cmp(b).unwrap())
            .unwrap_or(0.0);

        if verbose > 0 {
            println!("Initial best fitness: {}", best_fitness);
        }

        // Evolution loop
        for generation in 0..self.parameters.max_generations {
            let mut offspring_population = Vec::new();

            // Generate offspring
            while offspring_population.len() < self.parameters.population_size {
                // Selection
                let parent1 = self.parameters.selection_operator.execute(&population).copy();
                let parent2 = self.parameters.selection_operator.execute(&population).copy();

                // Crossover
                let mut offspring = if crate::utils::random::Random::new(
                    crate::utils::random::seed_from_time(),
                )
                .next_f64()
                    < self.parameters.crossover_probability
                {
                    self.parameters
                        .crossover_operator
                        .execute(&parent1, &parent2)
                } else {
                    vec![parent1, parent2]
                };

                // Mutation
                for child in &mut offspring {
                    self.parameters
                        .mutation_operator
                        .execute(child, self.parameters.mutation_probability);
                    problem.evaluate(child);
                }

                offspring_population.extend(offspring);
            }

            // Truncate to population size if necessary
            offspring_population.truncate(self.parameters.population_size);

            population = offspring_population;

            // Track best fitness
            let current_best = population
                .iter()
                .map(|s| s.value())
                .max_by(|a, b| a.partial_cmp(b).unwrap())
                .unwrap_or(0.0);

            if current_best > best_fitness {
                best_fitness = current_best;
                if verbose > 1 {
                    println!("Generation {}: New best fitness = {}", generation, best_fitness);
                }
            }
        }

        if verbose > 0 {
            println!("Genetic Algorithm finished. Best fitness: {}", best_fitness);
        }

        // Find and return the best solution
        let best_solution = population
            .into_iter()
            .max_by(|a, b| a.value().partial_cmp(&b.value()).unwrap())
            .unwrap();

        let mut result = VectorSolutionSet::new();
        result.add_solution(best_solution);

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
