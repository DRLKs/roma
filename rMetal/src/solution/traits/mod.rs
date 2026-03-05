//! Traits and quality models for `Solution`.
//!
//! This module centralizes trait definitions and related implementations used
//! by quality caches.

mod dominance;
mod scalar_dominance;
mod pareto_crowding_distance_quality;

pub use dominance::Dominance;
pub use scalar_dominance::{
    scalar_dominance_direction,
    set_scalar_dominance_direction,
    ScalarDominanceDirection,
};
pub use pareto_crowding_distance_quality::ParetoCrowdingDistanceQuality;
