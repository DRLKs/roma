//! Traits and quality models for `Solution`.
//!
//! This module centralizes trait definitions and related implementations used
//! by quality caches.
pub mod evaluator;
mod pareto_crowding_distance_quality;

pub use pareto_crowding_distance_quality::ParetoCrowdingDistanceQuality;
