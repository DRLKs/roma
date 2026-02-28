/// Scalar objective alias kept for compatibility.
pub type ScalarQuality = f64;

/// Trait for objective payloads that can expose a scalar proxy value.
pub trait QualityValue {
    fn quality_value(&self) -> f64;
}

impl QualityValue for ScalarQuality {
    fn quality_value(&self) -> f64 {
        *self
    }
}

/// Multi-objective metadata (NSGA-II style).
#[derive(Clone, Debug, Default, PartialEq)]
pub struct MultiObjectiveInfo {
    pub objectives: Vec<f64>,
    pub rank: Option<usize>,
    pub crowding_distance: Option<f64>,
}

impl QualityValue for MultiObjectiveInfo {
    /// Returns the first value of the objectives vector
    fn quality_value(&self) -> f64 {
        self.objectives.first().copied().unwrap_or(0.0)
    }
}