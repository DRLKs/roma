use crate::solution::{finalize_scalar_solution, Solution};
use crate::utils::random::{seed_from_time, Random};

/// Builder for binary solutions (`Solution<bool>`).
pub struct BinarySolutionBuilder {
    variables: Vec<bool>,
    quality: Option<f64>,
}

impl BinarySolutionBuilder {
    /// Creates a binary builder initialized with all `true` values.
    pub fn ones(size: usize) -> Self {
        Self {
            variables: vec![true; size],
            quality: None,
        }
    }

    /// Creates a binary builder initialized with all `false` values.
    pub fn zeros(size: usize) -> Self {
        Self {
            variables: vec![false; size],
            quality: None,
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
            quality: None,
        }
    }

    /// Creates a builder from an existing variable vector.
    pub fn from_variables(variables: Vec<bool>) -> Self {
        Self {
            variables,
            quality: None,
        }
    }

    /// Replaces the current variable vector.
    pub fn with_variables(mut self, variables: Vec<bool>) -> Self {
        self.variables = variables;
        self
    }

    /// Sets an optional initial scalar quality value.
    pub fn with_quality(mut self, quality: f64) -> Self {
        self.quality = Some(quality);
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
        finalize_scalar_solution(self.variables, self.quality)
    }
}

#[cfg(test)]
mod tests {
    use crate::solution::implementations::binary_solution::BinarySolutionBuilder;

    #[test]
    fn test_binary_solution_builder_basic() {
        let solution = BinarySolutionBuilder::zeros(5).build();
        assert_eq!(solution.num_variables(), 5);
        assert_eq!(solution.variables(), &[false, false, false, false, false]);
        assert_eq!(solution.quality(), None);
    }

    #[test]
    fn test_binary_solution_builder_with_quality() {
        let solution = BinarySolutionBuilder::zeros(3)
            .with_quality(42.5)
            .build();
        assert_eq!(solution.quality().copied(), Some(42.5));
    }
}