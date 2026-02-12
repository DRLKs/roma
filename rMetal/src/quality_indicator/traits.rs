use std::cmp::Ordering;

pub trait QualityIndicator {
    type Fitness : PartialOrd;

    fn invalidate(&mut self);
    
    fn get_fitness_indicator(&self) -> &Self::Fitness;
    
    fn get_fitness_indicator_mut(&mut self) -> &mut Self::Fitness;
    
    fn set_fitness_indicator(&mut self, fitness: Self::Fitness);

    fn compare(&self, other: &Self) -> Option<Ordering> {
        self.get_fitness_indicator().partial_cmp(&other.get_fitness_indicator())
    }
}

/// Fitness del Quality Indicator
pub trait FitnessIndicatorTrait: PartialOrd{
    fn invalidate(&mut self);
}








