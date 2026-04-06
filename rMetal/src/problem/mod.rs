pub(crate) mod traits;
pub(crate) mod implementations;

pub use traits::Problem;
pub use implementations::knapsack_problem::{KnapsackBuilder, KnapsackProblem, build_knapsack_from_records};
pub use implementations::tsp_problem::{TspProblem, build_tsp_from_records};
pub use implementations::zdt1_problem::ZDT1Problem;