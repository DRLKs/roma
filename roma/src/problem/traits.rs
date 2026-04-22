use crate::algorithms::objective::ImprovementDirection;
use crate::solution::Solution;
use crate::solution::codec::SolutionCodec;
use crate::utils::random::Random;

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
/// - objective direction (maximize or minimize),
/// - optional domain-specific formatting used by observers/reports.
pub trait Problem<T, Q = f64>
where
    T: Clone,
    Q: Clone + Default,
{
    fn new() -> Self;

    /// Evaluates a solution and updates its quality/fitness
    fn evaluate(&self, solution: &mut Solution<T, Q>);

    /// Creates a new random solution for this problem that serves as a starting point for the algorithm
    fn create_solution(&self, _rng: &mut Random) -> Solution<T, Q>;

    fn set_problem_description(&mut self, description: String);

    fn get_problem_description(&self) -> String;

    /// Returns the objective improvement direction for this problem.
    ///
    /// This is the single source of truth for scalar optimization direction
    /// in the framework. Algorithms and runtime termination consume this value
    fn get_improvement_direction(&self) -> ImprovementDirection;

    /// Returns an optional codec used to encode/decode solution payloads.
    ///
    /// Default implementation keeps codec support opt-in and lightweight.
    fn solution_codec(&self) -> Option<&dyn SolutionCodec<T, Q>> {
        None
    }

    /// Returns a human-friendly representation for one solution.
    ///
    /// Observers use this string to present best snapshots in CLI/HTML outputs.
    /// Problem implementations can override this to provide domain-specific
    /// formatting (for example routes, selected items, or compact objective
    /// summaries).
    fn format_solution(&self, solution: &Solution<T, Q>) -> String {
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
