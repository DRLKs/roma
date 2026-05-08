use std::collections::VecDeque;
use std::fmt::Display;

use crate::solution::Solution;
use crate::solution_set::traits::SolutionSet;

/// `SolutionSet` implementation backed by `VecDeque`.
///
/// Useful when frequent pushes and pops from the front/back are expected.
#[derive(Clone)]
pub struct DequeSolutionSet<T, Q = f64>
where
    T: Clone,
    Q: Clone,
{
    solutions: VecDeque<Solution<T, Q>>,
}

impl<T, Q> DequeSolutionSet<T, Q>
where
    T: Clone,
    Q: Clone,
{
    pub fn new() -> Self {
        Self {
            solutions: VecDeque::new(),
        }
    }

    pub fn from_deque(solutions: VecDeque<Solution<T, Q>>) -> Self {
        Self { solutions }
    }
}

impl<T, Q> SolutionSet<T, Q> for DequeSolutionSet<T, Q>
where
    T: Clone + Display,
    Q: Clone + Display,
{
    fn iter(&self) -> Box<dyn Iterator<Item = &Solution<T, Q>> + '_> {
        Box::new(self.solutions.iter())
    }

    fn push_solution(&mut self, solution: Solution<T, Q>) {
        self.solutions.push_back(solution);
    }

    fn pop_solution(&mut self) -> Option<Solution<T, Q>> {
        self.solutions.pop_back()
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
mod tests {
    use crate::problem::traits::Problem;
    use crate::solution::implementations::real_solution::RealSolutionBuilder;
    use crate::solution::Solution;
    use crate::solution_set::implementations::deque_solution_set::DequeSolutionSet;
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

    #[test]
    fn deque_solution_set_selects_best_solution() {
        let mut set: DequeSolutionSet<f64> = DequeSolutionSet::new();
        set.add_solution(RealSolutionBuilder::new(2).with_quality(1.0).build());
        set.add_solution(RealSolutionBuilder::new(2).with_quality(5.0).build());

        let best = set.best_solution(&MaxProblem).expect("expected best solution");
        assert_eq!(best.quality().copied(), Some(5.0));
    }

    #[test]
    fn deque_solution_set_supports_push_pop_and_clear() {
        let mut set: DequeSolutionSet<f64> = DequeSolutionSet::new();
        set.add_solution(RealSolutionBuilder::new(2).with_quality(2.0).build());
        set.add_solution(RealSolutionBuilder::new(2).with_quality(3.0).build());

        assert_eq!(set.size(), 2);
        let popped = set.remove_solution().expect("expected pop result");
        assert_eq!(popped.quality().copied(), Some(3.0));
        assert_eq!(set.size(), 1);

        set.clear();
        assert!(set.is_empty());
    }
}
