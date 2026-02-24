use crate::solution::{QualityValue, ScalarQuality, Solution};

/// Trait that defines the basic interface for sets of solutions.
/// * `T` - Type of the solution variables
pub trait SolutionSet<T, Q = ScalarQuality>
where 
    T: Clone,
    Q: Clone + QualityValue,
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
        let mut best_fitness = f64::NEG_INFINITY;
        
        for (i, sol) in self.solutions().iter().enumerate() {
            let value = sol.value();
            if value > best_fitness {
                best_fitness = value;
                best_index = i;
            }
        }
        
        self.get(best_index)
    }

    fn get(&self, index: usize) -> Option<&Solution<T, Q>> {
        self.solutions().get(index)
    }
    
    fn get_mut(&mut self, index: usize) -> Option<&mut Solution<T, Q>> {
        self.solutions_mut().get_mut(index)
    }
}