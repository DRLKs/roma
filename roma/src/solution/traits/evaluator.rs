
pub fn maximizing_fitness(candidate: f64, reference: f64) -> bool {
    candidate > reference
}

pub fn minimizing_fitness(candidate: f64, reference: f64) -> bool {
    candidate < reference
}