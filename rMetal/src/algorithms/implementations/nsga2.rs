use crate::algorithms::traits::Algorithm;
use crate::observer::runtime::run_with_observers_in_worker;
use crate::observer::traits::{AlgorithmObserver, Observable};
use crate::observer::AlgorithmEvent;
use crate::operator::traits::{CrossoverOperator, MutationOperator, SelectionOperator};
use crate::problem::traits::Problem;
use crate::solution::MultiObjectiveInfo;
use crate::solution_set::implementations::vector_solution_set::VectorSolutionSet;
use crate::utils::random::{seed_from_time, Random};

pub struct NSGAIIParameters<C, M, Sel>
where
    C: CrossoverOperator<f64, MultiObjectiveInfo>,
    M: MutationOperator<f64, MultiObjectiveInfo>,
    Sel: SelectionOperator<f64, MultiObjectiveInfo>,
{
    pub population_size: usize,
    pub max_generations: usize,
    pub crossover_probability: f64,
    pub mutation_probability: f64,
    pub crossover_operator: C,
    pub mutation_operator: M,
    pub selection_operator: Sel,
    pub random_seed: Option<u64>,
}

impl<C, M, Sel> NSGAIIParameters<C, M, Sel>
where
    C: CrossoverOperator<f64, MultiObjectiveInfo>,
    M: MutationOperator<f64, MultiObjectiveInfo>,
    Sel: SelectionOperator<f64, MultiObjectiveInfo>,
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
        Self {
            population_size,
            max_generations,
            crossover_probability,
            mutation_probability,
            crossover_operator,
            mutation_operator,
            selection_operator,
            random_seed: None,
        }
    }

    pub fn with_seed(mut self, seed: u64) -> Self {
        self.random_seed = Some(seed);
        self
    }
}

pub struct NSGAII<C, M, Sel>
where
    C: CrossoverOperator<f64, MultiObjectiveInfo>,
    M: MutationOperator<f64, MultiObjectiveInfo>,
    Sel: SelectionOperator<f64, MultiObjectiveInfo>,
{
    parameters: NSGAIIParameters<C, M, Sel>,
    solution_set: Option<VectorSolutionSet<f64, MultiObjectiveInfo>>,
    observers: Vec<Box<dyn AlgorithmObserver<f64>>>,
}

impl<C, M, Sel> NSGAII<C, M, Sel>
where
    C: CrossoverOperator<f64, MultiObjectiveInfo>,
    M: MutationOperator<f64, MultiObjectiveInfo>,
    Sel: SelectionOperator<f64, MultiObjectiveInfo>,
{
    pub fn new(parameters: NSGAIIParameters<C, M, Sel>) -> Self {
        Self {
            parameters,
            solution_set: None,
            observers: Vec::new(),
        }
    }
}

impl<C, M, Sel> Observable<f64> for NSGAII<C, M, Sel>
where
    C: CrossoverOperator<f64, MultiObjectiveInfo>,
    M: MutationOperator<f64, MultiObjectiveInfo>,
    Sel: SelectionOperator<f64, MultiObjectiveInfo>,
{
    fn add_observer(&mut self, observer: Box<dyn AlgorithmObserver<f64>>) {
        self.observers.push(observer);
    }

    fn clear_observers(&mut self) {
        self.observers.clear();
    }
}

impl<C, M, Sel> Algorithm<f64, MultiObjectiveInfo> for NSGAII<C, M, Sel>
where
    C: CrossoverOperator<f64, MultiObjectiveInfo>,
    M: MutationOperator<f64, MultiObjectiveInfo>,
    Sel: SelectionOperator<f64, MultiObjectiveInfo>,
{
    type SolutionSet = VectorSolutionSet<f64, MultiObjectiveInfo>;
    type Parameters = NSGAIIParameters<C, M, Sel>;

    fn run(&mut self, problem: &(impl Problem<f64, MultiObjectiveInfo> + Sync)) -> Self::SolutionSet {
        let is_valid = self.validate_parameters();
        let parameters = &self.parameters;
        let observers = std::mem::take(&mut self.observers);

        let (result, observers) = run_with_observers_in_worker(observers, move |context| {
            if !is_valid {
                let message = "Invalid parameters: population_size and max_generations must be > 0, probabilities must be in [0,1]".to_string();
                context.emit(AlgorithmEvent::Error {
                    message: message.clone(),
                });
                panic!("{}", message);
            }

            context.emit(AlgorithmEvent::Start {
                algorithm_name: "NSGA-II".to_string(),
            });

            let mut rng = Random::new(parameters.random_seed.unwrap_or_else(seed_from_time));

            let mut population: Vec<_> = (0..parameters.population_size)
                .map(|_| {
                    let mut solution = problem.create_solution(&mut rng);
                    problem.evaluate(&mut solution);
                    solution
                })
                .collect();

            let mut evaluations = parameters.population_size;

            for generation in 1..=parameters.max_generations {
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

                let best = population
                    .first()
                    .and_then(|s| s.get_objective(0))
                    .unwrap_or(0.0);
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

                context.emit(AlgorithmEvent::GenerationCompleted {
                    generation,
                    evaluations,
                    best_fitness: best,
                    average_fitness: avg,
                    worst_fitness: worst,
                });
            }

            context.emit(AlgorithmEvent::End {
                total_generations: parameters.max_generations,
                total_evaluations: evaluations,
            });

            VectorSolutionSet::from_vec(population)
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
    }

    fn get_solution_set(&self) -> Option<&Self::SolutionSet> {
        self.solution_set.as_ref()
    }
}
