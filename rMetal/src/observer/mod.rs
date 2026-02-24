pub mod traits;
pub mod implementations;


use crate::solution::Solution;

/// Events that can be observed during algorithm execution
#[derive(Debug, Clone)]
pub enum AlgorithmEvent<T>
where
    T: Clone,
{
    /// Algorithm has started
    Start {
        algorithm_name: String,
    },
    /// A new generation/iteration has been completed
    GenerationCompleted {
        generation: usize,
        evaluations: usize,
        best_fitness: f64,
        worst_fitness: f64,
        average_fitness: f64,
    },
    /// A new best solution has been found
    BestSolutionUpdate {
        generation: usize,
        solution: Solution<T>,
    },
    /// Algorithm has finished
    End {
        total_generations: usize,
        total_evaluations: usize,
    },
    /// An error occurred during execution
    Error {
        message: String,
    },
    _Phantom(std::marker::PhantomData<T>),
}