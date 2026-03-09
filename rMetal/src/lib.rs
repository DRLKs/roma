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
	GeneticAlgorithmParameters,
	HillClimbing,
	HillClimbingExperiment,
	HillClimbingExperimentConfig,
	HillClimbingParameters,
	ExecutionStateSnapshot,
	ImprovementDirection,
	NSGAII,
	NSGAIIParameters,
	TerminationController,
	TerminationCriteria,
	TerminationCriterion,
	TerminationReason,
	TerminationState,
	RuntimeExecutionOutput,
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
pub use observer::{
	AlgorithmEvent,
	AlgorithmObserver,
	ChartObserver,
	ConsoleObserver,
	ExperimentConsoleObserver,
	ExperimentEvent,
	ExperimentObservable,
	ExperimentObserver,
	HtmlReportObserver,
	Observable,
};
pub use experiment::{
	AlgorithmConfiguration,
	Experiment,
	ExperimentReport,
	ExperimentRunResult,
	ExperimentSummary,
	ExperimentableAlgorithm,
	Objective,
};
pub use solution::{
	BinarySolutionBuilder,
	Dominance,
	ParetoCrowdingDistanceQuality,
	MultiObjectiveRealSolutionBuilder,
	MultiObjectiveVectorRealSolutionBuilder,
	RealSolutionBuilder,
	ScalarDominanceDirection,
	Solution,
	StringSolutionBuilder,
	scalar_dominance_direction,
	set_scalar_dominance_direction,
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
		HillClimbingExperiment,
		HillClimbingExperimentConfig,
		HillClimbingParameters,
		ExecutionStateSnapshot,
		ImprovementDirection,
		NSGAII,
		NSGAIIParameters,
		TerminationController,
		TerminationCriteria,
		TerminationCriterion,
		TerminationReason,
		RuntimeExecutionOutput,
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
		ExperimentConsoleObserver,
		ExperimentEvent,
		ExperimentObservable,
		ExperimentObserver,
		HtmlReportObserver,
		Observable,
	};

	pub use crate::experiment::{
		AlgorithmConfiguration,
		Experiment,
		ExperimentReport,
		ExperimentRunResult,
		ExperimentSummary,
		ExperimentableAlgorithm,
		Objective,
	};

	pub use crate::solution::{
		BinarySolutionBuilder,
		Dominance,
		ParetoCrowdingDistanceQuality,
		MultiObjectiveRealSolutionBuilder,
		MultiObjectiveVectorRealSolutionBuilder,
		RealSolutionBuilder,
		ScalarDominanceDirection,
		Solution,
		StringSolutionBuilder,
		scalar_dominance_direction,
		set_scalar_dominance_direction,
	};

	pub use crate::solution_set::{SolutionSet, VectorSolutionSet};

	pub use crate::utils::{seed_from_cli_or, seed_from_time, Random};
}
