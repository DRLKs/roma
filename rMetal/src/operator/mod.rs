pub(crate) mod traits;
pub(crate) mod mutation_operator_implementations;
pub(crate) mod crossover_operator_implementations;
pub(crate) mod selection_operator_implementations;

pub use traits::{CrossoverOperator, MutationOperator, Operator, SelectionOperator};

pub use mutation_operator_implementations::bit_flip_mutation::BitFlipMutation;
pub use mutation_operator_implementations::polynomial_mutation::PolynomialMutation;

pub use crossover_operator_implementations::single_point_crossover::SinglePointCrossover;
pub use crossover_operator_implementations::sbx_crossover::SBXCrossover;

pub use selection_operator_implementations::binary_tournament_selection::BinaryTournamentSelection;
pub use selection_operator_implementations::multi_objective_tournament_selection::MultiObjectiveTournamentSelection;

/// Idiomatic short aliases for operator groups.
pub mod mutation {
	pub use super::mutation_operator_implementations::bit_flip_mutation::BitFlipMutation;
	pub use super::mutation_operator_implementations::polynomial_mutation::PolynomialMutation;
}

/// Idiomatic short aliases for operator groups.
pub mod crossover {
	pub use super::crossover_operator_implementations::single_point_crossover::SinglePointCrossover;
	pub use super::crossover_operator_implementations::sbx_crossover::SBXCrossover;
}

/// Idiomatic short aliases for operator groups.
pub mod selection {
	pub use super::selection_operator_implementations::binary_tournament_selection::BinaryTournamentSelection;
	pub use super::selection_operator_implementations::multi_objective_tournament_selection::MultiObjectiveTournamentSelection;
}