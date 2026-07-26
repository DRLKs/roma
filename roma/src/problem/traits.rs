use std::cmp::Ordering;
use std::fmt::Display;

use crate::solution::{RealBounds, Solution};
use crate::utils::random::Random;

/// Result of comparing two candidates under a problem's domain rules.
///
/// `Incomparable` is distinct from `Equivalent`: Pareto candidates can be
/// mutually non-dominating without being equal.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum SolutionComparison {
    Better,
    Worse,
    Equivalent,
    Incomparable,
}

impl SolutionComparison {
    /// Converts the domain result to an ordering for scalar consumers.
    ///
    /// A total sort must not invent a preference for incomparable candidates,
    /// so both `Equivalent` and `Incomparable` become `Ordering::Equal`.
    pub fn scalar_ordering(self) -> Ordering {
        match self {
            Self::Better => Ordering::Less,
            Self::Worse => Ordering::Greater,
            Self::Equivalent | Self::Incomparable => Ordering::Equal,
        }
    }

    pub fn is_better(self) -> bool {
        matches!(self, Self::Better)
    }
}

/// Applies a scalar preference rule while handling unevaluated candidates
/// consistently across scalar problems.
pub fn compare_scalar_qualities(
    left: Option<&f64>,
    right: Option<&f64>,
    prefers: impl Fn(f64, f64) -> bool,
) -> SolutionComparison {
    match (left, right) {
        (Some(left), Some(right)) if prefers(*left, *right) => SolutionComparison::Better,
        (Some(left), Some(right)) if prefers(*right, *left) => SolutionComparison::Worse,
        (Some(_), Some(_)) | (None, None) => SolutionComparison::Equivalent,
        (Some(_), None) => SolutionComparison::Better,
        (None, Some(_)) => SolutionComparison::Worse,
    }
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

    /// The sole problem-owned comparison rule.
    ///
    /// Legacy implementations that override `dominates` continue to compile,
    /// but new problems should override this method instead.
    fn compare_qualities(&self, _left: Option<&Q>, _right: Option<&Q>) -> SolutionComparison {
        SolutionComparison::Incomparable
    }

    fn compare_solutions(
        &self,
        left: &Solution<T, Q>,
        right: &Solution<T, Q>,
    ) -> SolutionComparison {
        self.compare_qualities(left.quality(), right.quality())
    }

    fn dominates(&self, solution_a: &Solution<T, Q>, solution_b: &Solution<T, Q>) -> bool {
        self.compare_solutions(solution_a, solution_b).is_better()
    }

    /// Runtime adapter for termination snapshots.
    ///
    /// Implementations must express the same scalar preference as
    /// `compare_qualities`; this remains a function pointer because the
    /// observer runtime stores it independently of the problem instance.
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

    /// Returns optional bounds metadata for real-valued solutions.
    ///
    /// The bounds type belongs to the solution module, but problems can expose
    /// a shared view of that metadata so algorithms and operators can enforce
    /// domain constraints without storing bounds inside each solution.
    fn real_bounds(&self) -> Option<&RealBounds> {
        None
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
