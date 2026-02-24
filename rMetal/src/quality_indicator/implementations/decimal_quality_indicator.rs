use crate::quality_indicator::traits::{QualityIndicator};

/// Represents a quality indicator based on a single decimal (floating-point) value.
///
/// The fitness value is optional to allow the creation of unevaluated solutions
/// or delayed evaluation strategies.
///
/// # Fields
///
/// * `fitness` - Optional floating-point value representing the quality of a solution.
#[derive(Clone)]
pub struct DecimalQualityIndicator {
    fitness: Option<f64>
}

impl DecimalQualityIndicator {
    pub fn new(fitness: Option<f64>) -> Self {
        Self { fitness }
    }

    pub fn get_fitness(&self) -> Option<f64> {
        self.fitness
    }
}

impl QualityIndicator for DecimalQualityIndicator {
    type Fitness = Option<f64>;

    fn invalidate(&mut self) {
        self.fitness = None;
    }

    fn get_fitness_indicator(&self) -> &Self::Fitness {
        &self.fitness
    }

    fn get_fitness_indicator_mut(&mut self) -> &mut Self::Fitness {
        &mut self.fitness
    }

    fn set_fitness_indicator(&mut self, fitness: Self::Fitness) {
        self.fitness = fitness;
    }
}