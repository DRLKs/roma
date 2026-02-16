use crate::solutions::traits::Solution;
use crate::problem::traits::Problem;
use crate::algorithms::traits::Algorithm;
use crate::solution_set::traits::SolutionSet;
use crate::solution_set::implementations::vector_solution_set::VectorSolutionSet;
use crate::operator::traits::MutationOperator;
use crate::observer::traits::{AlgorithmEvent, AlgorithmObserver};

/// Parameters for Hill Climbing algorithm.
/// Uses generics to allow any mutation operator.
pub struct HillClimbingParameters<T, S, M>
where
    S: Solution<T> + Clone,
    T: Clone,
    M: MutationOperator<T, S>,
{
    pub max_iterations: usize,
    pub mutation_operator: M,
    pub mutation_probability: f64,
    _phantom: std::marker::PhantomData<(T, S)>,
}

impl<T, S, M> HillClimbingParameters<T, S, M>
where
    S: Solution<T> + Clone,
    T: Clone,
    M: MutationOperator<T, S>,
{
    pub fn new(max_iterations: usize, mutation_operator: M, mutation_probability: f64) -> Self {
        HillClimbingParameters {
            max_iterations,
            mutation_operator,
            mutation_probability,
            _phantom: std::marker::PhantomData,
        }
    }
}

/// Hill Climbing algorithm for single-objective optimization.
/// Now uses a configurable mutation operator to generate neighbors.
pub struct HillClimbing<T, S, M>
where
    S: Solution<T> + Clone,
    T: Clone,
    M: MutationOperator<T, S>,
{
    parameters: HillClimbingParameters<T, S, M>,
    solution_set: Option<VectorSolutionSet<T, S>>,
    is_maximization: bool,
    observers: Vec<Box<dyn AlgorithmObserver<T, S>>>,
}

impl<T, S, M> HillClimbing<T, S, M>
where
    S: Solution<T> + Clone,
    T: Clone,
    M: MutationOperator<T, S>,
{
    pub fn new(parameters: HillClimbingParameters<T, S, M>, maximization: bool) -> Self {
        HillClimbing {
            parameters,
            solution_set: None,
            is_maximization: maximization,
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

impl<T, S, P, M> Algorithm<T, S, P> for HillClimbing<T, S, M>
where
    S: Solution<T> + Clone,
    T: Clone,
    P: Problem<T, S>,
    M: MutationOperator<T, S>,
{
    type SolutionSet = VectorSolutionSet<T, S>;
    type Parameters = HillClimbingParameters<T, S, M>;

    fn run(&mut self, problem: &P, verbose: u8) -> Self::SolutionSet {
        // Notify start
        self.notify_observers(&AlgorithmEvent::Start {
            algorithm_name: "HillClimbing".to_string(),
        });

        if verbose > 0 {
            println!("Starting Hill Climbing with {} iterations", self.parameters.max_iterations);
            println!("  Mutation operator: {}", self.parameters.mutation_operator.name());
        }

        let mut current = problem.create_solution();
        problem.evaluate(&mut current);

        let mut best_value = current.value();
        let mut evaluations = 1;

        // Initial event
        let fitness = current.value();
        self.notify_observers(&AlgorithmEvent::GenerationCompleted {
            generation: 0,
            evaluations,
            best_fitness: fitness,
            average_fitness: fitness,
            worst_fitness: fitness,
        });

        for iteration in 1..=self.parameters.max_iterations {
            // Generate neighbor using mutation operator
            let mut neighbor = current.copy();
            self.parameters.mutation_operator.execute(&mut neighbor, self.parameters.mutation_probability);
            problem.evaluate(&mut neighbor);
            evaluations += 1;

            // For minimization: neighbor < current
            // For maximization: neighbor > current
            let improved = if self.is_maximization {
                neighbor.value() > current.value()
            } else {
                neighbor.value() < current.value()
            };

            if improved {
                current = neighbor;
                best_value = current.value();
                
                // Notify improvement
                self.notify_observers(&AlgorithmEvent::BestSolutionUpdate {
                    generation: iteration,
                    solution: current.copy(),
                });
                
                if verbose > 1 {
                    println!("Iteration {}: Improved to {}", iteration, best_value);
                }
            }

            // Notify iteration completed (every 10 iterations or when improved)
            if iteration % 10 == 0 || improved {
                let fitness = current.value();
                self.notify_observers(&AlgorithmEvent::GenerationCompleted {
                    generation: iteration,
                    evaluations,
                    best_fitness: fitness,
                    average_fitness: fitness,
                    worst_fitness: fitness,
                });
            }
        }

        // Notify end
        self.notify_observers(&AlgorithmEvent::End {
            total_generations: self.parameters.max_iterations,
            total_evaluations: evaluations,
        });

        if verbose > 0 {
            println!("Hill Climbing finished. Best value: {}", best_value);
        }

        // Finalize observers
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
