extern crate self as rmetal;

pub mod algorithms;
pub mod experiment;
pub mod observer;
pub mod operator;
pub mod problem;
pub mod solution;
pub mod solution_set;
pub mod utils;

pub use rmetal_derive::{rmetal_algorithm, rmetal_case, AlgorithmCase, Observable};

// Top-level re-exports for ergonomic imports.
pub use algorithms::{
    run_algorithm_instances_async, run_algorithms_async, spawn_algorithm_run, Algorithm,
    ExecutionStateSnapshot, GeneticAlgorithm, GeneticAlgorithmParameters, HillClimbing,
    HillClimbingParameters, ImprovementDirection, NSGAIIParameters, PSOParameters,
    SimulatedAnnealing, SimulatedAnnealingParameters, TerminationController, TerminationCriteria,
    TerminationCriterion, TerminationReason, TerminationState, NSGAII, PSO,
};
pub use experiment::Experiment;
pub use observer::{
    AlgorithmEvent, AlgorithmObserver, ChartObserver, ConsoleObserver, HtmlReportObserver,
    Observable,
};
pub use operator::{
    BinaryTournamentSelection, BitFlipMutation, CrossoverOperator,
    MultiObjectiveTournamentSelection, MutationOperator, Operator, PolynomialMutation,
    SBXCrossover, SelectionOperator, SinglePointCrossover, SwapMutation,
};
pub use problem::{
    build_knapsack_from_records, build_tsp_from_records, KnapsackBuilder, KnapsackProblem, Problem,
    TspProblem, ZDT1Problem,
};
pub use solution::{
    BinarySolutionBuilder, Dominance, MultiObjectiveRealSolutionBuilder,
    MultiObjectiveVectorRealSolutionBuilder, ParetoCrowdingDistanceQuality,
    PermutationSolutionBuilder, RealSolutionBuilder, Solution, StringSolutionBuilder,
};
pub use solution_set::{DequeSolutionSet, SolutionSet, VectorSolutionSet};
pub use utils::{
    checkpoint_dir_candidates, checkpoint_file_path, ensure_checkpoint_dir,
    initialize_checkpoint_dir, latest_checkpoint_record, latest_checkpoint_record_for_algorithm,
    latest_resumable_checkpoint_for, list_checkpoint_run_ids, list_checkpoints,
    read_checkpoint_record, resolve_checkpoint_dir, write_checkpoint_record, CheckpointDirSource,
    CheckpointInitMode, CheckpointInitResult, CheckpointPathConfig, CheckpointRecord,
    CheckpointRunStatus,
};

/// Commonly used types and traits.
///
/// Library users can import `rMetal::prelude::*` to get a practical baseline.
pub mod prelude {
    pub use crate::algorithms::{
        run_algorithm_instances_async, run_algorithms_async, spawn_algorithm_run, Algorithm,
        ExecutionStateSnapshot, GeneticAlgorithm, GeneticAlgorithmParameters, HillClimbing,
        HillClimbingParameters, ImprovementDirection, NSGAIIParameters, PSOParameters,
        SimulatedAnnealing, SimulatedAnnealingParameters, TerminationController,
        TerminationCriteria, TerminationCriterion, TerminationReason, NSGAII, PSO,
    };

    pub use crate::operator::{
        BinaryTournamentSelection, BitFlipMutation, CrossoverOperator,
        MultiObjectiveTournamentSelection, MutationOperator, Operator, PolynomialMutation,
        SBXCrossover, SelectionOperator, SinglePointCrossover, SwapMutation,
    };

    pub use crate::problem::{
        build_knapsack_from_records, build_tsp_from_records, KnapsackBuilder, KnapsackProblem,
        Problem, TspProblem, ZDT1Problem,
    };

    pub use crate::observer::{
        AlgorithmEvent, AlgorithmObserver, ChartObserver, ConsoleObserver, HtmlReportObserver,
        Observable,
    };

    pub use crate::experiment::Experiment;

    pub use crate::solution::{
        BinarySolutionBuilder, Dominance, MultiObjectiveRealSolutionBuilder,
        MultiObjectiveVectorRealSolutionBuilder, ParetoCrowdingDistanceQuality,
        PermutationSolutionBuilder, RealSolutionBuilder, Solution, StringSolutionBuilder,
    };

    pub use crate::solution_set::{DequeSolutionSet, SolutionSet, VectorSolutionSet};

    pub use crate::utils::{
        checkpoint_dir_candidates, checkpoint_file_path, ensure_checkpoint_dir,
        initialize_checkpoint_dir, latest_checkpoint_record,
        latest_checkpoint_record_for_algorithm, latest_resumable_checkpoint_for,
        list_checkpoint_run_ids, list_checkpoints, read_checkpoint_record, resolve_checkpoint_dir,
        seed_from_cli_or, seed_from_time, write_checkpoint_record, CheckpointDirSource,
        CheckpointInitMode, CheckpointInitResult, CheckpointPathConfig, CheckpointRecord,
        CheckpointRunStatus, Random,
    };
}
