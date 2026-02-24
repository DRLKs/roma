//! Core solution abstractions and builders.
//!
//! This module provides:
//! - A generic `Solution<T, Q>` type with pluggable quality metadata.
//! - Quality models for single-objective and multi-objective optimization.
//! - Convenience builders for common variable types.

use crate::utils::random::{seed_from_time, Random};

/// Trait implemented by quality metadata containers.
///
/// It allows `Solution` to invalidate quality values without knowing
/// the concrete quality representation.
pub trait QualityState {
    /// Resets quality values to an unevaluated state.
    fn invalidate(&mut self);
}

/// Quality metadata for single-objective optimization.
#[derive(Clone, Debug, Default, PartialEq)]
pub struct ScalarQuality {
    /// `None` means the solution has not been evaluated yet.
    pub fitness: Option<f64>,
}

/// Defines a scalar view for any quality model.
pub trait QualityValue {
    fn value(&self) -> f64;
}

impl QualityState for ScalarQuality {
    fn invalidate(&mut self) {
        self.fitness = None;
    }
}

impl QualityValue for ScalarQuality {
    fn value(&self) -> f64 {
        self.fitness.unwrap_or(0.0)
    }
}

/// Quality metadata for multi-objective optimization.
///
/// The structure stores objective values and NSGA-II related metadata.
#[derive(Clone, Debug, Default, PartialEq)]
pub struct MultiObjectiveInfo {
    /// Objective values associated with this solution.
    pub objectives: Vec<f64>,
    /// Pareto rank (lower is better).
    pub rank: Option<usize>,
    /// Crowding distance (higher is better when rank is equal).
    pub crowding_distance: Option<f64>,
}

impl QualityValue for MultiObjectiveInfo {
    fn value(&self) -> f64 {
        self.objectives.first().copied().unwrap_or(0.0)
    }
}

impl QualityState for MultiObjectiveInfo {
    fn invalidate(&mut self) {
        self.objectives.clear();
        self.rank = None;
        self.crowding_distance = None;
    }
}

/// Generic optimization solution.
///
/// # Type Parameters
/// - `T`: variable type.
/// - `Q`: quality metadata type (defaults to `ScalarQuality`).
#[derive(Clone, Debug)]
pub struct Solution<T, Q = ScalarQuality> {
    /// Decision variables.
    pub variables: Vec<T>,
    /// Quality metadata.
    pub quality: Q,
}

impl<T, Q: Default> Solution<T, Q> {
    /// Creates a new solution with default quality metadata.
    pub fn new(variables: Vec<T>) -> Self {
        Self {
            variables,
            quality: Q::default(),
        }
    }
    
    /// Returns an immutable view of variables.
    pub fn variables(&self) -> &[T] {
        &self.variables
    }
    
    /// Returns a mutable view of variables.
    pub fn variables_mut(&mut self) -> &mut [T] {
        &mut self.variables
    }

    /// Returns an immutable reference to quality metadata.
    pub fn quality(&self) -> &Q {
        &self.quality
    }

    /// Returns a mutable reference to quality metadata.
    pub fn quality_mut(&mut self) -> &mut Q {
        &mut self.quality
    }

    /// Returns a variable by index.
    pub fn get_variable(&self, index: usize) -> Option<&T> {
        self.variables.get(index)
    }

    /// Returns a mutable variable by index.
    pub fn get_variable_mut(&mut self, index: usize) -> Option<&mut T> {
        self.variables.get_mut(index)
    }
}

impl<T, Q> Solution<T, Q>
where
    Q: QualityState,
{
    /// Invalidates the quality metadata.
    pub fn invalidate(&mut self){
        self.quality.invalidate();
    }
}

impl<T, Q> Solution<T, Q> {
    /// Returns the number of decision variables.
    pub fn num_variables(&self) -> usize {
        self.variables.len()
    }

    /// Compatibility alias used by legacy code.
    pub fn get_number_of_variables(&self) -> usize {
        self.num_variables()
    }

    /// Clone helper kept for compatibility with legacy algorithm code.
    pub fn copy(&self) -> Self
    where
        T: Clone,
        Q: Clone,
    {
        self.clone()
    }
}

impl<T, Q> Solution<T, Q>
where
    Q: QualityValue,
{
    /// Returns a scalar proxy value from the quality state.
    pub fn value(&self) -> f64 {
        self.quality.value()
    }
}

impl<T> Solution<T, ScalarQuality> {
    /// Returns the scalar fitness value.
    pub fn fitness(&self) -> Option<f64> {
        self.quality.fitness
    }
    
    /// Sets the scalar fitness value.
    pub fn set_fitness(&mut self, fitness: f64) {
        self.quality.fitness = Some(fitness);
    }

}

impl<T> Solution<T, MultiObjectiveInfo> {
    /// Returns the objective vector.
    pub fn objectives(&self) -> &[f64] {
        &self.quality.objectives
    }

    /// Sets the objective vector.
    pub fn set_objectives(&mut self, objectives: Vec<f64>) {
        self.quality.objectives = objectives;
    }

    /// Returns the Pareto rank.
    pub fn rank(&self) -> Option<usize> {
        self.quality.rank
    }

    /// Sets the Pareto rank.
    pub fn set_rank(&mut self, rank: usize) {
        self.quality.rank = Some(rank);
    }

    /// Returns the crowding distance.
    pub fn crowding_distance(&self) -> Option<f64> {
        self.quality.crowding_distance
    }

    /// Sets the crowding distance.
    pub fn set_crowding_distance(&mut self, distance: f64) {
        self.quality.crowding_distance = Some(distance);
    }

    /// Returns objective value by index.
    pub fn get_objective(&self, index: usize) -> Option<f64> {
        self.quality.objectives.get(index).copied()
    }

    /// Returns all objectives if present.
    pub fn get_objectives(&self) -> Option<&[f64]> {
        if self.quality.objectives.is_empty() {
            None
        } else {
            Some(&self.quality.objectives)
        }
    }

    /// Returns true if this solution Pareto-dominates `other` (minimization).
    pub fn dominates(&self, other: &Self) -> bool {
        if self.quality.objectives.len() != other.quality.objectives.len() {
            return false;
        }

        let mut at_least_one_better = false;
        for (a, b) in self
            .quality
            .objectives
            .iter()
            .zip(other.quality.objectives.iter())
        {
            if a > b {
                return false;
            }
            if a < b {
                at_least_one_better = true;
            }
        }
        at_least_one_better
    }

}

fn finalize_scalar_solution<T>(variables: Vec<T>, fitness: Option<f64>) -> Solution<T> {
    let mut solution = Solution::new(variables);
    if let Some(fitness) = fitness {
        solution.set_fitness(fitness);
    }
    solution
}

fn apply_bounds(
    mut variables: Vec<f64>,
    lower_bounds: &Option<Vec<f64>>,
    upper_bounds: &Option<Vec<f64>>,
) -> Vec<f64> {
    if let (Some(lower), Some(upper)) = (lower_bounds, upper_bounds) {
        for i in 0..variables.len() {
            if i < lower.len() && i < upper.len() {
                variables[i] = variables[i].clamp(lower[i], upper[i]);
            }
        }
    }

    variables
}

/// Builder for binary solutions (`Solution<bool>`).
pub struct BinarySolutionBuilder {
    variables: Vec<bool>,
    fitness: Option<f64>,
}

impl BinarySolutionBuilder {
    /// Creates a binary builder initialized with all `true` values.
    pub fn ones(size: usize) -> Self {
        Self {
            variables: vec![true; size],
            fitness: None,
        }
    }

    /// Creates a binary builder initialized with all `false` values.
    pub fn zeros(size: usize) -> Self {
        Self {
            variables: vec![false; size],
            fitness: None,
        }
    }
    
    /// Creates a binary builder with random bits.
    ///
    /// If `seed` is `None`, a time-based seed is used.
    pub fn random(size: usize, seed: Option<u64>) -> Self {
        let mut rng = if let Some(seed) = seed {
            Random::new(seed)
        } else {
            Random::new(seed_from_time())
        };
        let variables: Vec<bool> = (0..size).map(|_| rng.coin_flip()).collect();
        Self {
            variables,
            fitness: None,
        }
    }
    
    /// Creates a builder from an existing variable vector.
    pub fn from_variables(variables: Vec<bool>) -> Self {
        Self {
            variables,
            fitness: None,
        }
    }
    
    /// Replaces the current variable vector.
    pub fn with_variables(mut self, variables: Vec<bool>) -> Self {
        self.variables = variables;
        self
    }
    
    /// Sets an optional initial scalar fitness.
    pub fn with_fitness(mut self, fitness: f64) -> Self {
        self.fitness = Some(fitness);
        self
    }
    
    /// Sets a bit to `true` if `index` is within bounds.
    pub fn set_bit(mut self, index: usize) -> Self {
        if index < self.variables.len() {
            self.variables[index] = true;
        }
        self
    }
    
    /// Sets a bit to `false` if `index` is within bounds.
    pub fn clear_bit(mut self, index: usize) -> Self {
        if index < self.variables.len() {
            self.variables[index] = false;
        }
        self
    }
    
    /// Copies a pattern into the solution prefix.
    ///
    /// Copy length is `min(self.len(), pattern.len())`.
    pub fn with_pattern(mut self, pattern: &[bool]) -> Self {
        let len = self.variables.len().min(pattern.len());
        self.variables[..len].copy_from_slice(&pattern[..len]);
        self
    }
    
    /// Builds the final binary solution.
    pub fn build(self) -> Solution<bool> {
        finalize_scalar_solution(self.variables, self.fitness)
    }
}

/// Builder for real-valued single-objective solutions (`Solution<f64>`).
pub struct RealSolutionBuilder {
    variables: Vec<f64>,
    fitness: Option<f64>,
    lower_bounds: Option<Vec<f64>>,
    upper_bounds: Option<Vec<f64>>,
}

impl RealSolutionBuilder {
    /// Creates a builder with `size` variables initialized to `0.0`.
    pub fn new(size: usize) -> Self {
        Self {
            variables: vec![0.0; size],
            fitness: None,
            lower_bounds: None,
            upper_bounds: None,
        }
    }
    
    /// Creates a builder from an existing variable vector.
    pub fn from_variables(variables: Vec<f64>) -> Self {
        Self {
            variables,
            fitness: None,
            lower_bounds: None,
            upper_bounds: None,
        }
    }
    
    /// Replaces the current variable vector.
    pub fn with_variables(mut self, variables: Vec<f64>) -> Self {
        self.variables = variables;
        self
    }
    
    /// Sets an optional initial scalar fitness.
    pub fn with_fitness(mut self, fitness: f64) -> Self {
        self.fitness = Some(fitness);
        self
    }

    /// Sets per-variable lower bounds.
    pub fn with_lower_bounds(mut self, bounds: Vec<f64>) -> Self {
        self.lower_bounds = Some(bounds);
        self
    }

    /// Sets per-variable upper bounds.
    pub fn with_upper_bounds(mut self, bounds: Vec<f64>) -> Self {
        self.upper_bounds = Some(bounds);
        self
    }

    /// Sets a uniform lower/upper bound for all variables.
    pub fn with_bounds(mut self, lower: f64, upper: f64) -> Self {
        let size = self.variables.len();
        self.lower_bounds = Some(vec![lower; size]);
        self.upper_bounds = Some(vec![upper; size]);
        self
    }
    
    /// Sets one variable if `index` is within bounds.
    pub fn set_variable(mut self, index: usize, value: f64) -> Self {
        if index < self.variables.len() {
            self.variables[index] = value;
        }
        self
    }
    
    /// Fills all variables with the same value.
    pub fn fill(mut self, value: f64) -> Self {
        self.variables.fill(value);
        self
    }
    
    /// Builds the final single-objective real solution.
    pub fn build(self) -> Solution<f64> {
        let variables = apply_bounds(self.variables, &self.lower_bounds, &self.upper_bounds);
        finalize_scalar_solution(variables, self.fitness)
    }

    /// Converts this builder into the multi-objective real builder.
    pub fn into_multi_objective(self) -> MultiObjectiveRealSolutionBuilder {
        MultiObjectiveRealSolutionBuilder {
            variables: self.variables,
            objectives: vec![],
            rank: None,
            crowding_distance: None,
            lower_bounds: self.lower_bounds,
            upper_bounds: self.upper_bounds,
        }
    }
}

/// Builder for multi-objective real solutions (`Solution<f64, MultiObjectiveInfo>`).
pub struct MultiObjectiveRealSolutionBuilder {
    variables: Vec<f64>,
    objectives: Vec<f64>,
    rank: Option<usize>,
    crowding_distance: Option<f64>,
    lower_bounds: Option<Vec<f64>>,
    upper_bounds: Option<Vec<f64>>,
}

impl MultiObjectiveRealSolutionBuilder {
    /// Creates a builder from an existing variable vector.
    pub fn from_variables(variables: Vec<f64>) -> Self {
        Self {
            variables,
            objectives: vec![],
            rank: None,
            crowding_distance: None,
            lower_bounds: None,
            upper_bounds: None,
        }
    }

    /// Sets objective values.
    pub fn with_objectives(mut self, objectives: Vec<f64>) -> Self {
        self.objectives = objectives;
        self
    }

    /// Sets Pareto rank.
    pub fn with_rank(mut self, rank: usize) -> Self {
        self.rank = Some(rank);
        self
    }

    /// Sets crowding distance.
    pub fn with_crowding_distance(mut self, distance: f64) -> Self {
        self.crowding_distance = Some(distance);
        self
    }

    /// Sets per-variable lower bounds.
    pub fn with_lower_bounds(mut self, bounds: Vec<f64>) -> Self {
        self.lower_bounds = Some(bounds);
        self
    }

    /// Sets per-variable upper bounds.
    pub fn with_upper_bounds(mut self, bounds: Vec<f64>) -> Self {
        self.upper_bounds = Some(bounds);
        self
    }

    /// Sets a uniform lower/upper bound for all variables.
    pub fn with_bounds(mut self, lower: f64, upper: f64) -> Self {
        let size = self.variables.len();
        self.lower_bounds = Some(vec![lower; size]);
        self.upper_bounds = Some(vec![upper; size]);
        self
    }

    /// Builds the final multi-objective real solution.
    pub fn build(self) -> Solution<f64, MultiObjectiveInfo> {
        let variables = apply_bounds(self.variables, &self.lower_bounds, &self.upper_bounds);

        let mut solution: Solution<f64, MultiObjectiveInfo> = Solution::new(variables);
        solution.set_objectives(self.objectives);
        if let Some(rank) = self.rank {
            solution.set_rank(rank);
        }
        if let Some(distance) = self.crowding_distance {
            solution.set_crowding_distance(distance);
        }
        solution
    }
}

/// Builder for string-based solutions (`Solution<String>`).
pub struct StringSolutionBuilder {
    variables: Vec<String>,
    fitness: Option<f64>
}

impl StringSolutionBuilder {
    /// Creates a builder with `size` empty strings.
    pub fn new(size: usize) -> Self {
        Self {
            variables: vec!["".to_string(); size],
            fitness: None
        }
    }

    /// Creates a builder from an existing variable vector.
    pub fn from_variables(variables: Vec<String>) -> Self {
        Self {
            variables,
            fitness: None
        }
    }

    /// Replaces the current variable vector.
    pub fn with_variables(mut self, variables: Vec<String>) -> Self {
        self.variables = variables;
        self
    }

    /// Sets an optional initial scalar fitness.
    pub fn with_fitness(mut self, fitness: f64) -> Self {
        self.fitness = Some(fitness);
        self
    }

    /// Sets one variable if `index` is within bounds.
    pub fn set_variable(mut self, index: usize, value: String) -> Self {
        if index < self.variables.len() {
            self.variables[index] = value;
        }
        self
    }

    /// Fills all variables with the same string value.
    pub fn fill(mut self, value: String) -> Self {
        self.variables.fill(value);
        self
    }

    /// Builds the final string solution.
    pub fn build(self) -> Solution<String> {
        finalize_scalar_solution(self.variables, self.fitness)
    }
}



#[cfg(test)]
mod tests {
    use super::*;
    
    #[test]
    fn test_binary_solution_builder_basic() {
        let solution = BinarySolutionBuilder::zeros(5).build();
        assert_eq!(solution.num_variables(), 5);
        assert_eq!(solution.variables(), &[false, false, false, false, false]);
        assert_eq!(solution.fitness(), None);
    }

    #[test]
    fn test_binary_solution_builder_with_fitness() {
        let solution = BinarySolutionBuilder::zeros(3)
            .with_fitness(42.5)
            .build();
        assert_eq!(solution.fitness(), Some(42.5));
    }

    #[test]
    fn test_real_solution_builder_with_bounds() {
        let solution = RealSolutionBuilder::new(3)
            .fill(5.0)
            .with_bounds(0.0, 1.0)
            .build();
        assert_eq!(solution.variables(), &[1.0, 1.0, 1.0]);
    }

    #[test]
    fn test_real_solution_builder_individual_bounds() {
        let solution = RealSolutionBuilder::from_variables(vec![5.0, -5.0, 10.0])
            .with_lower_bounds(vec![0.0, 0.0, 0.0])
            .with_upper_bounds(vec![1.0, 1.0, 1.0])
            .build();
        assert_eq!(solution.variables(), &[1.0, 0.0, 1.0]);
    }

    #[test]
    fn test_multiobjective_builder_rank_and_crowding() {
        let solution = RealSolutionBuilder::new(2)
            .into_multi_objective()
            .with_objectives(vec![0.2, 0.8])
            .with_rank(0)
            .with_crowding_distance(1.5)
            .build();

        assert_eq!(solution.objectives(), &[0.2, 0.8]);
        assert_eq!(solution.rank(), Some(0));
        assert_eq!(solution.crowding_distance(), Some(1.5));
    }
}