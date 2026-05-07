use std::fmt::Display;

use crate::solution::Solution;
use crate::problem::traits::Problem;

/// Trait that defines the basic interface for sets of solutions.
/// * `T` - Type of the solution variables
/// * `Q` - Quality payload type (defaults to `f64`)
pub trait SolutionSet<T, Q = f64>
where
    T: Clone,
    Q: Clone + Display,
{
    /// Returns an iterator over all solutions in the set.
    fn iter(&self) -> Box<dyn Iterator<Item = &Solution<T, Q>> + '_>;

    /// Appends one solution to the set.
    fn push_solution(&mut self, solution: Solution<T, Q>);

    /// Removes one solution from the tail of the set semantics.
    fn pop_solution(&mut self) -> Option<Solution<T, Q>>;

    /// Removes all solutions from the set.
    fn clear_solutions(&mut self);

    /// Returns one solution by index when indexable.
    fn get_solution(&self, index: usize) -> Option<&Solution<T, Q>>;

    /// Returns one mutable solution by index when indexable.
    fn get_solution_mut(&mut self, index: usize) -> Option<&mut Solution<T, Q>>;

    /// Returns the number of solutions in the set.
    fn len(&self) -> usize;

    fn add_solution(&mut self, solution: Solution<T, Q>) {
        self.push_solution(solution);
    }

    /// Remove the last solution
    fn remove_solution(&mut self) -> Option<Solution<T, Q>> {
        self.pop_solution()
    }

    /// Remove all the solutions
    fn clear(&mut self) {
        self.clear_solutions();
    }

    /// Contains some solution
    fn is_empty(&self) -> bool {
        self.len() == 0
    }

    /// Number of solutions
    fn size(&self) -> usize {
        self.len()
    }

    fn best_solution<P>(&self, problem: &P) -> Option<&Solution<T, Q>>
    where
        Self: Sized,
        P: Problem<T, Q>,
    {
        let mut best: Option<&Solution<T, Q>> = None;

        for solution in self.iter() {
            let should_replace = match best {
                Some(current_best) => problem.dominates(solution, current_best),
                None => true,
            };

            if should_replace {
                best = Some(solution);
            }
        }

        best
    }

    fn best_solution_value<P>(&self, problem: &P) -> Option<f64>
    where
        Self: Sized,
        P: Problem<T, Q>,
        T: Display,
        Q: Copy + Into<f64>,
    {
        self.best_solution(problem)
            .and_then(|solution| solution.quality().copied().map(Into::into))
    }

    fn best_solution_value_or<P>(&self, problem: &P, default: f64) -> f64
    where
        Self: Sized,
        P: Problem<T, Q>,
        T: Display,
        Q: Copy + Into<f64>,
    {
        self.best_solution_value(problem).unwrap_or(default)
    }

    fn get(&self, index: usize) -> Option<&Solution<T, Q>> {
        self.get_solution(index)
    }

    fn get_mut(&mut self, index: usize) -> Option<&mut Solution<T, Q>> {
        self.get_solution_mut(index)
    }
}
