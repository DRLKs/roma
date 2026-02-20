use crate::solutions::traits::Solution;

/// Trait for observing algorithm execution
/// 
/// Observers can monitor the algorithm's progress and perform actions
pub trait AlgorithmObserver<T, S>: Send
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

use std::sync::{Arc, Mutex};
use crate::observer::AlgorithmEvent;

/// Thread-safe wrapper for observers that can be shared across threads
///
/// This wrapper encapsulates Arc<Mutex<>> to allow observers to be notified
/// from parallel contexts without polluting the algorithm logic with
/// thread-safety concerns.
pub struct ThreadSafeObserverCollection<T, S>
where
    S: Solution<T>,
    T: Clone,
{
    observers: Arc<Mutex<Vec<Box<dyn AlgorithmObserver<T, S>>>>>,
}

impl<T, S> ThreadSafeObserverCollection<T, S>
where
    S: Solution<T>,
    T: Clone,
{
    /// Creates a new thread-safe observer collection from a vector of observers
    pub fn new(observers: Vec<Box<dyn AlgorithmObserver<T, S>>>) -> Self {
        ThreadSafeObserverCollection {
            observers: Arc::new(Mutex::new(observers)),
        }
    }

    /// Notifies all observers of an event in a thread-safe manner
    ///
    /// Returns true if notification was successful, false if the lock could not be acquired
    pub fn notify(&self, event: &AlgorithmEvent<T, S>) -> bool {
        if let Ok(mut observers) = self.observers.lock() {
            for observer in observers.iter_mut() {
                observer.update(event);
            }
            true
        } else {
            false
        }
    }

    /// Finalizes all observers (call this at the end of execution)
    pub fn finalize(&self) {
        if let Ok(mut observers) = self.observers.lock() {
            for observer in observers.iter_mut() {
                observer.finalize();
            }
        }
    }

    /// Clones the Arc to create a new handle to the same collection
    ///
    /// This is useful for sharing the collection across threads
    pub fn clone_handle(&self) -> Self {
        ThreadSafeObserverCollection {
            observers: Arc::clone(&self.observers),
        }
    }
}

/// Creates a thread-safe observer collection that can be easily cloned and shared
///
/// This function is a convenient wrapper for creating observer collections
pub fn create_thread_safe_observers<T, S>(
    observers: Vec<Box<dyn AlgorithmObserver<T, S>>>,
) -> ThreadSafeObserverCollection<T, S>
where
    S: Solution<T>,
    T: Clone,
{
    ThreadSafeObserverCollection::new(observers)
}
