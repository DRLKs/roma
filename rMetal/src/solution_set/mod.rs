pub(crate) mod traits;
pub(crate) mod implementations;

pub use traits::SolutionSet;
pub use implementations::vector_solution_set::VectorSolutionSet;
pub use implementations::deque_solution_set::DequeSolutionSet;