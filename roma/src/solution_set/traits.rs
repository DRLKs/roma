use crate::solution::traits::Dominance;
use crate::solution::Solution;

/// Trait that defines the basic interface for sets of solutions.
/// * `T` - Type of the solution variables
/// * `Q` - Quality payload type (defaults to `f64`)
pub trait SolutionSet<T, Q = f64>
where
    T: Clone,
    Q: Clone + Dominance,
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

    /// Returns the best solution according to `Dominance`.
    ///
    /// This works for both scalar and multi-objective quality payloads because
    /// the comparison logic is delegated to `Q: Dominance`.
    fn best_solution(&self) -> Option<&Solution<T, Q>> {
        let mut best: Option<&Solution<T, Q>> = None;
        for solution in self.iter() {
            let is_better = match best {
                Some(current_best) => solution.dominates(current_best),
                None => true,
            };

            if is_better {
                best = Some(solution);
            }
        }

        best
    }

    /// Returns the scalar value of the best solution, if any.
    fn best_solution_value(&self) -> Option<f64>
    where
        Q: Copy + Into<f64>,
    {
        self.best_solution()
            .and_then(|s| s.quality().copied().map(Into::into))
    }

    /// Returns the scalar value of the best solution, or `default` when empty.
    fn best_solution_value_or(&self, default: f64) -> f64
    where
        Q: Copy + Into<f64>,
    {
        self.best_solution_value().unwrap_or(default)
    }

    fn get(&self, index: usize) -> Option<&Solution<T, Q>> {
        self.get_solution(index)
    }

    fn get_mut(&mut self, index: usize) -> Option<&mut Solution<T, Q>> {
        self.get_solution_mut(index)
    }
}
