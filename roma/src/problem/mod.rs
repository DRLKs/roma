//! Problem definitions and built-in benchmark/problem implementations.
//!
//! The central abstraction is [`Problem`], which defines how to:
//! - create random candidate solutions,
//! - evaluate quality/fitness,
//! - compare fitness according to problem-owned semantics,
//! - render domain-specific solution summaries for observers.

pub(crate) mod implementations;
pub(crate) mod traits;

pub use implementations::{
    knapsack_problem::{build_knapsack_from_records, KnapsackBuilder, KnapsackProblem},
    rastrigin_problem::RastriginProblem,
    tsp_problem::{build_tsp_from_records, TspProblem},
    zdt1_problem::ZDT1Problem,
};
pub use traits::{maximizing_fitness, minimizing_fitness, Problem};
