use crate::solution_set::solution_set_trait::SolutionSet;
use crate::solutions::solution_trait::Solution;

pub struct VectorSolutionSet<T, S>
where
    S: Solution<T>,
    T: Clone
{
    solutions: Vec<S>,
    _phantom: std::marker::PhantomData<T>,
}

impl<T, S> VectorSolutionSet<T, S>
where
    S: Solution<T>,
    T: Clone
{
    pub fn new() -> Self {
        Self {
            solutions: Vec::new(),
            _phantom: std::marker::PhantomData,
        }
    }

    pub fn from_vec(solutions: Vec<S>) -> Self {
        Self {
            solutions,
            _phantom: std::marker::PhantomData,
        }
    }
}

impl<T, S> SolutionSet<T, S> for VectorSolutionSet<T, S>
where
    S: Solution<T>,
    T: Clone 
{
    fn add_solution(&mut self, solution: S) {
        self.solutions.push(solution);
    }

    fn remove_solution(&mut self) -> Option<S> {
        self.solutions.pop()
    }

    fn clear(&mut self) {
        self.solutions.clear();
    }

    fn length(&self) -> usize {
        self.solutions.len()
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
        self.solutions.get(index)
    }

    fn get_mut(&mut self, index: usize) -> Option<&mut S> {
        self.solutions.get_mut(index)
    }
}

#[cfg(test)]
mod test {
    use crate::quality_indicator::implementations::decimal_quality_indicator::DecimalQualityIndicator;
    use crate::quality_indicator::quality_indicator_trait::QualityIndicator;
    use crate::solution_set::solution_set_trait::SolutionSet;
    use crate::solution_set::implementations::vector_solution_set::VectorSolutionSet;
    use crate::solutions::implementations::binary_solution::BinarySolution;
    use crate::solutions::solution_trait::Solution;

    #[test]
    fn get_best_solution_test() {
        let mut solution_set: VectorSolutionSet<bool, BinarySolution> = VectorSolutionSet::new();

        let mut best_solution = BinarySolution::ones(3);
        let best_quality = DecimalQualityIndicator::new(Some(10.0));
        best_solution.set_quality(best_quality);

        let mut worst_solution = BinarySolution::zeros(3);
        let worst_quality = DecimalQualityIndicator::new(Some(0.0));
        worst_solution.set_quality(worst_quality);

        solution_set.add_solution(worst_solution);
        solution_set.add_solution(best_solution);

        let best = solution_set.get_best_solution().unwrap();
        assert_eq!(
            best.get_quality().unwrap().get_fitness_indicator(),
            &Some(10.0)
        );
    }
}