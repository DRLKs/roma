use crate::solutions::traits::Solution;

/// Trait that defines the basic interface for sets of solutions.
/// * `T` - Type of the solution variables
/// * `S` - Solution type
pub trait SolutionSet<T, S>
where 
    S: Solution<T>, T: std::clone::Clone
{
    fn solutions(&self) -> &Vec<S>;
    
    fn solutions_mut(&mut self) -> &mut Vec<S>;
    
    fn add_solution(&mut self, solution: S) {
        self.solutions_mut().push(solution);
    }
    
    /// Remove the last solution
    fn remove_solution(&mut self) -> Option<S> {
        self.solutions_mut().pop()
    }

    /// Remove all the solutions
    fn clear(&mut self) {
        self.solutions_mut().clear();
    }
    
    /// 
    fn is_empty(&self) -> bool {
        self.solutions().is_empty()
    }

    fn length(&self) -> usize {
        self.solutions().len()
    }

    fn size(&self) -> usize {
        self.length()
    }

    fn get_best_solution(&self) -> Option<&S> {
        if self.is_empty() {
            return None;
        }

        let mut best_index = 0;

        for i in 0..self.length() - 1 {
            if !self.get(i).unwrap().dominates(self.get(i+1).unwrap()) {
                best_index = i + 1;
            }
        }
        
        self.get(best_index)
    }

    fn get(&self, index: usize) -> Option<&S> {
        self.solutions().get(index)
    }
    
    fn get_mut(&mut self, index: usize) -> Option<&mut S> {
        self.solutions_mut().get_mut(index)
    }
}
