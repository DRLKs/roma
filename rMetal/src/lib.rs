pub mod algorithms;
pub mod problem;
pub mod solution_set;
pub mod solution;
pub mod operator;
pub mod observer;
pub mod utils;
pub mod experiment;

// Top-level re-exports for ergonomic imports.
pub use algorithms::{
	Algorithm,
	GeneticAlgorithm,
	GeneticAlgorithmExperiment,
	GeneticAlgorithmParameters,
	HillClimbing,
	HillClimbingParameters,
	ExecutionStateSnapshot,
	ImprovementDirection,
	NSGAII,
	NSGAIIParameters,
	PSO,
	PSOParameters,
	SimulatedAnnealing,
	SimulatedAnnealingParameters,
	spawn_algorithm_run,
	TerminationController,
	TerminationCriteria,
	TerminationCriterion,
	TerminationReason,
	TerminationState,
};
pub use operator::{
	BitFlipMutation,
	BinaryTournamentSelection,
	CrossoverOperator,
	MutationOperator,
	MultiObjectiveTournamentSelection,
	Operator,
	PolynomialMutation,
	SwapMutation,
	SBXCrossover,
	SelectionOperator,
	SinglePointCrossover,
};
pub use problem::{KnapsackBuilder, KnapsackProblem, Problem, TspProblem, ZDT1Problem};
pub use observer::{
	AlgorithmEvent,
	AlgorithmObserver,
	ChartObserver,
	ConsoleObserver,
	HtmlReportObserver,
	Observable,
};
pub use experiment::{
	Experiment,
	run_algorithms_async,
};
pub use solution::{
	BinarySolutionBuilder,
	Dominance,
	ParetoCrowdingDistanceQuality,
	MultiObjectiveRealSolutionBuilder,
	MultiObjectiveVectorRealSolutionBuilder,
	PermutationSolutionBuilder,
	RealSolutionBuilder,
	Solution,
	StringSolutionBuilder,
};
pub use solution_set::{DequeSolutionSet, SolutionSet, VectorSolutionSet};

/// Commonly used types and traits.
///
/// Library users can import `rMetal::prelude::*` to get a practical baseline.
pub mod prelude {
	pub use crate::algorithms::{
		Algorithm,
		GeneticAlgorithm,
		GeneticAlgorithmExperiment,
		GeneticAlgorithmParameters,
		HillClimbing,
		HillClimbingParameters,
		ExecutionStateSnapshot,
		ImprovementDirection,
		NSGAII,
		NSGAIIParameters,
		PSO,
		PSOParameters,
		SimulatedAnnealing,
		SimulatedAnnealingParameters,
		spawn_algorithm_run,
		TerminationController,
		TerminationCriteria,
		TerminationCriterion,
		TerminationReason,
	};

	pub use crate::operator::{
		BitFlipMutation,
		BinaryTournamentSelection,
		CrossoverOperator,
		MutationOperator,
		MultiObjectiveTournamentSelection,
		Operator,
		PolynomialMutation,
		SwapMutation,
		SBXCrossover,
		SelectionOperator,
		SinglePointCrossover,
	};

	pub use crate::problem::{KnapsackBuilder, KnapsackProblem, Problem, TspProblem, ZDT1Problem};

	pub use crate::observer::{
		AlgorithmEvent,
		AlgorithmObserver,
		ChartObserver,
		ConsoleObserver,
		HtmlReportObserver,
		Observable,
	};

	pub use crate::experiment::{
		Experiment,
		run_algorithms_async,
	};

	pub use crate::solution::{
		BinarySolutionBuilder,
		Dominance,
		ParetoCrowdingDistanceQuality,
		MultiObjectiveRealSolutionBuilder,
		MultiObjectiveVectorRealSolutionBuilder,
		PermutationSolutionBuilder,
		RealSolutionBuilder,
		Solution,
		StringSolutionBuilder,
	};

	pub use crate::solution_set::{DequeSolutionSet, SolutionSet, VectorSolutionSet};

	pub use crate::utils::{seed_from_cli_or, seed_from_time, Random};
}
