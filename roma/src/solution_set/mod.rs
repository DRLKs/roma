//! Containers and traits for algorithm output solution collections.
//!
//! The [`SolutionSet`] trait defines the common API used by algorithms,
//! experiments, and user code to inspect, mutate, and query best solutions.

pub(crate) mod implementations;
pub(crate) mod traits;

pub use implementations::{
    deque_solution_set::DequeSolutionSet, vector_solution_set::VectorSolutionSet,
};
pub use traits::SolutionSet;
