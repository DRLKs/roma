use std::fmt::Display;

use crate::solution::Solution;
use crate::utils::random::Random;

pub fn maximizing_fitness(candidate: f64, reference: f64) -> bool {
    candidate > reference
}

pub fn minimizing_fitness(candidate: f64, reference: f64) -> bool {
    candidate < reference
}

/// Trait that defines the basic interface for optimization problems.
///
/// # Type Parameters
/// - `T`: decision variable type.
/// - `Q`: quality payload type (defaults to `f64`).
///
/// # Responsibilities
/// Implementors provide:
/// - random solution creation,
/// - evaluation of candidate solutions,
/// - problem-owned comparison semantics for ranking solutions,
/// - optional domain-specific formatting used by observers/reports.
pub trait Problem<T, Q = f64>
where
    T: Clone,
    Q: Clone,
{
    fn new() -> Self
    where
        Self: Sized;

    /// Evaluates a solution and updates its quality/fitness
    fn evaluate(&self, solution: &mut Solution<T, Q>);

    /// Creates a new random solution for this problem that serves as a starting point for the algorithm
    fn create_solution(&self, _rng: &mut Random) -> Solution<T, Q>;

    fn set_problem_description(&mut self, description: String);

    fn get_problem_description(&self) -> String;

    fn dominates(&self, solution_a: &Solution<T, Q>, solution_b: &Solution<T, Q>) -> bool;

    fn better_fitness_fn(&self) -> fn(f64, f64) -> bool;

    fn is_better_fitness(&self, candidate: f64, reference: f64) -> bool {
        (self.better_fitness_fn())(candidate, reference)
    }

    fn non_improving_fitness_loss(&self, current: f64, candidate: f64) -> f64 {
        if self.is_better_fitness(candidate, current) {
            0.0
        } else {
            (candidate - current).abs()
        }
    }

    fn get_problem_parameters_payload(&self) -> String {
        String::new()
    }

    /// Returns a human-friendly representation for one solution.
    ///
    /// Observers use this string to present best snapshots in CLI/HTML outputs.
    /// Problem implementations can override this to provide domain-specific
    /// formatting (for example routes, selected items, or compact objective
    /// summaries).
    fn format_solution(&self, solution: &Solution<T, Q>) -> String
    where
        T: Display,
        Q: Display,
    {
        let quality_state = if solution.has_quality() {
            "evaluated"
        } else {
            "not evaluated"
        };

        format!(
            "variables={}, quality={}",
            solution.num_variables(),
            quality_state
        )
    }
}
