pub(crate) mod traits;
pub(crate) mod implementations;
pub(crate) mod termination;
pub(crate) mod runtime;
pub(crate) mod objective;

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
pub use objective::{
	ImprovementDirection,
	is_better,
	non_improving_loss,
	best_worst,
};