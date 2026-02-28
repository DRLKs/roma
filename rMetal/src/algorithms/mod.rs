pub(crate) mod traits;
pub(crate) mod implementations;

pub use traits::Algorithm;
pub use implementations::genetic_algorithm::{GeneticAlgorithm, GeneticAlgorithmParameters};
pub use implementations::hill_climbing::{HillClimbing, HillClimbingParameters};
pub use implementations::nsga2::{NSGAII, NSGAIIParameters};