use crate::quality_indicator::quality_indicator_trait::{QualityIndicator};

pub struct DecimalQualityIndicator {
    fitness: Option<f64>
}

impl QualityIndicator for DecimalQualityIndicator {
    type Fitness = Option<f64>;

    fn new() -> Self {
        Self {
            fitness: None,
        }
    }

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