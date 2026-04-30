use crate::algorithms::objective::{ImprovementDirection, best_worst};
use crate::solution::Solution;

/// Calculates fitness statistics from a population
///
/// Returns a tuple (best_fitness, average_fitness, worst_fitness)
pub fn calculate_statistics<T>(
    population: &[Solution<T>],
    direction: ImprovementDirection,
) -> (f64, f64, f64)
where
{
    if population.is_empty() {
        return (0.0, 0.0, 0.0);
    }

    let mut values = Vec::with_capacity(population.len());
    let mut sum_fitness = 0.0;

    for solution in population {
        let fitness = solution.quality_value();
        values.push(fitness);
        sum_fitness += fitness;
    }

    let (best_fitness, worst_fitness) = best_worst(&values, direction);

    let average_fitness = sum_fitness / population.len() as f64;

    (best_fitness, average_fitness, worst_fitness)
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::solution::Solution;
    use crate::solution::implementations::binary_solution::BinarySolutionBuilder;

    #[test]
    fn test_calculate_statistics_empty() {
        let population: Vec<Solution<bool>> = vec![];
        let (best, avg, worst) = calculate_statistics(&population, ImprovementDirection::Maximize);
        assert_eq!(best, 0.0);
        assert_eq!(avg, 0.0);
        assert_eq!(worst, 0.0);
    }

    #[test]
    fn test_calculate_statistics_single() {
        let mut solution: Solution<bool> = Solution::new(vec![]);
        let _fitness = 10.0;
        solution.set_quality(_fitness);

        let population = vec![solution];
        let (best, avg, worst) = calculate_statistics(&population, ImprovementDirection::Maximize);

        assert_eq!(best, _fitness);
        assert_eq!(avg, _fitness);
        assert_eq!(worst, _fitness);
    }

    #[test]
    fn test_calculate_statistics_multiple() {
        let best_quality = 20.0;
        let worst_quality = 10.0;
        let avg_quality = 15.0;

        let s1 = BinarySolutionBuilder::ones(3)
            .with_quality(best_quality)
            .build();
        let s2 = BinarySolutionBuilder::zeros(3)
            .with_quality(worst_quality)
            .build();
        let s3 = BinarySolutionBuilder::random(3, Some(10))
            .with_quality(avg_quality)
            .build();

        let population = vec![s1, s2, s3];
        let (best, avg, worst) = calculate_statistics(&population, ImprovementDirection::Maximize);

        assert_eq!(best, 20.0);
        assert_eq!(avg, 15.0);
        assert_eq!(worst, 10.0);
    }

    #[test]
    fn test_calculate_statistics_minimization() {
        let s1 = BinarySolutionBuilder::ones(3).with_quality(20.0).build();
        let s2 = BinarySolutionBuilder::zeros(3).with_quality(10.0).build();
        let s3 = BinarySolutionBuilder::random(3, Some(10))
            .with_quality(15.0)
            .build();

        let population = vec![s1, s2, s3];
        let (best, avg, worst) = calculate_statistics(&population, ImprovementDirection::Minimize);

        assert_eq!(best, 10.0);
        assert_eq!(avg, 15.0);
        assert_eq!(worst, 20.0);
    }
}
