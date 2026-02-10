use crate::solutions::solution_trait::Solution;
use crate::problem::problem_trait::Problem;
use crate::algorithms::algorithm_trait::Algorithm;
use crate::solution_set::solution_set_trait::SolutionSet;
use crate::solution_set::implementations::vector_solution_set::VectorSolutionSet;

#[derive(Clone)]
pub struct HillClimbingParameters {
    pub max_iterations: usize
}

impl Default for HillClimbingParameters {
    fn default() -> Self {
        HillClimbingParameters {
            max_iterations: 1000,
        }
    }
}

/// Hill Climbing algorithm for single-objective optimization
pub struct HillClimbing<T, S>
where
    S: Solution<T> + Clone,
    T: Clone,
{
    parameters: HillClimbingParameters,
    solution_set: Option<VectorSolutionSet<T, S>>,
    is_maximization: bool,
}

impl<T, S> HillClimbing<T, S>
where
    S: Solution<T> + Clone,
    T: Clone,
{
    pub fn new(parameters: HillClimbingParameters, maximization: bool) -> Self {
        HillClimbing {
            parameters,
            solution_set: None,
            is_maximization: maximization,
        }
    }
}

impl<T, S, P> Algorithm<T, S, P> for HillClimbing<T, S>
where
    S: Solution<T> + Clone,
    T: Clone,
    P: Problem<S, T>,
{
    type SolutionSet = VectorSolutionSet<T, S>;
    type Parameters = HillClimbingParameters;

    fn run(&mut self, problem: &P, verbose: u8) -> Self::SolutionSet {
        if verbose > 0 {
            println!("Starting Hill Climbing with {} iterations", self.parameters.max_iterations);
        }

        let mut current = problem.create_solution();
        problem.evaluate(&mut current);

        let mut best_value = current.value();

        for iteration in 0..self.parameters.max_iterations {
            let mut neighbor = problem.neighbor(&current);
            problem.evaluate(&mut neighbor);

            // For minimization: neighbor < current
            // For maximization: neighbor > current
            if self.is_maximization && neighbor.value() > current.value() || !self.is_maximization && neighbor.value() < current.value() {
                current = neighbor;
                best_value = current.value();
                
                if verbose > 1 {
                    println!("Iteration {}: Improved to {}", iteration, best_value);
                }
            }
        }

        if verbose > 0 {
            println!("Hill Climbing finished. Best value: {}", best_value);
        }

        let mut result = VectorSolutionSet::new();
        result.add_solution(current);
        
        self.solution_set = Some(result.clone());
        result
    }

    fn validate_parameters(&self) -> bool {
        self.parameters.max_iterations > 0
    }

    fn get_solution_set(&self) -> Option<&Self::SolutionSet> {
        self.solution_set.as_ref()
    }

    fn get_parameters(&self) -> &Self::Parameters {
        &self.parameters
    }

    fn set_parameters(&mut self, params: Self::Parameters) {
        self.parameters = params;
    }
}
