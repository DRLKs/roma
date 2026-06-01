use std::collections::HashMap;
use std::fmt::Display;

use crate::solution::{RealBounds, Solution};
use crate::utils::random::Random;

/// Base trait for all operators in the framework.
/// Operators transform solutions in some way (mutation, crossover, selection, etc.)
pub trait Operator {
    /// Returns the name of the operator for debugging/logging purposes
    fn name(&self) -> &str;
}

/// Trait for mutation operators that modify a single solution.
///
/// # Type Parameters
/// * `T` - Type of the solution variables
pub trait MutationOperator<T, Q = f64>: Operator
where
    T: Clone,
    Q: Clone,
{
    /// Applies the mutation to a solution, modifying it in place.
    ///
    /// # Arguments
    /// * `solution` - The solution to mutate
    /// * `probability` - Probability of mutation (0.0 to 1.0)
    /// * `bounds` - Optional solution-space bounds for bounded real-valued operators
    /// * `rng` - Random generator provided by the algorithm
    fn execute(
        &self,
        solution: &mut Solution<T, Q>,
        probability: f64,
        bounds: Option<&RealBounds>,
        rng: &mut Random,
    );
}

/// Trait for neighborhood operators that define the structure of the local
/// search space around a solution.
///
/// A neighborhood operator answers: "What solutions are reachable from this
/// one via a single move?" It defines the **set** of neighbors, and provides
/// methods to sample from or enumerate that set.
///
/// This is fundamentally different from [`MutationOperator`]:
/// - **Neighborhood**: defines the set of reachable solutions (the structure).
/// - **Mutation**: randomly applies a perturbation to create a new solution
///   (stochastic, probability-driven, used in population-based methods).
///
/// # Example
/// - Neighborhood "all single-swap permutations": defines $\binom{n}{2}$ neighbors.
/// - Mutation "swap two random positions with probability p": picks one and applies it.
pub trait NeighborhoodOperator<T, Q = f64>: Operator
where
    T: Clone,
    Q: Clone,
{
    /// Returns the number of neighbors reachable from `solution` via one move,
    /// if the neighborhood is finite and computable.
    ///
    /// Returns `None` for infinite or intractable neighborhoods (e.g., continuous spaces).
    fn neighborhood_size(&self, solution: &Solution<T, Q>) -> Option<usize> {
        let _ = solution;
        None
    }

    /// Samples one neighbor uniformly at random from the defined neighborhood.
    ///
    /// This is the primary method used by local-search algorithms (HC, SA, TS, VNS).
    /// The move is chosen uniformly among all valid moves in the neighborhood.
    ///
    /// # Arguments
    /// * `solution` - The current solution whose neighborhood is being explored
    /// * `bounds` - Optional real-valued bounds for continuous operators
    /// * `rng` - Random generator provided by the algorithm
    fn random_neighbor(
        &self,
        solution: &Solution<T, Q>,
        bounds: Option<&RealBounds>,
        rng: &mut Random,
    ) -> Solution<T, Q>;

    /// Enumerates all neighbors of the given solution, if feasible.
    ///
    /// Returns `None` if the neighborhood is too large or infinite.
    /// Useful for exhaustive local search or small combinatorial spaces.
    fn all_neighbors(
        &self,
        solution: &Solution<T, Q>,
        bounds: Option<&RealBounds>,
    ) -> Option<Vec<Solution<T, Q>>> {
        let _ = (solution, bounds);
        None
    }
}

/// Trait for crossover operators that combine two parent solutions.
///
/// # Type Parameters
/// * `T` - Type of the solution variables
pub trait CrossoverOperator<T, Q = f64>: Operator
where
    T: Clone,
    Q: Clone,
{
    /// Applies crossover to two parent solutions and returns offspring.
    ///
    /// # Arguments
    /// * `parent1` - First parent solution
    /// * `parent2` - Second parent solution
    /// * `bounds` - Optional solution-space bounds for bounded real-valued operators
    /// * `rng` - Random generator provided by the algorithm
    ///
    /// # Returns
    /// A vector of offspring solutions (typically 1 or 2)
    fn execute(
        &self,
        parent1: &Solution<T, Q>,
        parent2: &Solution<T, Q>,
        bounds: Option<&RealBounds>,
        rng: &mut Random,
    ) -> Vec<Solution<T, Q>>;

    /// Applies crossover to several parent solutions and returns offspring.
    ///
    /// # Arguments
    /// * `parents` - Vector of parent solutions
    /// * `rng` - Random generator provided by the algorithm
    ///
    /// # Returns
    /// A vector of offspring solutions (typically 1 or 2)
    fn execute_several(
        &self,
        parents: Vec<Solution<T, Q>>,
        bounds: Option<&RealBounds>,
        _rng: &mut Random,
    ) -> Vec<Solution<T, Q>> {
        let _ = bounds;
        let mut offspring_result = vec![];
        for i in 1..parents.len() {
            offspring_result.push(parents[i].clone());
        }
        offspring_result
    }

    /// Returns the expected number of offspring produced by this operator
    fn number_of_offspring(&self) -> usize {
        2
    }
}

/// Trait for selection operators that choose solutions from a population.
///
/// # Type Parameters
/// * `T` - Type of the solution variables
pub trait SelectionOperator<T, Q = f64>: Operator
where
    T: Clone,
    Q: Clone,
{
    /// Selects a solution from a population.
    ///
    /// # Arguments
    /// * `population` - The population to select from
    /// * `rng` - Random generator provided by the algorithm
    /// * `dominates` - Function to determine if one solution dominates another
    ///
    /// # Returns
    /// A reference to the selected solution
    fn execute<'a>(
        &self,
        population: &'a [Solution<T, Q>],
        rng: &mut Random,
        dominates: &dyn Fn(&Solution<T, Q>, &Solution<T, Q>) -> bool,
    ) -> &'a Solution<T, Q>;

    /// Selects multiple solutions from a population.
    ///
    /// # Arguments
    /// * `population` - The population to select from
    /// * `count` - Number of solutions to select
    /// * `rng` - Random generator provided by the algorithm
    ///
    /// # Returns
    /// A vector of references to selected solutions
    fn select_many<'a>(
        &self,
        population: &'a [Solution<T, Q>],
        count: usize,
        rng: &mut Random,
        dominates: fn(&Solution<T, Q>, &Solution<T, Q>) -> bool,
    ) -> Vec<&'a Solution<T, Q>> {
        (0..count)
            .map(|_| self.execute(population, rng, &dominates))
            .collect()
    }
}

/// Default tabu-memory policy based on solution signatures.
#[derive(Clone, Copy, Debug, Default)]
pub struct SolutionTabuMemory;

impl SolutionTabuMemory {
    /// Creates a default solution-signature-based tabu memory policy.
    pub const fn new() -> Self {
        Self
    }
}

impl Operator for SolutionTabuMemory {
    fn name(&self) -> &str {
        "SolutionTabuMemory"
    }
}

/// Trait for short-term memory policies used by memory-based search methods.
///
/// The default representation models a classical tabu list keyed by solution
/// signatures with iteration-based expiration.
pub trait TabuMemoryOperator<T, Q = f64>: Operator
where
    T: Clone + Display,
    Q: Clone + Display,
{
    /// Returns the memory signature used to represent a solution inside the tabu list.
    fn signature(&self, solution: &Solution<T, Q>) -> String {
        solution.encode()
    }

    /// Creates the initial tabu memory from the starting solution.
    fn initialize_memory(
        &self,
        initial_solution: &Solution<T, Q>,
        iteration: usize,
        tabu_tenure: usize,
    ) -> HashMap<String, usize> {
        let mut memory = HashMap::new();
        self.remember(&mut memory, initial_solution, iteration, tabu_tenure);
        memory
    }

    /// Removes expired entries from the tabu list.
    fn purge_expired(&self, tabu_memory: &mut HashMap<String, usize>, iteration: usize) {
        tabu_memory.retain(|_, expiry| *expiry > iteration);
    }

    /// Returns whether the candidate is currently tabu.
    fn is_tabu(
        &self,
        tabu_memory: &HashMap<String, usize>,
        candidate: &Solution<T, Q>,
        iteration: usize,
    ) -> bool {
        tabu_memory
            .get(&self.signature(candidate))
            .is_some_and(|expiry| *expiry > iteration)
    }

    /// Stores a solution in the tabu list until the configured expiration iteration.
    fn remember(
        &self,
        tabu_memory: &mut HashMap<String, usize>,
        solution: &Solution<T, Q>,
        iteration: usize,
        tabu_tenure: usize,
    ) {
        tabu_memory.insert(self.signature(solution), iteration + tabu_tenure);
    }
}

impl<T, Q> TabuMemoryOperator<T, Q> for SolutionTabuMemory
where
    T: Clone + Display,
    Q: Clone + Display,
{
}
