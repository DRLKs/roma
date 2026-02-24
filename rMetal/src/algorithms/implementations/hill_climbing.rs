use crate::algorithms::traits::Algorithm;
use crate::observer::traits::{AlgorithmObserver, Observable};
use crate::observer::AlgorithmEvent;
use crate::operator::traits::MutationOperator;
use crate::problem::traits::Problem;
use crate::solution_set::implementations::vector_solution_set::VectorSolutionSet;
use crate::solution_set::traits::SolutionSet;

pub struct HillClimbingParameters<T, M>
where
    T: Clone,
    M: MutationOperator<T>,
{
    pub max_iterations: usize,
    pub mutation_operator: M,
    pub mutation_probability: f64,
    _phantom: std::marker::PhantomData<T>,
}

impl<T, M> HillClimbingParameters<T, M>
where
    T: Clone,
    M: MutationOperator<T>,
{
    pub fn new(max_iterations: usize, mutation_operator: M, mutation_probability: f64) -> Self {
        Self {
            max_iterations,
            mutation_operator,
            mutation_probability,
            _phantom: std::marker::PhantomData,
        }
    }
}

pub struct HillClimbing<T, M>
where
    T: Clone,
    M: MutationOperator<T>,
{
    parameters: HillClimbingParameters<T, M>,
    solution_set: Option<VectorSolutionSet<T>>,
    is_maximization: bool,
    observers: Vec<Box<dyn AlgorithmObserver<T>>>,
}

impl<T, M> HillClimbing<T, M>
where
    T: Clone,
    M: MutationOperator<T>,
{
    pub fn new(parameters: HillClimbingParameters<T, M>, maximization: bool) -> Self {
        Self {
            parameters,
            solution_set: None,
            is_maximization: maximization,
            observers: Vec::new(),
        }
    }
}

impl<T, M> Observable<T> for HillClimbing<T, M>
where
    T: Clone,
    M: MutationOperator<T>,
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

impl<T, M> Algorithm<T> for HillClimbing<T, M>
where
    T: Clone,
    M: MutationOperator<T>,
{
    type SolutionSet = VectorSolutionSet<T>;
    type Parameters = HillClimbingParameters<T, M>;

    fn run(&mut self, problem: &impl Problem<T>) -> Self::SolutionSet {
        if !self.validate_parameters() {
            let message = "Invalid parameters: max_iterations must be > 0, mutation_probability must be in [0,1]".to_string();
            self.notify_observers(&AlgorithmEvent::Error {
                message: message.clone(),
            });
            panic!("{}", message);
        }

        self.notify_observers(&AlgorithmEvent::Start {
            algorithm_name: "HillClimbing".to_string(),
        });

        let mut current = problem.create_solution();
        problem.evaluate(&mut current);
        let mut evaluations = 1;

        let initial = current.value();
        self.notify_observers(&AlgorithmEvent::GenerationCompleted {
            generation: 0,
            evaluations,
            best_fitness: initial,
            average_fitness: initial,
            worst_fitness: initial,
        });

        for iteration in 1..=self.parameters.max_iterations {
            let mut neighbor = current.copy();
            self.parameters
                .mutation_operator
                .execute(&mut neighbor, self.parameters.mutation_probability);
            problem.evaluate(&mut neighbor);
            evaluations += 1;

            let improved = if self.is_maximization {
                neighbor.value() > current.value()
            } else {
                neighbor.value() < current.value()
            };

            if improved {
                current = neighbor;
                self.notify_observers(&AlgorithmEvent::BestSolutionUpdate {
                    generation: iteration,
                    solution: current.copy(),
                });
            }

            if iteration % 10 == 0 || improved {
                let fit = current.value();
                self.notify_observers(&AlgorithmEvent::GenerationCompleted {
                    generation: iteration,
                    evaluations,
                    best_fitness: fit,
                    average_fitness: fit,
                    worst_fitness: fit,
                });
            }
        }

        self.notify_observers(&AlgorithmEvent::End {
            total_generations: self.parameters.max_iterations,
            total_evaluations: evaluations,
        });

        for observer in &mut self.observers {
            observer.finalize();
        }

        let mut result = VectorSolutionSet::new();
        result.add_solution(current);
        self.solution_set = Some(result.clone());
        result
    }

    fn validate_parameters(&self) -> bool {
        self.parameters.max_iterations > 0
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
