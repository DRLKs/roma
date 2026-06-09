use crate::problem::traits::Problem;
use crate::solution::Solution;

/// Summary statistics computed from an evaluated population.
#[derive(Clone, Copy, Debug, PartialEq)]
pub struct PopulationStatistics {
    pub best_index: Option<usize>,
    pub best_fitness: f64,
    pub average_fitness: f64,
    pub worst_fitness: f64,
}

impl PopulationStatistics {
    fn empty() -> Self {
        Self {
            best_index: None,
            best_fitness: 0.0,
            average_fitness: 0.0,
            worst_fitness: 0.0,
        }
    }

    /// Returns `(best, average, worst)` as a compact tuple.
    pub fn as_tuple(self) -> (f64, f64, f64) {
        (self.best_fitness, self.average_fitness, self.worst_fitness)
    }
}

/// Calculates population statistics using each solution's cached quality value.
pub fn calculate_population_statistics<T, P>(
    population: &[Solution<T>],
    problem: &P,
) -> PopulationStatistics
where
    T: Clone,
    P: Problem<T>,
{
    calculate_population_statistics_by(population, problem, |solution| {
        Some(solution.quality_value())
    })
}

/// Calculates population statistics using a custom fitness extractor.
///
/// Solutions for which `fitness_of` returns `None` are skipped. If every
/// solution is skipped, the returned statistics are empty-valued.
pub fn calculate_population_statistics_by<T, Q, P, F>(
    population: &[Solution<T, Q>],
    problem: &P,
    fitness_of: F,
) -> PopulationStatistics
where
    T: Clone,
    Q: Clone,
    P: Problem<T, Q>,
    F: Fn(&Solution<T, Q>) -> Option<f64>,
{
    let mut observed = 0usize;
    let mut sum_fitness = 0.0;
    let mut best_index = None;
    let mut best_fitness = 0.0;
    let mut worst_fitness = 0.0;

    for (index, solution) in population.iter().enumerate() {
        let Some(fitness) = fitness_of(solution) else {
            continue;
        };

        if observed == 0 {
            best_index = Some(index);
            best_fitness = fitness;
            worst_fitness = fitness;
        } else {
            if problem.is_better_fitness(fitness, best_fitness) {
                best_fitness = fitness;
                best_index = Some(index);
            }

            if problem.is_better_fitness(worst_fitness, fitness) {
                worst_fitness = fitness;
            }
        }

        sum_fitness += fitness;
        observed += 1;
    }

    if observed == 0 {
        return PopulationStatistics::empty();
    }

    PopulationStatistics {
        best_index,
        best_fitness,
        average_fitness: sum_fitness / observed as f64,
        worst_fitness,
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::problem::traits::Problem;
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
        fn better_fitness_fn(&self) -> fn(f64, f64) -> bool { crate::solution::traits::evaluator::maximizing_fitness }
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
        fn better_fitness_fn(&self) -> fn(f64, f64) -> bool { crate::solution::traits::evaluator::minimizing_fitness }
    }

    #[test]
    fn empty_population_returns_zero_statistics() {
        let population: Vec<Solution<bool>> = vec![];
        let (best, avg, worst) = calculate_population_statistics(&population, &MaxProblem).as_tuple();
        assert_eq!(best, 0.0);
        assert_eq!(avg, 0.0);
        assert_eq!(worst, 0.0);

        let stats = calculate_population_statistics(&population, &MaxProblem);
        assert_eq!(stats.best_index, None);
    }

    #[test]
    fn single_solution_population_returns_same_best_avg_worst() {
        let mut solution: Solution<bool> = Solution::new(vec![]);
        let _fitness = 10.0;
        solution.set_quality(_fitness);

        let population = vec![solution];
        let (best, avg, worst) = calculate_population_statistics(&population, &MaxProblem).as_tuple();

        assert_eq!(best, _fitness);
        assert_eq!(avg, _fitness);
        assert_eq!(worst, _fitness);
    }

    #[test]
    fn multiple_solutions_compute_expected_statistics() {
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
        let (best, avg, worst) = calculate_population_statistics(&population, &MaxProblem).as_tuple();

        assert_eq!(best, 20.0);
        assert_eq!(avg, 15.0);
        assert_eq!(worst, 10.0);
    }

    #[test]
    fn minimization_statistics_invert_best_and_worst() {
        let s1 = BinarySolutionBuilder::ones(3).with_quality(20.0).build();
        let s2 = BinarySolutionBuilder::zeros(3).with_quality(10.0).build();
        let s3 = BinarySolutionBuilder::random(3, Some(10))
            .with_quality(15.0)
            .build();

        let population = vec![s1, s2, s3];
        let (best, avg, worst) = calculate_population_statistics(&population, &MinProblem).as_tuple();

        assert_eq!(best, 10.0);
        assert_eq!(avg, 15.0);
        assert_eq!(worst, 20.0);
    }

    #[test]
    fn calculate_population_statistics_tracks_best_index() {
        let s1 = BinarySolutionBuilder::ones(3).with_quality(20.0).build();
        let s2 = BinarySolutionBuilder::zeros(3).with_quality(10.0).build();
        let s3 = BinarySolutionBuilder::random(3, Some(10))
            .with_quality(15.0)
            .build();

        let population = vec![s1, s2, s3];
        let stats = calculate_population_statistics(&population, &MinProblem);

        assert_eq!(stats.best_index, Some(1));
        assert_eq!(stats.best_fitness, 10.0);
        assert_eq!(stats.average_fitness, 15.0);
        assert_eq!(stats.worst_fitness, 20.0);
    }

    #[test]
    fn calculate_population_statistics_by_skips_missing_quality_values() {
        let mut s1: Solution<bool> = Solution::new(vec![true]);
        s1.set_quality(12.0);
        let s2: Solution<bool> = Solution::new(vec![false]);
        let mut s3: Solution<bool> = Solution::new(vec![true, false]);
        s3.set_quality(8.0);

        let population = vec![s1, s2, s3];
        let stats = calculate_population_statistics_by(&population, &MinProblem, |solution| {
            solution.try_quality_value()
        });

        assert_eq!(stats.best_index, Some(2));
        assert_eq!(stats.best_fitness, 8.0);
        assert_eq!(stats.average_fitness, 10.0);
        assert_eq!(stats.worst_fitness, 12.0);
    }
}
