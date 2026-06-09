use std::fmt::Display;

use crate::solution::Solution;
use crate::solution_set::traits::SolutionSet;

#[derive(Clone)]
pub struct VectorSolutionSet<T, Q = f64>
where
    T: Clone,
    Q: Clone,
{
    solutions: Vec<Solution<T, Q>>,
}

impl<T, Q> VectorSolutionSet<T, Q>
where
    T: Clone,
    Q: Clone,
{
    pub fn new() -> Self {
        Self {
            solutions: Vec::new(),
        }
    }

    pub fn from_vec(solutions: Vec<Solution<T, Q>>) -> Self {
        Self { solutions }
    }
}

impl<T, Q> SolutionSet<T, Q> for VectorSolutionSet<T, Q>
where
    T: Clone + Display,
    Q: Clone + Display,
{
    fn iter(&self) -> Box<dyn Iterator<Item = &Solution<T, Q>> + '_> {
        Box::new(self.solutions.iter())
    }

    fn push_solution(&mut self, solution: Solution<T, Q>) {
        self.solutions.push(solution);
    }

    fn pop_solution(&mut self) -> Option<Solution<T, Q>> {
        self.solutions.pop()
    }

    fn clear_solutions(&mut self) {
        self.solutions.clear();
    }

    fn get_solution(&self, index: usize) -> Option<&Solution<T, Q>> {
        self.solutions.get(index)
    }

    fn get_solution_mut(&mut self, index: usize) -> Option<&mut Solution<T, Q>> {
        self.solutions.get_mut(index)
    }

    fn len(&self) -> usize {
        self.solutions.len()
    }
}

#[cfg(test)]
mod test {
    use crate::problem::traits::Problem;
    use crate::solution::implementations::real_solution::RealSolutionBuilder;
    use crate::solution::implementations::string_solution::StringSolutionBuilder;
    use crate::solution::Solution;
    use crate::solution_set::implementations::vector_solution_set::VectorSolutionSet;
    use crate::solution_set::traits::SolutionSet;
    use crate::utils::random::Random;

    struct MaxProblem;

    impl<T> Problem<T, f64> for MaxProblem
    where
        T: Clone + std::fmt::Display,
        f64: Default,
    {
        fn new() -> Self {
            Self
        }

        fn evaluate(&self, _solution: &mut Solution<T, f64>) {}

        fn create_solution(&self, _rng: &mut Random) -> Solution<T, f64> {
            panic!("not needed in tests")
        }

        fn set_problem_description(&mut self, _description: String) {}

        fn get_problem_description(&self) -> String {
            "max".to_string()
        }

        fn better_fitness_fn(&self) -> fn(f64, f64) -> bool {
            crate::solution::traits::evaluator::maximizing_fitness
        }

        fn dominates(&self, solution_a: &Solution<T, f64>, solution_b: &Solution<T, f64>) -> bool {
            solution_a.quality().copied().unwrap_or(f64::NEG_INFINITY)
                > solution_b.quality().copied().unwrap_or(f64::NEG_INFINITY)
        }
    }

    struct MinProblem;

    impl<T> Problem<T, f64> for MinProblem
    where
        T: Clone + std::fmt::Display,
        f64: Default,
    {
        fn new() -> Self {
            Self
        }

        fn evaluate(&self, _solution: &mut Solution<T, f64>) {}

        fn create_solution(&self, _rng: &mut Random) -> Solution<T, f64> {
            panic!("not needed in tests")
        }

        fn set_problem_description(&mut self, _description: String) {}

        fn get_problem_description(&self) -> String {
            "min".to_string()
        }

        fn better_fitness_fn(&self) -> fn(f64, f64) -> bool {
            crate::solution::traits::evaluator::minimizing_fitness
        }

        fn dominates(&self, solution_a: &Solution<T, f64>, solution_b: &Solution<T, f64>) -> bool {
            solution_a.quality().copied().unwrap_or(f64::INFINITY)
                < solution_b.quality().copied().unwrap_or(f64::INFINITY)
        }
    }

    #[test]
    fn best_solution_supports_maximization() {
        let mut solution_set: VectorSolutionSet<f64> = VectorSolutionSet::new();

        let best_solution = RealSolutionBuilder::new(3).with_quality(10.0).build();

        let worst_solution = RealSolutionBuilder::new(3).with_quality(0.0).build();

        solution_set.add_solution(worst_solution);
        solution_set.add_solution(best_solution);

        let best = solution_set.best_solution(&MaxProblem).unwrap();
        assert_eq!(best.quality().copied(), Some(10.0));
    }

    #[test]
    fn new_set_is_empty() {
        let solution_set: VectorSolutionSet<String> = VectorSolutionSet::new();

        assert!(solution_set.is_empty());
    }

    #[test]
    fn size_tracks_added_solutions() {
        let mut solution_set: VectorSolutionSet<String> = VectorSolutionSet::new();

        let variables = vec![
            "jMetal".to_string(),
            "jMetalPy".to_string(),
            "MEALPY".to_string(),
        ];
        let solution = StringSolutionBuilder::from_variables(variables)
            .with_quality(10.0)
            .build();

        solution_set.add_solution(solution);

        assert!(!solution_set.is_empty());
        assert_eq!(solution_set.size(), 1);
        assert_eq!(solution_set.best_solution(&MaxProblem).unwrap().quality(), Some(&10.0));
    }

    #[test]
    fn best_solution_value_or_uses_default_for_empty_set() {
        let solution_set: VectorSolutionSet<f64> = VectorSolutionSet::new();
        assert_eq!(solution_set.best_solution_value_or(&MaxProblem, -1.0), -1.0);
    }

    #[test]
    fn best_solution_supports_minimization() {
        let mut solution_set: VectorSolutionSet<f64> = VectorSolutionSet::new();

        solution_set.add_solution(RealSolutionBuilder::new(2).with_quality(10.0).build());
        solution_set.add_solution(RealSolutionBuilder::new(2).with_quality(3.0).build());

        let best = solution_set.best_solution(&MinProblem).unwrap();

        assert_eq!(best.quality().copied(), Some(3.0));
        assert_eq!(solution_set.best_solution_value_or(&MinProblem, -1.0), 3.0);
    }

    #[test]
    fn remove_solution_returns_last_inserted_solution() {
        let mut solution_set: VectorSolutionSet<f64> = VectorSolutionSet::new();
        solution_set.add_solution(RealSolutionBuilder::new(2).with_quality(1.0).build());
        solution_set.add_solution(RealSolutionBuilder::new(2).with_quality(3.0).build());

        let removed = solution_set
            .remove_solution()
            .expect("Expected one removed solution");
        assert_eq!(removed.quality().copied(), Some(3.0));
        assert_eq!(solution_set.size(), 1);
    }
}
