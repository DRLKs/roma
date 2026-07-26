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
pub(crate) mod implementations;
pub(crate) mod runtime;
pub(crate) mod termination;
pub(crate) mod traits;

pub use crate::utils::checkpoint::{ExecutionStateSnapshot, StepStateCheckpoint};
pub use async_runner::{run_algorithm_instances_async, run_algorithms_async};
pub use implementations::{
    differential_evolution::{DifferentialEvolution, DifferentialEvolutionParameters},
    genetic_algorithm::{GeneticAlgorithm, GeneticAlgorithmParameters},
    hill_climbing::{HillClimbing, HillClimbingParameters},
    nsga2::{NSGAII, NSGAIIParameters},
    pso::{PSO, PSOParameters},
    simulated_annealing::{SimulatedAnnealing, SimulatedAnnealingParameters},
    tabu_search::{TabuSearch, TabuSearchParameters},
    vns::{VNS, VNSParameters},
};
pub use runtime::{ExecutionContext, spawn_algorithm_run};
pub use termination::{
    TerminationController, TerminationCriteria, TerminationCriterion, TerminationReason,
    TerminationState,
};
pub use traits::Algorithm;
