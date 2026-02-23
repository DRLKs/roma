use crate::algorithms::traits::Algorithm;
use crate::observer::AlgorithmEvent;
use crate::observer::traits::{AlgorithmObserver, Observable};
use crate::operator::traits::{CrossoverOperator, MutationOperator, SelectionOperator};
use crate::problem::traits::Problem;
use crate::solution_set::implementations::vector_solution_set::VectorSolutionSet;
use crate::solution_set::traits::SolutionSet;
use crate::solutions::implementations::real_solution::RealSolution;
use crate::solutions::traits::Solution;

/// Parameters for NSGA-II algorithm
pub struct NSGAIIParameters<C, M, Sel>
where
    C: CrossoverOperator<f64, RealSolution>,
    M: MutationOperator<f64, RealSolution>,
    Sel: SelectionOperator<f64, RealSolution>,
{
    pub population_size: usize,
    pub max_generations: usize,
    pub crossover_probability: f64,
    pub mutation_probability: f64,
    pub crossover_operator: C,
    pub mutation_operator: M,
    pub selection_operator: Sel,
}

impl<C, M, Sel> NSGAIIParameters<C, M, Sel>
where
    C: CrossoverOperator<f64, RealSolution>,
    M: MutationOperator<f64, RealSolution>,
    Sel: SelectionOperator<f64, RealSolution>,
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
        NSGAIIParameters {
            population_size,
            max_generations,
            crossover_probability,
            mutation_probability,
            crossover_operator,
            mutation_operator,
            selection_operator,
        }
    }
}

/// NSGA-II: Non-dominated Sorting Genetic Algorithm II
///
/// A multi-objective evolutionary algorithm that uses:
/// - Fast non-dominated sorting
/// - Crowding distance for diversity preservation
/// - Elitist selection
pub struct NSGAII<C, M, Sel>
where
    C: CrossoverOperator<f64, RealSolution>,
    M: MutationOperator<f64, RealSolution>,
    Sel: SelectionOperator<f64, RealSolution>,
{
    parameters: NSGAIIParameters<C, M, Sel>,
    solution_set: Option<VectorSolutionSet<f64, RealSolution>>,
    observers: Vec<Box<dyn AlgorithmObserver<f64, RealSolution>>>,
}

impl<C, M, Sel> NSGAII<C, M, Sel>
where
    C: CrossoverOperator<f64, RealSolution>,
    M: MutationOperator<f64, RealSolution>,
    Sel: SelectionOperator<f64, RealSolution>,
{
    pub fn new(parameters: NSGAIIParameters<C, M, Sel>) -> Self {
        NSGAII {
            parameters,
            solution_set: None,
            observers: Vec::new(),
        }
    }

}

/// Implementation of Observable trait for NSGA-II
impl<C, M, Sel> Observable<f64, RealSolution> for NSGAII<C, M, Sel>
where
    C: CrossoverOperator<f64, RealSolution>,
    M: MutationOperator<f64, RealSolution>,
    Sel: SelectionOperator<f64, RealSolution>,
{
    fn add_observer(&mut self, observer: Box<dyn AlgorithmObserver<f64, RealSolution>>) {
        self.observers.push(observer);
    }

    fn clear_observers(&mut self) {
        self.observers.clear();
    }

    fn notify_observers(&mut self, event: &AlgorithmEvent<f64, RealSolution>) {
        for observer in &mut self.observers {
            observer.update(event);
        }
    }
}

impl<C, M, Sel> NSGAII<C, M, Sel>
where
    C: CrossoverOperator<f64, RealSolution>,
    M: MutationOperator<f64, RealSolution>,
    Sel: SelectionOperator<f64, RealSolution>,
{
    /// Fast non-dominated sorting
    /// Assigns rank to each solution based on dominance
    fn fast_non_dominated_sort(&self, population: &mut [RealSolution]) {
        let n = population.len();
        let mut domination_count = vec![0; n];
        let mut dominated_solutions: Vec<Vec<usize>> = vec![vec![]; n];
        let mut fronts: Vec<Vec<usize>> = vec![];
        let mut current_front = vec![];

        // Find domination relationships
        for i in 0..n {
            for j in 0..n {
                if i == j {
                    continue;
                }

                if population[i].dominates(&population[j]) {
                    dominated_solutions[i].push(j);
                } else if population[j].dominates(&population[i]) {
                    domination_count[i] += 1;
                }
            }

            if domination_count[i] == 0 {
                population[i].set_rank(0);
                current_front.push(i);
            }
        }

        fronts.push(current_front.clone());

        // Build subsequent fronts
        let mut rank = 0;
        while rank < fronts.len() && !fronts[rank].is_empty() {
            let mut next_front = vec![];
            for &i in &fronts[rank] {
                for &j in &dominated_solutions[i] {
                    domination_count[j] -= 1;
                    if domination_count[j] == 0 {
                        population[j].set_rank(rank + 1);
                        next_front.push(j);
                    }
                }
            }
            rank += 1;
            if !next_front.is_empty() {
                fronts.push(next_front);
            }
        }
    }

    /// Calculate crowding distance for solutions in the same front
    fn calculate_crowding_distance(&self, population: &mut [RealSolution], indices: &[usize]) {
        if indices.is_empty() {
            return;
        }

        // Initialize crowding distances to 0
        for &i in indices {
            population[i].set_crowding_distance(0.0);
        }

        // Get number of objectives
        let num_objectives = population[indices[0]]
            .get_objectives()
            .map(|objs| objs.len())
            .unwrap_or(0);

        // For each objective
        for obj_index in 0..num_objectives {
            // Sort by this objective
            let mut sorted_indices: Vec<usize> = indices.to_vec();
            sorted_indices.sort_by(|&a, &b| {
                let obj_a = population[a].get_objective(obj_index).unwrap_or(0.0);
                let obj_b = population[b].get_objective(obj_index).unwrap_or(0.0);
                obj_a.partial_cmp(&obj_b).unwrap()
            });

            // Boundary solutions get infinite distance
            population[sorted_indices[0]].set_crowding_distance(f64::INFINITY);
            population[sorted_indices[sorted_indices.len() - 1]].set_crowding_distance(f64::INFINITY);

            // Get objective range
            let min_obj = population[sorted_indices[0]]
                .get_objective(obj_index)
                .unwrap_or(0.0);
            let max_obj = population[sorted_indices[sorted_indices.len() - 1]]
                .get_objective(obj_index)
                .unwrap_or(1.0);
            let range = max_obj - min_obj;

            if range > 0.0 {
                for i in 1..sorted_indices.len() - 1 {
                    let prev_obj = population[sorted_indices[i - 1]]
                        .get_objective(obj_index)
                        .unwrap_or(0.0);
                    let next_obj = population[sorted_indices[i + 1]]
                        .get_objective(obj_index)
                        .unwrap_or(0.0);

                    let current_distance = population[sorted_indices[i]]
                        .get_crowding_distance()
                        .unwrap_or(0.0);
                    population[sorted_indices[i]]
                        .set_crowding_distance(current_distance + (next_obj - prev_obj) / range);
                }
            }
        }
    }

    /// Environmental selection: select the best population_size solutions
    fn environmental_selection(&self, population: &mut Vec<RealSolution>) {
        if population.len() <= self.parameters.population_size {
            return;
        }

        // Perform non-dominated sorting
        self.fast_non_dominated_sort(population);

        // Collect solutions by fronts
        let mut fronts: Vec<Vec<usize>> = vec![];
        let max_rank = population
            .iter()
            .filter_map(|s| s.get_rank())
            .max()
            .unwrap_or(0);

        for rank in 0..=max_rank {
            let front: Vec<usize> = population
                .iter()
                .enumerate()
                .filter(|(_, s)| s.get_rank() == Some(rank))
                .map(|(i, _)| i)
                .collect();

            if !front.is_empty() {
                fronts.push(front);
            }
        }

        // Select solutions
        let mut new_population = vec![];
        let mut front_index = 0;

        while new_population.len() < self.parameters.population_size && front_index < fronts.len() {
            let front = &fronts[front_index];

            if new_population.len() + front.len() <= self.parameters.population_size {
                // Add entire front
                for &i in front {
                    new_population.push(population[i].clone());
                }
            } else {
                // Calculate crowding distance for this front
                self.calculate_crowding_distance(population, front);

                // Sort by crowding distance (descending)
                let mut sorted_front = front.clone();
                sorted_front.sort_by(|&a, &b| {
                    let dist_a = population[a].get_crowding_distance().unwrap_or(0.0);
                    let dist_b = population[b].get_crowding_distance().unwrap_or(0.0);
                    dist_b.partial_cmp(&dist_a).unwrap()
                });

                // Add solutions with highest crowding distance
                let remaining = self.parameters.population_size - new_population.len();
                for &i in sorted_front.iter().take(remaining) {
                    new_population.push(population[i].clone());
                }
            }

            front_index += 1;
        }

        *population = new_population;
    }
}

impl<P, C, M, Sel> Algorithm<f64, RealSolution, P> for NSGAII<C, M, Sel>
where
    P: Problem<f64, RealSolution>,
    C: CrossoverOperator<f64, RealSolution>,
    M: MutationOperator<f64, RealSolution>,
    Sel: SelectionOperator<f64, RealSolution>,
{
    type SolutionSet = VectorSolutionSet<f64, RealSolution>;
    type Parameters = NSGAIIParameters<C, M, Sel>;

    fn run(&mut self, problem: &P, verbose: u8) -> Self::SolutionSet {
        use crate::utils::random::{Random, seed_from_time};

        if verbose > 0 {
            println!("Starting NSGA-II");
            println!("  Population size: {}", self.parameters.population_size);
            println!("  Max generations: {}", self.parameters.max_generations);
        }

        self.notify_observers(&AlgorithmEvent::Start {
            algorithm_name: "NSGA-II".to_string(),
        });

        // Initialize population
        let mut population: Vec<RealSolution> = (0..self.parameters.population_size)
            .map(|_| {
                let mut solution = problem.create_solution();
                problem.evaluate(&mut solution);
                solution
            })
            .collect();

        let mut rng = Random::new(seed_from_time());
        let mut evaluations = self.parameters.population_size;

        // Evolution loop
        for generation in 0..self.parameters.max_generations {
            // Create offspring population
            let mut offspring = vec![];

            while offspring.len() < self.parameters.population_size {
                let parent1 = self.parameters.selection_operator.execute(&population).copy();
                let parent2 = self.parameters.selection_operator.execute(&population).copy();

                let mut children = if rng.next_f64() < self.parameters.crossover_probability {
                    self.parameters.crossover_operator.execute(&parent1, &parent2)
                } else {
                    vec![parent1, parent2]
                };

                for child in &mut children {
                    self.parameters
                        .mutation_operator
                        .execute(child, self.parameters.mutation_probability);
                    problem.evaluate(child);
                    evaluations += 1;
                }

                offspring.extend(children);
            }

            offspring.truncate(self.parameters.population_size);

            // Combine parent and offspring populations
            population.extend(offspring);

            // Environmental selection
            self.environmental_selection(&mut population);

            // Calculate statistics for first objective (for monitoring)
            let first_obj_values: Vec<f64> = population
                .iter()
                .filter_map(|s| s.get_objective(0))
                .collect();

            if !first_obj_values.is_empty() {
                let best = first_obj_values
                    .iter()
                    .min_by(|a, b| a.partial_cmp(b).unwrap())
                    .copied()
                    .unwrap();
                let worst = first_obj_values
                    .iter()
                    .max_by(|a, b| a.partial_cmp(b).unwrap())
                    .copied()
                    .unwrap();
                let avg = first_obj_values.iter().sum::<f64>() / first_obj_values.len() as f64;

                self.notify_observers(&AlgorithmEvent::GenerationCompleted {
                    generation,
                    evaluations,
                    best_fitness: best,
                    average_fitness: avg,
                    worst_fitness: worst,
                });

                if verbose > 0 && generation % 10 == 0 {
                    println!("Generation {}: Best f1 = {:.6}", generation, best);
                }
            }
        }

        if verbose > 0 {
            println!("NSGA-II finished. Total evaluations: {}", evaluations);
        }

        self.notify_observers(&AlgorithmEvent::End {
            total_generations: self.parameters.max_generations,
            total_evaluations: evaluations,
        });

        for observer in &mut self.observers {
            observer.finalize();
        }

        // Return Pareto front (rank 0 solutions)
        let mut result = VectorSolutionSet::new();
        for solution in population {
            if solution.get_rank() == Some(0) {
                result.add_solution(solution);
            }
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
