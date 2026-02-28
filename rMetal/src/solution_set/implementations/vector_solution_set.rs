use crate::solution::Solution;
use crate::solution::traits::{QualityValue, ScalarQuality};
use crate::solution_set::traits::SolutionSet;

#[derive(Clone)]
pub struct VectorSolutionSet<T, Q = ScalarQuality>
where
    T: Clone,
    Q: Clone,
{
    solutions: Vec<Solution<T, Q>>,
}

impl<T, Q> VectorSolutionSet<T, Q>
where
    T: Clone,
    Q: Clone,
{
    pub fn new() -> Self {
        Self {
            solutions: Vec::new(),
        }
    }

    pub fn from_vec(solutions: Vec<Solution<T, Q>>) -> Self {
        Self {
            solutions,
        }
    }
}

impl<T, Q> SolutionSet<T, Q> for VectorSolutionSet<T, Q>
where
    T: Clone,
    Q: Clone + QualityValue,
{
    fn solutions(&self) -> &Vec<Solution<T, Q>> {
        &self.solutions
    }
    
    fn solutions_mut(&mut self) -> &mut Vec<Solution<T, Q>> {
        &mut self.solutions
    }
}


#[cfg(test)]
mod test {
    use crate::solution::implementations::real_solution::RealSolutionBuilder;
    use crate::solution::implementations::string_solution::StringSolutionBuilder;
    use crate::solution_set::traits::SolutionSet;
    use crate::solution_set::implementations::vector_solution_set::VectorSolutionSet;

    #[test]
    fn get_best_solution_test() {
        let mut solution_set: VectorSolutionSet<f64> = VectorSolutionSet::new();

        let best_solution = RealSolutionBuilder::new(3).with_quality(10.0).build();

        let worst_solution = RealSolutionBuilder::new(3).with_quality(0.0).build();

        solution_set.add_solution(worst_solution);
        solution_set.add_solution(best_solution);

        let best = solution_set.best_solution().unwrap();
        assert_eq!(best.quality().copied(), Some(10.0));
    }



    #[test]
    fn vector_solution_creates_empty_test() {
        let solution_set: VectorSolutionSet<String> = VectorSolutionSet::new();

        assert!(solution_set.is_empty());
    }

    #[test]
    fn number_of_solutions_test() {
        let mut solution_set: VectorSolutionSet<String> = VectorSolutionSet::new();

        let variables = vec!["jMetal".to_string(),"jMetalPy".to_string(), "MEALPY".to_string() ];
        let solution = StringSolutionBuilder::from_variables(variables).with_quality(10.0).build();

        solution_set.add_solution(solution);

        assert!(!solution_set.is_empty());
        assert_eq!(solution_set.solutions().len(), 1);
        assert_eq!(solution_set.best_solution().unwrap().quality_value(), 10.0);
    }
}