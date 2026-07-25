//! Integration tests for optimization algorithms and their crate-root exports.

#[path = "algorithms/genetic_algorithm.rs"]
mod genetic_algorithm;
#[path = "algorithms/hill_climbing.rs"]
mod hill_climbing;
#[path = "algorithms/metaheuristics_crate_root_exports.rs"]
mod metaheuristics_crate_root_exports;
#[path = "algorithms/nsga2.rs"]
mod nsga2;
#[path = "algorithms/pso.rs"]
mod pso;
#[path = "algorithms/simulated_annealing.rs"]
mod simulated_annealing;
#[path = "algorithms/tsp.rs"]
mod tsp;
