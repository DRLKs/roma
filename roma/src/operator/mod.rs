//! Variation, neighborhood, selection, and memory operators used by algorithms.
//!
//! This module includes concrete mutation, crossover, and selection operators,
//! plus trait interfaces for custom operator implementations.
//!
//! Convenience submodules [`mutation`], [`crossover`], and [`selection`] provide
//! short access paths to commonly used operator types.

pub(crate) mod crossover_operator_implementations;
pub(crate) mod mutation_operator_implementations;
pub(crate) mod neighborhood_operator_implementations;
pub(crate) mod selection_operator_implementations;
pub(crate) mod tabu_memory_operator_implementations;
pub(crate) mod traits;

pub use traits::{
    CrossoverOperator, MutationOperator, NeighborhoodOperator, Operator, SelectionOperator,
    SolutionTabuMemory, TabuMemoryOperator,
};

pub use mutation_operator_implementations::{
    bit_flip_mutation::BitFlipMutation, polynomial_mutation::PolynomialMutation,
    real_perturbation_mutation::RealPerturbationMutation, swap_mutation::SwapMutation,
};

pub use crossover_operator_implementations::{
    order_crossover::OrderCrossover, sbx_crossover::SBXCrossover,
    single_point_crossover::SinglePointCrossover,
};

pub use selection_operator_implementations::{
    binary_tournament_selection::BinaryTournamentSelection,
    multi_objective_tournament_selection::MultiObjectiveTournamentSelection,
};

pub use neighborhood_operator_implementations::{
    bit_flip_neighborhood::BitFlipNeighborhood, gaussian_neighborhood::GaussianNeighborhood,
    insertion_neighborhood::InsertionNeighborhood, two_opt_neighborhood::TwoOptNeighborhood,
};

pub use tabu_memory_operator_implementations::{
    attribute_tabu_memory::AttributeTabuMemory, frequency_tabu_memory::FrequencyTabuMemory,
};

/// Idiomatic short aliases for operator groups.
pub mod mutation {
    pub use super::mutation_operator_implementations::bit_flip_mutation::BitFlipMutation;
    pub use super::mutation_operator_implementations::polynomial_mutation::PolynomialMutation;
    pub use super::mutation_operator_implementations::real_perturbation_mutation::RealPerturbationMutation;
    pub use super::mutation_operator_implementations::swap_mutation::SwapMutation;
}

/// Idiomatic short aliases for operator groups.
pub mod crossover {
    pub use super::crossover_operator_implementations::order_crossover::OrderCrossover;
    pub use super::crossover_operator_implementations::sbx_crossover::SBXCrossover;
    pub use super::crossover_operator_implementations::single_point_crossover::SinglePointCrossover;
}

/// Idiomatic short aliases for operator groups.
pub mod selection {
    pub use super::selection_operator_implementations::binary_tournament_selection::BinaryTournamentSelection;
    pub use super::selection_operator_implementations::multi_objective_tournament_selection::MultiObjectiveTournamentSelection;
}

/// Idiomatic short aliases for neighborhood operator types.
pub mod neighborhood {
    pub use super::neighborhood_operator_implementations::bit_flip_neighborhood::BitFlipNeighborhood;
    pub use super::neighborhood_operator_implementations::gaussian_neighborhood::GaussianNeighborhood;
    pub use super::neighborhood_operator_implementations::insertion_neighborhood::InsertionNeighborhood;
    pub use super::neighborhood_operator_implementations::two_opt_neighborhood::TwoOptNeighborhood;
}

/// Idiomatic short aliases for tabu memory types.
pub mod memory {
    pub use super::tabu_memory_operator_implementations::attribute_tabu_memory::AttributeTabuMemory;
    pub use super::tabu_memory_operator_implementations::frequency_tabu_memory::FrequencyTabuMemory;
}
