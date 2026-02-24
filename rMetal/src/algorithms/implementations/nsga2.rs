use crate::algorithms::traits::Algorithm;
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
        }
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

    fn notify_observers(&mut self, event: &AlgorithmEvent<f64>) {
        for observer in &mut self.observers {
            observer.update(event);
        }
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

    fn run(&mut self, problem: &impl Problem<f64, MultiObjectiveInfo>) -> Self::SolutionSet {
        self.notify_observers(&AlgorithmEvent::Start {
            algorithm_name: "NSGA-II".to_string(),
        });

        let mut population: Vec<_> = (0..self.parameters.population_size)
            .map(|_| {
                let mut solution = problem.create_solution();
                problem.evaluate(&mut solution);
                solution
            })
            .collect();

        let mut rng = Random::new(seed_from_time());
        let mut evaluations = self.parameters.population_size;

        for generation in 1..=self.parameters.max_generations {
            let mut offspring = Vec::with_capacity(self.parameters.population_size);

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
            population.extend(offspring);

            // Minimization environmental selection over first objective.
            population.sort_by(|a, b| {
                a.get_objective(0)
                    .unwrap_or(f64::INFINITY)
                    .partial_cmp(&b.get_objective(0).unwrap_or(f64::INFINITY))
                    .unwrap_or(std::cmp::Ordering::Equal)
            });
            population.truncate(self.parameters.population_size);

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

            self.notify_observers(&AlgorithmEvent::GenerationCompleted {
                generation,
                evaluations,
                best_fitness: best,
                average_fitness: avg,
                worst_fitness: worst,
            });
        }

        self.notify_observers(&AlgorithmEvent::End {
            total_generations: self.parameters.max_generations,
            total_evaluations: evaluations,
        });

        for observer in &mut self.observers {
            observer.finalize();
        }

        let result = VectorSolutionSet::from_vec(population);
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
