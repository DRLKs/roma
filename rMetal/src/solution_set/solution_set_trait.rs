use crate::solutions::solution_trait::Solution;

pub trait SolutionSet<T, S>
where 
    S: Solution<T>, T: std::clone::Clone
{
    fn add_solution(&mut self, solution: S);
    
    fn remove_solution(&mut self) -> Option<S>;

    fn clear(&mut self);
    
    fn is_empty(&self) -> bool {
        self.length() == 0
    }

    fn length(&self) -> usize;

    fn get_best_solution(&self) -> Option<&S>;

    fn get(&self, index: usize) -> Option<&S>;
    
    fn get_mut(&mut self, index: usize) -> Option<&mut S>;
}
