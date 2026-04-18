use crate::observer::AlgorithmEvent;

/// Trait for observing algorithm execution.
///
/// Observers receive lifecycle and progress events emitted by the runtime.
/// Typical use cases include console output, report generation, metrics export,
/// and external integrations.
pub trait AlgorithmObserver<T, Q = f64>: Send + 'static
where
    T: Clone + Send + 'static,
    Q: Clone + Send + 'static,
{
    /// Called when an event occurs during algorithm execution
    fn update(&mut self, event: &AlgorithmEvent<T, Q>);

    /// Called at the end of the algorithm to finalize any resources
    fn finalize(&mut self) {}

    /// Returns the name of this observer
    fn name(&self) -> &str;
}

/// Trait for objects that can register and manage observers.
pub trait Observable<T, Q = f64>
where
    T: Clone + Send + 'static,
    Q: Clone + Send + 'static,
{
    /// Adds an observer to this observable
    fn add_observer(&mut self, observer: Box<dyn AlgorithmObserver<T, Q>>);

    /// Removes all observers
    fn clear_observers(&mut self);
}
