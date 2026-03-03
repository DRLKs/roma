pub(crate) mod traits;
pub(crate) mod implementations;
pub(crate) mod termination;
pub(crate) mod runtime;

pub use traits::Algorithm;
pub use implementations::genetic_algorithm::{GeneticAlgorithm, GeneticAlgorithmParameters};
pub use implementations::hill_climbing::{HillClimbing, HillClimbingParameters};
pub use implementations::nsga2::{NSGAII, NSGAIIParameters};
pub use termination::{
	ExecutionStateSnapshot,
	ImprovementDirection,
	TerminationController,
	TerminationCriteria,
	TerminationCriterion,
	TerminationReason,
	TerminationState,
};
pub use runtime::{
	ExecutionContext,
	run_with_observers,
};