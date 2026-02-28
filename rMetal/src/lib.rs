pub mod algorithms;
pub mod problem;
pub mod solution_set;
pub mod solution;
pub mod operator;
pub mod observer;
pub mod utils;

// Top-level re-exports for ergonomic imports.
pub use algorithms::{
	Algorithm,
	GeneticAlgorithm,
	GeneticAlgorithmParameters,
	HillClimbing,
	HillClimbingParameters,
	NSGAII,
	NSGAIIParameters,
};
pub use operator::{
	BitFlipMutation,
	BinaryTournamentSelection,
	CrossoverOperator,
	MutationOperator,
	MultiObjectiveTournamentSelection,
	Operator,
	PolynomialMutation,
	SBXCrossover,
	SelectionOperator,
	SinglePointCrossover,
};
pub use problem::{KnapsackBuilder, KnapsackProblem, Problem, ZDT1Problem};
pub use observer::{AlgorithmEvent, AlgorithmObserver, ChartObserver, ConsoleObserver, Observable};
pub use solution::{
	BinarySolutionBuilder,
	MultiObjectiveInfo,
	MultiObjectiveRealSolutionBuilder,
	RealSolutionBuilder,
	Solution,
	StringSolutionBuilder,
};
pub use solution_set::{SolutionSet, VectorSolutionSet};

/// Commonly used types and traits.
///
/// Library users can import `rMetal::prelude::*` to get a practical baseline.
pub mod prelude {
	pub use crate::algorithms::{
		Algorithm,
		GeneticAlgorithm,
		GeneticAlgorithmParameters,
		HillClimbing,
		HillClimbingParameters,
		NSGAII,
		NSGAIIParameters,
	};

	pub use crate::operator::{
		BitFlipMutation,
		BinaryTournamentSelection,
		CrossoverOperator,
		MutationOperator,
		MultiObjectiveTournamentSelection,
		Operator,
		PolynomialMutation,
		SBXCrossover,
		SelectionOperator,
		SinglePointCrossover,
	};

	pub use crate::problem::{KnapsackBuilder, KnapsackProblem, Problem, ZDT1Problem};

	pub use crate::observer::{
		AlgorithmEvent,
		AlgorithmObserver,
		ChartObserver,
		ConsoleObserver,
		Observable,
	};

	pub use crate::solution::{
		BinarySolutionBuilder,
		MultiObjectiveInfo,
		MultiObjectiveRealSolutionBuilder,
		RealSolutionBuilder,
		Solution,
		StringSolutionBuilder,
	};

	pub use crate::solution_set::{SolutionSet, VectorSolutionSet};

	pub use crate::utils::{seed_from_cli_or, seed_from_time, Random};
}
