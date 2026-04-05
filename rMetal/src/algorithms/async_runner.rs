use crate::algorithms::traits::Algorithm;
use crate::problem::traits::Problem;
use crate::solution::traits::Dominance;
use std::sync::Arc;
use std::thread;

/// Executes a batch of independent jobs concurrently.
///
/// Jobs are joined in submission order.
pub fn run_algorithms_async<R, J>(jobs: Vec<J>) -> Vec<R>
where
    R: Send,
    J: FnOnce() -> R + Send,
{
    if jobs.is_empty() {
        return Vec::new();
    }

    let mut results = Vec::with_capacity(jobs.len());

    thread::scope(|scope| {
        let mut handles = Vec::with_capacity(jobs.len());
        for job in jobs {
            handles.push(scope.spawn(job));
        }

        for handle in handles {
            results.push(
                handle
                    .join()
                    .expect("async worker panicked while executing job"),
            );
        }
    });

    results
}

/// Executes a batch of algorithm instances concurrently over one shared problem.
///
/// This is a convenience wrapper built on top of `run_algorithms_async`.
pub fn run_algorithm_instances_async<A, T, Q, P>(
    problem: Arc<P>,
    algorithms: Vec<A>,
) -> Vec<(A, Result<A::SolutionSet, String>)>
where
    T: Clone + Send + 'static,
    Q: Clone + Default + Dominance + Send + 'static,
    A: Algorithm<T, Q> + Send + 'static,
    A::SolutionSet: Clone + Send + 'static,
    P: Problem<T, Q> + Sync + Send + 'static,
{
    let jobs: Vec<_> = algorithms
        .into_iter()
        .map(|mut algorithm| {
            let problem_ref = Arc::clone(&problem);
            move || {
                let result = algorithm.run(problem_ref.as_ref());
                (algorithm, result)
            }
        })
        .collect();

    run_algorithms_async(jobs)
}

#[cfg(test)]
mod tests {
    use super::{run_algorithm_instances_async, run_algorithms_async};
    use crate::algorithms::implementations::hill_climbing::{
        HillClimbing,
        HillClimbingParameters,
    };
    use crate::algorithms::objective::ImprovementDirection;
    use crate::algorithms::termination::{TerminationCriteria, TerminationCriterion};
    use crate::algorithms::traits::Algorithm;
    use crate::operator::mutation_operator_implementations::bit_flip_mutation::BitFlipMutation;
    use crate::problem::traits::Problem;
    use crate::solution::Solution;
    use crate::solution_set::traits::SolutionSet;
    use crate::utils::random::Random;
    use std::sync::Arc;

    struct MinOnesProblem {
        dimensions: usize,
    }

    impl Problem<bool> for MinOnesProblem {
        fn new() -> Self {
            Self { dimensions: 8 }
        }

        fn evaluate(&self, solution: &mut Solution<bool>) {
            let ones = solution.variables().iter().filter(|&&v| v).count() as f64;
            solution.set_quality(ones);
        }

        fn create_solution(&self, rng: &mut Random) -> Solution<bool> {
            let vars: Vec<bool> = (0..self.dimensions).map(|_| rng.coin_flip()).collect();
            Solution::new(vars)
        }

        fn set_problem_description(&mut self, _description: String) {}

        fn get_problem_description(&self) -> String {
            "MinOnesProblem".to_string()
        }

        fn get_improvement_direction(&self) -> ImprovementDirection {
            ImprovementDirection::Minimize
        }
    }

    #[test]
    fn run_algorithms_async_executes_jobs() {
        let jobs = vec![|| 2usize + 2usize, || 3usize + 4usize, || 7usize + 1usize];
        let values = run_algorithms_async(jobs);
        assert_eq!(values, vec![4, 7, 8]);
    }

    #[test]
    fn run_algorithm_instances_async_executes_all_instances() {
        let problem = Arc::new(MinOnesProblem::new());

        let params_a = HillClimbingParameters::new(
            BitFlipMutation::new(),
            0.2,
            TerminationCriteria::new(vec![TerminationCriterion::MaxIterations(8)]),
        )
        .with_seed(11);

        let params_b = HillClimbingParameters::new(
            BitFlipMutation::new(),
            0.2,
            TerminationCriteria::new(vec![TerminationCriterion::MaxIterations(8)]),
        )
        .with_seed(22);

        let algorithms = vec![HillClimbing::new(params_a), HillClimbing::new(params_b)];
        let results = run_algorithm_instances_async::<
            HillClimbing<bool, BitFlipMutation>,
            bool,
            f64,
            _,
        >(Arc::clone(&problem), algorithms);

        assert_eq!(results.len(), 2);
        for (_algorithm, run_result) in results {
            let solution_set = run_result.expect("expected successful async run");
            assert_eq!(solution_set.size(), 1);
            let value = solution_set
                .best_solution_value()
                .expect("expected one valid best value");
            assert!(value.is_finite());
        }
    }
}
