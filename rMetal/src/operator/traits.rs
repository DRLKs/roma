use crate::solutions::traits::Solution;

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
/// * `S` - Solution type
pub trait MutationOperator<T, S>: Operator
where
    S: Solution<T>,
    T: Clone,
{
    /// Applies the mutation to a solution, modifying it in place.
    /// 
    /// # Arguments
    /// * `solution` - The solution to mutate
    /// * `probability` - Probability of mutation (0.0 to 1.0)
    fn execute(&self, solution: &mut S, probability: f64);
}

/// Trait for crossover operators that combine two parent solutions.
/// 
/// # Type Parameters
/// * `T` - Type of the solution variables
/// * `S` - Solution type
pub trait CrossoverOperator<T, S>: Operator
where
    S: Solution<T>,
    T: Clone,
{
    /// Applies crossover to two parent solutions and returns offspring.
    /// 
    /// # Arguments
    /// * `parent1` - First parent solution
    /// * `parent2` - Second parent solution
    /// 
    /// # Returns
    /// A vector of offspring solutions (typically 1 or 2)
    fn execute(&self, parent1: &S, parent2: &S) -> Vec<S>;
    
    /// Returns the expected number of offspring produced by this operator
    fn number_of_offspring(&self) -> usize {
        2
    }
}

/// Trait for selection operators that choose solutions from a population.
/// 
/// # Type Parameters
/// * `T` - Type of the solution variables
/// * `S` - Solution type
pub trait SelectionOperator<T, S>: Operator
where
    S: Solution<T>,
    T: Clone,
{
    /// Selects a solution from a population.
    /// 
    /// # Arguments
    /// * `population` - The population to select from
    /// 
    /// # Returns
    /// A reference to the selected solution
    fn execute<'a>(&self, population: &'a [S]) -> &'a S;
    
    /// Selects multiple solutions from a population.
    /// 
    /// # Arguments
    /// * `population` - The population to select from
    /// * `count` - Number of solutions to select
    /// 
    /// # Returns
    /// A vector of references to selected solutions
    fn select_many<'a>(&self, population: &'a [S], count: usize) -> Vec<&'a S> {
        (0..count).map(|_| self.execute(population)).collect()
    }
}