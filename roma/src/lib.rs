#![cfg_attr(docsrs, feature(doc_cfg))]
#![deny(rustdoc::broken_intra_doc_links)]
#![doc = include_str!("../README.md")]
//!
//! ## Architecture
//!
//! Roma is organized around five core concepts:
//! - [`problem::Problem`]: domain definition and evaluation logic.
//! - [`solution::Solution`]: decision variables plus cached quality payload.
//! - [`algorithms::Algorithm`]: shared execution lifecycle for optimization methods.
//! - [`solution_set::SolutionSet`]: container abstraction for algorithm outputs.
//! - [`observer::AlgorithmObserver`]: event-based progress and reporting hooks.
//!
//! Built-in implementations cover common mono-objective and multi-objective workflows.
//!
//! ## Execution Lifecycle
//!
//! All algorithms run through a single runtime contract:
//! 1. initialize algorithm state,
//! 2. emit snapshots to observers,
//! 3. evaluate termination criteria,
//! 4. finalize into a solution set.
//!
//! This keeps observer behavior, checkpoint integration, and termination semantics
//! consistent across algorithms.
//!
//! ## Quick Start
//!
//! ```rust
//! use roma_lib::algorithms::{
//!     Algorithm,
//!     HillClimbing,
//!     HillClimbingParameters,
//!     TerminationCriteria,
//!     TerminationCriterion,
//! };
//! use roma_lib::operator::BitFlipMutation;
//! use roma_lib::problem::KnapsackBuilder;
//! use roma_lib::solution_set::SolutionSet;
//!
//! let problem = KnapsackBuilder::new()
//!     .with_capacity(30.0)
//!     .add_items(vec![(4.0, 8.0), (6.0, 12.0), (10.0, 24.0)])
//!     .build();
//!
//! let parameters = HillClimbingParameters::new(
//!     BitFlipMutation::new(),
//!     0.2,
//!     TerminationCriteria::new(vec![TerminationCriterion::MaxIterations(25)]),
//! )
//! .with_seed(7);
//!
//! let mut algorithm = HillClimbing::new(parameters);
//! let result = algorithm.run(&problem)?;
//!
//! let best = result
//!     .best_solution(&problem)
//!     .expect("solution set should not be empty");
//! assert!(best.quality_value().is_finite());
//! # Ok::<(), String>(())
//! ```
//!
//! ## Where to Go Next
//!
//! - Use [`prelude`] for ergonomic imports in applications and demos.
//! - Explore [`experiment::Experiment`] for repeated runs and comparative summaries.
//! - Use [`observer::ConsoleObserver`], [`observer::ChartObserver`], or
//!   [`observer::HtmlReportObserver`] for runtime visibility.

extern crate self as roma_lib;

pub mod algorithms;
pub mod experiment;
pub mod observer;
pub mod operator;
pub mod problem;
pub mod solution;
pub mod solution_set;
pub mod utils;

// Top-level re-exports for ergonomic imports.
pub use algorithms::{
    run_algorithm_instances_async, run_algorithms_async, spawn_algorithm_run, Algorithm,
    ExecutionStateSnapshot, GeneticAlgorithm, GeneticAlgorithmParameters, HillClimbing,
    HillClimbingParameters, NSGAIIParameters, PSOParameters, SimulatedAnnealing,
    SimulatedAnnealingParameters, TerminationController, TerminationCriteria,
    TerminationCriterion, TerminationReason, TerminationState, NSGAII, PSO,
};
pub use experiment::Experiment;
pub use observer::{
    AlgorithmEvent, AlgorithmObserver, ChartObserver, ConsoleObserver, HtmlReportObserver,
    Observable,
};
pub use operator::{
    BinaryTournamentSelection, BitFlipMutation, CrossoverOperator,
    MultiObjectiveTournamentSelection, MutationOperator, Operator, OrderCrossover,
    PolynomialMutation, SBXCrossover, SelectionOperator, SinglePointCrossover, SwapMutation,
};
pub use problem::{
    build_knapsack_from_records, build_tsp_from_records, KnapsackBuilder, KnapsackProblem, Problem,
    TspProblem, ZDT1Problem,
};
pub use solution::{
    BinarySolutionBuilder, MultiObjectiveRealSolutionBuilder,
    MultiObjectiveVectorRealSolutionBuilder, ParetoCrowdingDistanceQuality,
    PermutationSolutionBuilder, RealSolutionBuilder, Solution, StringSolutionBuilder,
};
pub use solution_set::{DequeSolutionSet, SolutionSet, VectorSolutionSet};
pub use utils::{delete_snapshot_on_success, read_snapshot, write_snapshot};

/// Commonly used types and traits.
///
/// Import this module to quickly access the most frequently used Roma APIs.
///
/// ```rust
/// use roma_lib::prelude::*;
///
/// let _criteria = TerminationCriteria::new(vec![TerminationCriterion::MaxIterations(10)]);
/// ```
pub mod prelude {
    pub use crate::algorithms::{
        run_algorithm_instances_async, run_algorithms_async, spawn_algorithm_run, Algorithm,
        ExecutionStateSnapshot, GeneticAlgorithm, GeneticAlgorithmParameters, HillClimbing,
        HillClimbingParameters, NSGAIIParameters, PSOParameters, SimulatedAnnealing,
        SimulatedAnnealingParameters, TerminationController, TerminationCriteria,
        TerminationCriterion, TerminationReason, NSGAII, PSO,
    };

    pub use crate::operator::{
        BinaryTournamentSelection, BitFlipMutation, CrossoverOperator,
        MultiObjectiveTournamentSelection, MutationOperator, Operator, OrderCrossover,
        PolynomialMutation, SBXCrossover, SelectionOperator, SinglePointCrossover,
        SwapMutation,
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
        BinarySolutionBuilder, MultiObjectiveRealSolutionBuilder,
        MultiObjectiveVectorRealSolutionBuilder, ParetoCrowdingDistanceQuality,
        PermutationSolutionBuilder, RealSolutionBuilder, Solution, StringSolutionBuilder,
    };

    pub use crate::solution_set::{DequeSolutionSet, SolutionSet, VectorSolutionSet};

    pub use crate::utils::{
        delete_snapshot_on_success, read_snapshot, seed_from_cli_or, seed_from_time,
        write_snapshot, Random,
    };
}
