use crate::solution::{apply_bounds, finalize_scalar_solution, Solution};

/// Builder for real-valued single-objective solutions (`Solution<f64>`).
pub struct RealSolutionBuilder {
    variables: Vec<f64>,
    quality: Option<f64>,
    lower_bounds: Option<Vec<f64>>,
    upper_bounds: Option<Vec<f64>>,
}

impl RealSolutionBuilder {
    /// Creates a builder with `size` variables initialized to `0.0`.
    pub fn new(size: usize) -> Self {
        Self {
            variables: vec![0.0; size],
            quality: None,
            lower_bounds: None,
            upper_bounds: None,
        }
    }

    /// Creates a builder from an existing variable vector.
    pub fn from_variables(variables: Vec<f64>) -> Self {
        Self {
            variables,
            quality: None,
            lower_bounds: None,
            upper_bounds: None,
        }
    }

    /// Replaces the current variable vector.
    pub fn with_variables(mut self, variables: Vec<f64>) -> Self {
        self.variables = variables;
        self
    }

    /// Sets an optional initial scalar quality value.
    pub fn with_quality(mut self, quality: f64) -> Self {
        self.quality = Some(quality);
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
        finalize_scalar_solution(variables, self.quality)
    }
}

#[cfg(test)]
mod tests {
    use super::*;

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
}
