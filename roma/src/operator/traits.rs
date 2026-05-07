use crate::problem::traits::Problem;
use crate::solution::Solution;
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
    /// * `rng` - Random generator provided by the algorithm
    fn execute(&self, solution: &mut Solution<T, Q>, probability: f64, rng: &mut Random);
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
    /// * `rng` - Random generator provided by the algorithm
    ///
    /// # Returns
    /// A vector of offspring solutions (typically 1 or 2)
    fn execute(
        &self,
        parent1: &Solution<T, Q>,
        parent2: &Solution<T, Q>,
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
        _rng: &mut Random,
    ) -> Vec<Solution<T, Q>> {
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
    /// * `direction` - Improvement direction for scalar quality comparisons.
    ///
    /// # Returns
    /// A reference to the selected solution
    fn execute<'a>(
        &self,
        population: &'a [Solution<T, Q>],
        rng: &mut Random,
        problem: &(impl Problem<T, Q> + Sync),
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
        problem: &(impl Problem<T, Q> + Sync),
    ) -> Vec<&'a Solution<T, Q>> {
        (0..count)
            .map(|_| self.execute(population, rng, problem))
            .collect()
    }
}
