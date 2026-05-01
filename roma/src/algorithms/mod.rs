//! Algorithm layer for optimization execution.
//!
//! This module exposes:
//! - the [`Algorithm`] trait (shared runtime contract),
//! - built-in algorithm implementations,
//! - termination criteria and execution snapshots,
//! - async helpers for running multiple algorithm instances.
//!
//! Typical users consume algorithm types through `roma::algorithms` or
//! `roma::prelude`.

pub(crate) mod async_runner;
pub(crate) mod checkpoint;
pub(crate) mod implementations;
pub(crate) mod objective;
pub(crate) mod runtime;
pub(crate) mod termination;
pub(crate) mod traits;

pub use async_runner::{run_algorithm_instances_async, run_algorithms_async};
pub use implementations::{
    genetic_algorithm::{GeneticAlgorithm, GeneticAlgorithmParameters},
    hill_climbing::{HillClimbing, HillClimbingParameters},
    nsga2::{NSGAIIParameters, NSGAII},
    pso::{PSOParameters, PSO},
    simulated_annealing::{SimulatedAnnealing, SimulatedAnnealingParameters},
};
pub use objective::{best_worst, is_better, non_improving_loss, ImprovementDirection};
pub use runtime::{spawn_algorithm_run, ExecutionContext};
pub use termination::{
    ExecutionStateSnapshot, TerminationController, TerminationCriteria, TerminationCriterion,
    TerminationReason, TerminationState,
};
pub use traits::Algorithm;
