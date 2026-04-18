use crate::solution::{finalize_scalar_solution, Solution};

/// Builder for permutation-based solutions (`Solution<usize>`).
///
/// Typical use case: routing problems like TSP.
pub struct PermutationSolutionBuilder {
    variables: Vec<usize>,
    quality: Option<f64>,
}

impl PermutationSolutionBuilder {
    /// Creates an identity permutation `[0, 1, 2, ..., size-1]`.
    pub fn new(size: usize) -> Self {
        Self {
            variables: (0..size).collect(),
            quality: None,
        }
    }

    /// Creates a builder from an existing permutation vector.
    pub fn from_variables(variables: Vec<usize>) -> Self {
        Self {
            variables,
            quality: None,
        }
    }

    /// Replaces the permutation vector.
    pub fn with_variables(mut self, variables: Vec<usize>) -> Self {
        self.variables = variables;
        self
    }

    /// Sets optional scalar quality.
    pub fn with_quality(mut self, quality: f64) -> Self {
        self.quality = Some(quality);
        self
    }

    /// Swaps two positions if both indexes are valid.
    pub fn swap(mut self, i: usize, j: usize) -> Self {
        if i < self.variables.len() && j < self.variables.len() {
            self.variables.swap(i, j);
        }
        self
    }

    /// Builds the final permutation solution.
    pub fn build(self) -> Solution<usize> {
        finalize_scalar_solution(self.variables, self.quality)
    }
}

#[cfg(test)]
mod tests {
    use super::PermutationSolutionBuilder;

    #[test]
    fn identity_permutation_is_created() {
        let s = PermutationSolutionBuilder::new(5).build();
        assert_eq!(s.variables(), &[0, 1, 2, 3, 4]);
    }

    #[test]
    fn swap_works() {
        let s = PermutationSolutionBuilder::new(4).swap(0, 3).build();
        assert_eq!(s.variables(), &[3, 1, 2, 0]);
    }
}
