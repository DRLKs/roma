use crate::solutions::traits::Solution;

/// Calculates fitness statistics from a population
///
/// Returns a tuple (best_fitness, average_fitness, worst_fitness)
pub fn calculate_statistics<T, S>(population: &[S]) -> (f64, f64, f64)
where
    S: Solution<T>,
    T: Clone,
{
    if population.is_empty() {
        return (0.0, 0.0, 0.0);
    }

    let mut best_fitness = f64::MIN;
    let mut worst_fitness = f64::MAX;
    let mut sum_fitness = 0.0;

    for solution in population {
        let fitness = solution.value();
        best_fitness = best_fitness.max(fitness);
        worst_fitness = worst_fitness.min(fitness);
        sum_fitness += fitness;
    }

    let average_fitness = sum_fitness / population.len() as f64;

    (best_fitness, average_fitness, worst_fitness)
}

#[cfg(test)]
mod tests {
    use crate::quality_indicator::implementations::decimal_quality_indicator::DecimalQualityIndicator;
    use super::*;
    use crate::solutions::implementations::binary_solution::BinarySolution;

    #[test]
    fn test_calculate_statistics_empty() {
        let population: Vec<BinarySolution> = vec![];
        let (best, avg, worst) = calculate_statistics(&population);
        assert_eq!(best, 0.0);
        assert_eq!(avg, 0.0);
        assert_eq!(worst, 0.0);
    }

    #[test]
    fn test_calculate_statistics_single() {

        let mut solution = BinarySolution::zeros(10);

        let decimal_quality = 10.0;
        let quality = DecimalQualityIndicator::new(Some(decimal_quality));
        solution.set_quality(quality);

        let population = vec![solution];
        let (best, avg, worst) = calculate_statistics(&population);

        assert_eq!(best, 10.0);
        assert_eq!(avg, 10.0);
        assert_eq!(worst, 10.0);
    }

    #[test]
    fn test_calculate_statistics_multiple() {
        let mut s1 = BinarySolution::zeros(10);
        let mut s2 = BinarySolution::ones(10);
        let mut s3 = BinarySolution::random(10, Some(100));

        let best_quality = 20.0;
        let worst_quality = 10.0;
        let avg_quality = 15.0;

        let q1 = DecimalQualityIndicator::new(Some(best_quality));
        let q2 = DecimalQualityIndicator::new(Some(worst_quality));
        let q3 = DecimalQualityIndicator::new(Some(avg_quality));

        s1.set_quality(q1);
        s2.set_quality(q2);
        s3.set_quality(q3);

        let population = vec![s1, s2, s3];
        let (best, avg, worst) = calculate_statistics(&population);

        assert_eq!(best, 20.0);
        assert_eq!(avg, 15.0);
        assert_eq!(worst, 10.0);
    }
}

