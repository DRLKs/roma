use crate::solution::Solution;

/// Calculates fitness statistics from a population
///
/// Returns a tuple (best_fitness, average_fitness, worst_fitness)
pub fn calculate_statistics<T>(population: &[Solution<T>]) -> (f64, f64, f64)
where
{
    if population.is_empty() {
        return (0.0, 0.0, 0.0);
    }

    let mut best_fitness = f64::MIN;
    let mut worst_fitness = f64::MAX;
    let mut sum_fitness = 0.0;

    for solution in population {
        let fitness = solution.quality_value();
        best_fitness = best_fitness.max(fitness);
        worst_fitness = worst_fitness.min(fitness);
        sum_fitness += fitness;
    }

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
        let (best, avg, worst) = calculate_statistics(&population);
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
        let (best, avg, worst) = calculate_statistics(&population);

        assert_eq!(best, _fitness);
        assert_eq!(avg, _fitness);
        assert_eq!(worst, _fitness);
    }

    #[test]
    fn test_calculate_statistics_multiple() {

        let best_quality = 20.0;
        let worst_quality = 10.0;
        let avg_quality = 15.0;

        let s1 = BinarySolutionBuilder::ones(3).with_quality(best_quality).build();
        let s2 = BinarySolutionBuilder::zeros(3).with_quality(worst_quality).build();
        let s3 = BinarySolutionBuilder::random(3, Some(10)).with_quality(avg_quality).build();

        let population = vec![s1, s2, s3];
        let (best, avg, worst) = calculate_statistics(&population);

        assert_eq!(best, 20.0);
        assert_eq!(avg, 15.0);
        assert_eq!(worst, 10.0);
    }
}

