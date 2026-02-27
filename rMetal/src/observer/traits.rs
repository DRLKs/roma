use crate::observer::AlgorithmEvent;

/// Trait for observing algorithm execution
/// 
/// Observers can monitor the algorithm's progress and perform actions.
/// Implementations are executed from a dedicated observer thread.
pub trait AlgorithmObserver<T>: Send + 'static
where
    T: Clone + Send + 'static,
{
    /// Called when an event occurs during algorithm execution
    fn update(&mut self, event: &AlgorithmEvent<T>);

    /// Called at the end of the algorithm to finalize any resources
    fn finalize(&mut self) {}

    /// Returns the name of this observer
    fn name(&self) -> &str;
}

/// Trait for objects that can be observed (algorithms)
pub trait Observable<T>
where
    T: Clone + Send + 'static,
{
    /// Adds an observer to this observable
    fn add_observer(&mut self, observer: Box<dyn AlgorithmObserver<T>>);

    /// Removes all observers
    fn clear_observers(&mut self);
}
