use crate::algorithms::termination::{
    ExecutionStateSnapshot,
    ImprovementDirection,
    TerminationCriteria,
};
use crate::algorithms::traits::Algorithm;
use crate::algorithms::runtime::run_with_observers;
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

impl<C, M, Sel> NSGAII<C, M, Sel>
where
    C: CrossoverOperator<f64, ParetoCrowdingDistanceQuality>,
    M: MutationOperator<f64, ParetoCrowdingDistanceQuality>,
    Sel: SelectionOperator<f64, ParetoCrowdingDistanceQuality>,
{
    pub fn new(parameters: NSGAIIParameters<C, M, Sel>) -> Self {
        Self {
            parameters,
            solution_set: None,
            observers: Vec::new(),
        }
    }
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

    fn run(&mut self, problem: &(impl Problem<f64, ParetoCrowdingDistanceQuality> + Sync)) -> Self::SolutionSet {
        let is_valid = self.validate_parameters();
        let parameters = &self.parameters;
        let observers = std::mem::take(&mut self.observers);

        let (result, observers) = run_with_observers(
            observers,
            parameters.termination_criteria.clone(),
            ImprovementDirection::Minimize,
            move |context| {
            if !is_valid {
                let message = "Invalid parameters: population_size must be > 0, termination_criteria must not be empty, probabilities must be in [0,1]".to_string();
                context.error(message.clone());
                panic!("{}", message);
            }

            context.start("NSGA-II");

            let mut rng = Random::new(parameters.random_seed.unwrap_or_else(seed_from_time));

            let mut population: Vec<_> = (0..parameters.population_size)
                .map(|_| {
                    let mut solution = problem.create_solution(&mut rng);
                    problem.evaluate(&mut solution);
                    solution
                })
                .collect();

            let mut evaluations = parameters.population_size;

            // For multi-objective tracking, use objective(0) statistics as the scalar proxy.
            let initial_best_solution = population
                .first()
                .map(|solution| solution.copy())
                .expect("population should not be empty when reporting progress");

            let initial_avg = if population.is_empty() {
                0.0
            } else {
                let values: Vec<f64> = population.iter().filter_map(|s| s.get_objective(0)).collect();
                values.iter().sum::<f64>() / values.len() as f64
            };
            let initial_best = initial_best_solution.get_objective(0).unwrap_or(0.0);
            let initial_worst = population.last().and_then(|s| s.get_objective(0)).unwrap_or(0.0);

            context.report_progress(ExecutionStateSnapshot::new(
                0,
                0,
                evaluations,
                initial_best_solution,
                initial_best,
                initial_avg,
                initial_worst,
            ));
            let mut should_terminate = context.should_terminate();

            let mut generation = 0;
            while !should_terminate {
                generation += 1;
                let mut offspring = Vec::with_capacity(parameters.population_size);

                while offspring.len() < parameters.population_size {
                    let parent1 = parameters.selection_operator.execute(&population, &mut rng).copy();
                    let parent2 = parameters.selection_operator.execute(&population, &mut rng).copy();

                    let mut children = if rng.next_f64() < parameters.crossover_probability {
                        parameters
                            .crossover_operator
                            .execute(&parent1, &parent2, &mut rng)
                    } else {
                        vec![parent1, parent2]
                    };

                    for child in &mut children {
                        parameters
                            .mutation_operator
                            .execute(child, parameters.mutation_probability, &mut rng);
                        problem.evaluate(child);
                        evaluations += 1;
                    }

                    offspring.extend(children);
                }

                offspring.truncate(parameters.population_size);
                population.extend(offspring);

                // Minimization environmental selection over first objective.
                population.sort_by(|a, b| {
                    a.get_objective(0)
                        .unwrap_or(f64::INFINITY)
                        .partial_cmp(&b.get_objective(0).unwrap_or(f64::INFINITY))
                        .unwrap_or(std::cmp::Ordering::Equal)
                });
                population.truncate(parameters.population_size);

                let worst = population
                    .last()
                    .and_then(|s| s.get_objective(0))
                    .unwrap_or(0.0);
                let avg = if population.is_empty() {
                    0.0
                } else {
                    let values: Vec<f64> = population
                        .iter()
                        .filter_map(|s| s.get_objective(0))
                        .collect();
                    if values.is_empty() {
                        0.0
                    } else {
                        values.iter().sum::<f64>() / values.len() as f64
                    }
                };

                let best_solution = population
                    .first()
                    .map(|solution| solution.copy())
                    .expect("population should not be empty when reporting progress");
                let best = best_solution.get_objective(0).unwrap_or(0.0);

                context.report_progress(ExecutionStateSnapshot::new(
                    0,
                    generation,
                    evaluations,
                    best_solution,
                    best,
                    avg,
                    worst,
                ));
                should_terminate = context.should_terminate();
            }

            context.end(generation, evaluations);

            VectorSolutionSet::from_vec(population)
        });

        self.observers = observers;
        self.solution_set = Some(result.clone());
        result
    }

    fn validate_parameters(&self) -> bool {
        self.parameters.population_size > 0
            && !self.parameters.termination_criteria.is_empty()
            && self.parameters.crossover_probability >= 0.0
            && self.parameters.crossover_probability <= 1.0
            && self.parameters.mutation_probability >= 0.0
            && self.parameters.mutation_probability <= 1.0
    }

    fn get_solution_set(&self) -> Option<&Self::SolutionSet> {
        self.solution_set.as_ref()
    }
}
