use std::cmp::Ordering;

pub trait QualityIndicator {
    type Fitness: FitnessIndicatorTrait;
    
    fn new() -> Self;
    
    fn invalidate(&mut self) {
        self.get_fitness_mut().invalidate();
    }
    
    fn get_fitness(&self) -> &Self::Fitness;
    
    fn get_fitness_mut(&mut self) -> &mut Self::Fitness;
    
    fn set_fitness(&mut self, fitness: Self::Fitness);
}

/// Fitness del Quality Indicator
pub trait FitnessIndicatorTrait: PartialOrd {
    fn invalidate(&mut self);
}


pub struct FitnessIndicator<T> where T: PartialOrd {
    fitness: Option<T>,
}

impl<T> FitnessIndicator<T> where T: PartialOrd {
    pub fn new() -> Self {
        Self { fitness: None }
    }

    pub fn get_value(&self) -> Option<&T> {
        self.fitness.as_ref()
    }

    pub fn set_value(&mut self, value: T) {
        self.fitness = Some(value);
    }
}

impl<T> PartialEq<Self> for FitnessIndicator<T>
where
    T: PartialOrd,
{
    fn eq(&self, other: &Self) -> bool {
        self.fitness == other.fitness
    }
}

impl<T> PartialOrd for FitnessIndicator<T>
where
    T: PartialOrd,
{
    fn partial_cmp(&self, other: &Self) -> Option<Ordering> {
        self.fitness.partial_cmp(&other.fitness)
    }
}

impl<T> FitnessIndicatorTrait for FitnessIndicator<T>
where T: PartialOrd
{
    fn invalidate(&mut self) {
        self.fitness = None;
    }
}




