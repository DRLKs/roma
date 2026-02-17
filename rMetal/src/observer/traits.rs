use crate::solutions::traits::Solution;

/// Events that can be observed during algorithm execution
#[derive(Debug, Clone)]
pub enum AlgorithmEvent<T, S>
where
    S: Solution<T>,
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
        solution: S,
    },
    /// Algorithm has finished
    End {
        total_generations: usize,
        total_evaluations: usize,
    },
    _Phantom(std::marker::PhantomData<T>),
}

/// Trait for observing algorithm execution
/// 
/// Observers can monitor the algorithm's progress and perform actions
pub trait AlgorithmObserver<T, S>
where
    S: Solution<T>,
    T: Clone,
{
    /// Called when an event occurs during algorithm execution
    fn update(&mut self, event: &AlgorithmEvent<T, S>);

    /// Called at the end of the algorithm to finalize any resources
    fn finalize(&mut self) {}

    /// Returns the name of this observer
    fn name(&self) -> &str;
}

/// Trait for objects that can be observed (algorithms)
pub trait Observable<T, S>
where
    S: Solution<T>,
    T: Clone,
{
    /// Adds an observer to this observable
    fn add_observer(&mut self, observer: Box<dyn AlgorithmObserver<T, S>>);

    /// Removes all observers
    fn clear_observers(&mut self);

    /// Notifies all observers of an event
    fn notify_observers(&mut self, event: &AlgorithmEvent<T, S>);
}
