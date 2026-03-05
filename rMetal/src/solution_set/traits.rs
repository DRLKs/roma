use crate::solution::{Solution};
use crate::solution::traits::{Dominance};

/// Trait that defines the basic interface for sets of solutions.
/// * `T` - Type of the solution variables
pub trait SolutionSet<T, Q = f64>
where 
    T: Clone,
    Q: Clone + Dominance,
{
    fn solutions(&self) -> &Vec<Solution<T, Q>>;
    
    fn solutions_mut(&mut self) -> &mut Vec<Solution<T, Q>>;
    
    fn add_solution(&mut self, solution: Solution<T, Q>) {
        self.solutions_mut().push(solution);
    }
    
    /// Remove the last solution
    fn remove_solution(&mut self) -> Option<Solution<T, Q>> {
        self.solutions_mut().pop()
    }

    /// Remove all the solutions
    fn clear(&mut self) {
        self.solutions_mut().clear();
    }
    
    /// Contains some solution
    fn is_empty(&self) -> bool {
        self.solutions().is_empty()
    }

    /// Number of solutions
    fn size(&self) -> usize {
        self.solutions().len()
    }
    
    fn best_solution(&self) -> Option<&Solution<T, Q>> {
        if self.is_empty() {
            return None;
        }

        let mut best_index = 0;
        for (i, sol) in self.solutions().iter().enumerate() {
            if sol.dominates(&self.solutions()[best_index]) {
                best_index = i;
            }
        }
        
        self.get(best_index)
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
        self.solutions().get(index)
    }
    
    fn get_mut(&mut self, index: usize) -> Option<&mut Solution<T, Q>> {
        self.solutions_mut().get_mut(index)
    }
}