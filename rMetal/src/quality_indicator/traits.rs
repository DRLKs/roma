use std::cmp::Ordering;

/// Defines a common interface for evaluating and comparing solution quality.
///
/// The indicator can be invalidated and updated during the execution of an
/// algorithm, enabling lazy evaluation or dynamic recalculation strategies.
///
/// # Associated Types
///
/// * `Fitness` - The type used to represent the fitness value. It must implement
///   `PartialOrd` to allow comparison between indicators.
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








