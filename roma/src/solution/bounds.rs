/// Bounds metadata for real-valued search spaces.
///
/// This type lives in the solution module because it describes constraints on
/// real-valued decision variables and is consumed directly by solution builders
/// and real-valued operators.
///
/// Problems may expose a shared `RealBounds` instance through
/// `Problem::real_bounds()`, but the type itself is solution-facing metadata.
#[derive(Clone, Debug, PartialEq)]
pub enum RealBounds {
    Uniform {
        lower: f64,
        upper: f64,
        dimensions: usize,
    },
    PerVariable {
        lower_bounds: Vec<f64>,
        upper_bounds: Vec<f64>,
    },
}

impl RealBounds {
    pub fn uniform(lower: f64, upper: f64, dimensions: usize) -> Self {
        debug_assert!(lower <= upper, "lower bound must not exceed upper bound");

        Self::Uniform {
            lower,
            upper,
            dimensions,
        }
    }

    pub fn per_variable(lower_bounds: Vec<f64>, upper_bounds: Vec<f64>) -> Self {
        debug_assert_eq!(
            lower_bounds.len(),
            upper_bounds.len(),
            "lower and upper bounds should have the same length"
        );

        Self::PerVariable {
            lower_bounds,
            upper_bounds,
        }
    }

    pub fn bounds_at(&self, index: usize) -> Option<(f64, f64)> {
        match self {
            Self::Uniform {
                lower,
                upper,
                dimensions,
            } => (index < *dimensions).then_some((*lower, *upper)),
            Self::PerVariable {
                lower_bounds,
                upper_bounds,
            } => Some((
                lower_bounds.get(index).copied()?,
                upper_bounds.get(index).copied()?,
            )),
        }
    }

    pub fn clamp(&self, index: usize, value: f64) -> Option<f64> {
        self.bounds_at(index)
            .map(|(lower, upper)| value.clamp(lower, upper))
    }
}