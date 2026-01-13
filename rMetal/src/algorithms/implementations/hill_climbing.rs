use crate::solutions::solution_trait::Solution;
use crate::problem::problem_trait::ProblemTrait;
use std::marker::PhantomData;

pub struct HillClimbing<P, S, T>
where
    P: ProblemTrait<S, T>,
    S: Solution<T>,
    T: Clone,
{
    problem: P,
    max_iterations: usize,
    _phantom: PhantomData<(S, T)>,
}

impl<P, S, T> HillClimbing<P, S, T>
where
    P: ProblemTrait<S, T>,
    S: Solution<T>,
    T: Clone,
{
    pub fn run(&self) -> S {
        let mut current = self.problem.create_solution();
        self.problem.evaluate(&mut current);

        for _ in 0..self.max_iterations {
            let mut neighbor = self.problem.neighbor(&current);
            self.problem.evaluate(&mut neighbor);

            if neighbor.value() < current.value() {
                current = neighbor;
            }
        }

        current
    }
}
