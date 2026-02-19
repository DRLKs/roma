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
    use crate::solutions::implementations::permutation_solution::PermutationSolution;
    use crate::solutions::traits::{Solution, SolutionInfo};

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

        let best = solution_set.best_solution().unwrap();
        assert_eq!(
            best.get_quality().unwrap().get_fitness_indicator(),
            &Some(10.0)
        );
    }



    #[test]
    fn vector_solution_creates_empty_test() {
        let solution_set: VectorSolutionSet<String, PermutationSolution<String>> = VectorSolutionSet::new();

        assert!(solution_set.is_empty());
    }

    #[test]
    fn number_of_solutions_test() {
        let mut solution_set: VectorSolutionSet<String, PermutationSolution<String>> = VectorSolutionSet::new();

        let solution_info = SolutionInfo::new(vec!["jMetal".to_string(),"jMetalPy".to_string(), "MEALPY".to_string() ]);
        let mut solution = PermutationSolution::new(solution_info);
        let best_quality = DecimalQualityIndicator::new(Some(10.0));
        solution.set_quality(best_quality);

        solution_set.add_solution(solution);

        assert!(!solution_set.is_empty());
        assert_eq!(solution_set.solutions().len(), 1);
        assert_eq!(solution_set.best_solution().unwrap().value(), 10.0);
    }
}