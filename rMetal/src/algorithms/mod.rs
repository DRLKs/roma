pub(crate) mod async_runner;
pub(crate) mod implementations;
pub(crate) mod objective;
pub(crate) mod runtime;
pub(crate) mod termination;
pub(crate) mod traits;

pub use async_runner::{run_algorithm_instances_async, run_algorithms_async};
pub use implementations::genetic_algorithm::{
    GeneticAlgorithm, GeneticAlgorithmExperiment, GeneticAlgorithmParameters,
};
pub use implementations::hill_climbing::{HillClimbing, HillClimbingParameters};
pub use implementations::nsga2::{NSGAIIParameters, NSGAII};
pub use implementations::pso::{PSOParameters, PSO};
pub use implementations::simulated_annealing::{SimulatedAnnealing, SimulatedAnnealingParameters};
pub use objective::{best_worst, is_better, non_improving_loss, ImprovementDirection};
pub use runtime::{spawn_algorithm_run, ExecutionContext};
pub use termination::{
    ExecutionStateSnapshot, TerminationController, TerminationCriteria, TerminationCriterion,
    TerminationReason, TerminationState,
};
pub use traits::Algorithm;
