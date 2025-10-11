use crate::quality_indicator::quality_indicator_trait::{FitnessIndicator, QualityIndicator};

pub struct DecimalQualityIndicator {
    fitness: FitnessIndicator<f64>,
}

impl QualityIndicator for DecimalQualityIndicator {
    type Fitness = FitnessIndicator<f64>;

    fn new() -> Self {
        Self {
            fitness: FitnessIndicator::new(),
        }
    }

    fn get_fitness(&self) -> &Self::Fitness {
        &self.fitness
    }

    fn get_fitness_mut(&mut self) -> &mut Self::Fitness {
        &mut self.fitness
    }

    fn set_fitness(&mut self, fitness: Self::Fitness) {
        self.fitness = fitness;
    }
}