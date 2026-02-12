use crate::solutions::traits::Solution;
use crate::problem::traits::Problem;
use crate::algorithms::traits::Algorithm;
use crate::solution_set::traits::SolutionSet;
use crate::solution_set::implementations::vector_solution_set::VectorSolutionSet;
use crate::operator::traits::MutationOperator;

/// Parameters for Hill Climbing algorithm.
/// Uses generics to allow any mutation operator.
pub struct HillClimbingParameters<T, S, M>
where
    S: Solution<T> + Clone,
    T: Clone,
    M: MutationOperator<T, S>,
{
    pub max_iterations: usize,
    pub mutation_operator: M,
    pub mutation_probability: f64,
    _phantom: std::marker::PhantomData<(T, S)>,
}

impl<T, S, M> HillClimbingParameters<T, S, M>
where
    S: Solution<T> + Clone,
    T: Clone,
    M: MutationOperator<T, S>,
{
    pub fn new(max_iterations: usize, mutation_operator: M, mutation_probability: f64) -> Self {
        HillClimbingParameters {
            max_iterations,
            mutation_operator,
            mutation_probability,
            _phantom: std::marker::PhantomData,
        }
    }
}

/// Hill Climbing algorithm for single-objective optimization.
/// Now uses a configurable mutation operator to generate neighbors.
pub struct HillClimbing<T, S, M>
where
    S: Solution<T> + Clone,
    T: Clone,
    M: MutationOperator<T, S>,
{
    parameters: HillClimbingParameters<T, S, M>,
    solution_set: Option<VectorSolutionSet<T, S>>,
    is_maximization: bool,
}

impl<T, S, M> HillClimbing<T, S, M>
where
    S: Solution<T> + Clone,
    T: Clone,
    M: MutationOperator<T, S>,
{
    pub fn new(parameters: HillClimbingParameters<T, S, M>, maximization: bool) -> Self {
        HillClimbing {
            parameters,
            solution_set: None,
            is_maximization: maximization,
        }
    }
}

impl<T, S, P, M> Algorithm<T, S, P> for HillClimbing<T, S, M>
where
    S: Solution<T> + Clone,
    T: Clone,
    P: Problem<S, T>,
    M: MutationOperator<T, S>,
{
    type SolutionSet = VectorSolutionSet<T, S>;
    type Parameters = HillClimbingParameters<T, S, M>;

    fn run(&mut self, problem: &P, verbose: u8) -> Self::SolutionSet {
        if verbose > 0 {
            println!("Starting Hill Climbing with {} iterations", self.parameters.max_iterations);
            println!("  Mutation operator: {}", self.parameters.mutation_operator.name());
        }

        let mut current = problem.create_solution();
        problem.evaluate(&mut current);

        let mut best_value = current.value();

        for iteration in 0..self.parameters.max_iterations {
            // Generate neighbor using mutation operator
            let mut neighbor = current.copy();
            self.parameters.mutation_operator.execute(&mut neighbor, self.parameters.mutation_probability);
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
