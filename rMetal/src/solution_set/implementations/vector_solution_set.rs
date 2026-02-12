use crate::solution_set::traits::SolutionSet;
use crate::solutions::traits::Solution;

#[derive(Clone)]
pub struct VectorSolutionSet<T, S>
where
    S: Solution<T> + Clone,
    T: Clone
{
    solutions: Vec<S>,
    _phantom: std::marker::PhantomData<T>,
}

impl<T, S> VectorSolutionSet<T, S>
where
    S: Solution<T> + Clone,
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
    S: Solution<T> + Clone,
    T: Clone 
{
    fn solutions(&self) -> &Vec<S> {
        &self.solutions
    }
    
    fn solutions_mut(&mut self) -> &mut Vec<S> {
        &mut self.solutions
    }
}

#[cfg(test)]
mod test {
    use crate::quality_indicator::implementations::decimal_quality_indicator::DecimalQualityIndicator;
    use crate::quality_indicator::traits::QualityIndicator;
    use crate::solution_set::traits::SolutionSet;
    use crate::solution_set::implementations::vector_solution_set::VectorSolutionSet;
    use crate::solutions::implementations::binary_solution::BinarySolution;
    use crate::solutions::traits::Solution;

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