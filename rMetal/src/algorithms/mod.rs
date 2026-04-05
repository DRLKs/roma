pub(crate) mod traits;
pub(crate) mod implementations;
pub(crate) mod termination;
pub(crate) mod runtime;
pub(crate) mod objective;
pub(crate) mod async_runner;

pub use traits::Algorithm;
pub use implementations::genetic_algorithm::{
	GeneticAlgorithm,
	GeneticAlgorithmExperiment,
	GeneticAlgorithmParameters,
};
pub use implementations::hill_climbing::{
	HillClimbing,
	HillClimbingParameters,
};
pub use implementations::nsga2::{NSGAII, NSGAIIParameters};
pub use implementations::simulated_annealing::{
	SimulatedAnnealing,
	SimulatedAnnealingParameters,
};
pub use implementations::pso::{PSO, PSOParameters};
pub use termination::{
	ExecutionStateSnapshot,
	TerminationController,
	TerminationCriteria,
	TerminationCriterion,
	TerminationReason,
	TerminationState,
};
pub use runtime::{
	ExecutionContext,
	spawn_algorithm_run,
};
pub use async_runner::{
	run_algorithm_instances_async,
	run_algorithms_async,
};
pub use objective::{
	ImprovementDirection,
	is_better,
	non_improving_loss,
	best_worst,
};