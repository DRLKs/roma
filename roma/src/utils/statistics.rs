use crate::problem::traits::Problem;
use crate::solution::Solution;

/// Calculates fitness statistics from a population
///
/// Returns a tuple (best_fitness, average_fitness, worst_fitness)
pub fn calculate_statistics<T, P>(
    population: &[Solution<T>],
    problem: &P,
) -> (f64, f64, f64)
where
    T: Clone,
    P: Problem<T>,
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

    let mut best_fitness = values[0];
    let mut worst_fitness = values[0];

    for &value in values.iter().skip(1) {
        if problem.is_better_fitness(value, best_fitness) {
            best_fitness = value;
        }
        if problem.is_better_fitness(worst_fitness, value) {
            worst_fitness = value;
        }
    }

    let average_fitness = sum_fitness / population.len() as f64;

    (best_fitness, average_fitness, worst_fitness)
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::problem::traits::{maximizing_fitness, minimizing_fitness, Problem};
    use crate::solution::implementations::binary_solution::BinarySolutionBuilder;
    use crate::solution::Solution;
    use crate::utils::random::Random;

    struct MaxProblem;
    struct MinProblem;

    impl Problem<bool> for MaxProblem {
        fn new() -> Self where Self: Sized { Self }
        fn evaluate(&self, _solution: &mut Solution<bool>) {}
        fn create_solution(&self, _rng: &mut Random) -> Solution<bool> { panic!("not needed") }
        fn set_problem_description(&mut self, _description: String) {}
        fn get_problem_description(&self) -> String { "max".to_string() }
        fn dominates(&self, solution_a: &Solution<bool>, solution_b: &Solution<bool>) -> bool {
            solution_a.quality_value() > solution_b.quality_value()
        }
        fn better_fitness_fn(&self) -> fn(f64, f64) -> bool { maximizing_fitness }
    }

    impl Problem<bool> for MinProblem {
        fn new() -> Self where Self: Sized { Self }
        fn evaluate(&self, _solution: &mut Solution<bool>) {}
        fn create_solution(&self, _rng: &mut Random) -> Solution<bool> { panic!("not needed") }
        fn set_problem_description(&mut self, _description: String) {}
        fn get_problem_description(&self) -> String { "min".to_string() }
        fn dominates(&self, solution_a: &Solution<bool>, solution_b: &Solution<bool>) -> bool {
            solution_a.quality_value() < solution_b.quality_value()
        }
        fn better_fitness_fn(&self) -> fn(f64, f64) -> bool { minimizing_fitness }
    }

    #[test]
    fn test_calculate_statistics_empty() {
        let population: Vec<Solution<bool>> = vec![];
        let (best, avg, worst) = calculate_statistics(&population, &MaxProblem);
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
        let (best, avg, worst) = calculate_statistics(&population, &MaxProblem);

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
        let (best, avg, worst) = calculate_statistics(&population, &MaxProblem);

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
        let (best, avg, worst) = calculate_statistics(&population, &MinProblem);

        assert_eq!(best, 10.0);
        assert_eq!(avg, 15.0);
        assert_eq!(worst, 20.0);
    }
}
