use crate::solution::{Solution, finalize_scalar_solution};

/// Builder for string-based solutions (`Solution<String>`).
pub struct StringSolutionBuilder {
    variables: Vec<String>,
    quality: Option<f64>,
}

impl StringSolutionBuilder {
    /// Creates a builder with `size` empty strings.
    pub fn new(size: usize) -> Self {
        Self {
            variables: vec!["".to_string(); size],
            quality: None,
        }
    }

    /// Creates a builder from an existing variable vector.
    pub fn from_variables(variables: Vec<String>) -> Self {
        Self {
            variables,
            quality: None,
        }
    }

    /// Replaces the current variable vector.
    pub fn with_variables(mut self, variables: Vec<String>) -> Self {
        self.variables = variables;
        self
    }

    /// Sets an optional initial scalar quality value.
    pub fn with_quality(mut self, quality: f64) -> Self {
        self.quality = Some(quality);
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
        finalize_scalar_solution(self.variables, self.quality)
    }
}

#[cfg(test)]
mod tests {
    use super::StringSolutionBuilder;

    #[test]
    fn builder_creates_expected_size() {
        let s = StringSolutionBuilder::new(3).build();
        assert_eq!(s.num_variables(), 3);
        assert_eq!(s.variables(), &["", "", ""]);
    }

    #[test]
    fn fill_and_set_variable_work() {
        let s = StringSolutionBuilder::new(3)
            .fill("x".to_string())
            .set_variable(1, "y".to_string())
            .build();

        assert_eq!(s.variables(), &["x", "y", "x"]);
    }

    #[test]
    fn with_quality_sets_scalar_quality() {
        let s = StringSolutionBuilder::new(2).with_quality(3.5).build();
        assert_eq!(s.quality().copied(), Some(3.5));
    }
}
