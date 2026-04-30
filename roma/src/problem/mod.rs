//! Problem definitions and built-in benchmark/problem implementations.
//!
//! The central abstraction is [`Problem`], which defines how to:
//! - create random candidate solutions,
//! - evaluate quality/fitness,
//! - declare objective direction (minimize/maximize),
//! - render domain-specific solution summaries for observers.

pub(crate) mod implementations;
pub(crate) mod traits;

pub use implementations::{
    knapsack_problem::{KnapsackBuilder, KnapsackProblem, build_knapsack_from_records},
    tsp_problem::{TspProblem, build_tsp_from_records},
    zdt1_problem::ZDT1Problem,
};
pub use traits::Problem;
