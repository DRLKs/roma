use crate::observer::experiment::ExperimentEvent;

/// Observer contract for experiment lifecycle and results.
pub trait ExperimentObserver: Send {
    /// Called every time an experiment event is emitted.
    fn update(&mut self, event: &ExperimentEvent);

    /// Called when experiment execution is complete.
    fn finalize(&mut self) {}

    /// Returns the observer name.
    fn name(&self) -> &str;
}

/// Trait for objects that can register experiment observers.
pub trait ExperimentObservable {
    fn add_experiment_observer(&mut self, observer: Box<dyn ExperimentObserver>);

    fn clear_experiment_observers(&mut self);
}
