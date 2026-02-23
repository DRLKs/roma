use crate::operator::traits::{MutationOperator, Operator};
use crate::solutions::implementations::real_solution::RealSolution;
use crate::solutions::traits::Solution;
use crate::utils::random::{Random, seed_from_time};
use std::cell::RefCell;

/// Polynomial Mutation
///
/// A mutation operator for real-valued variables that applies
/// polynomial-based perturbation. Commonly used in multi-objective
/// evolutionary algorithms like NSGA-II.
pub struct PolynomialMutation {
    distribution_index: f64,
    rng: RefCell<Random>,
}

impl PolynomialMutation {
    /// Create a new polynomial mutation operator
    ///
    /// # Parameters
    /// - `distribution_index`: Controls the mutation spread (typical value: 20.0)
    ///   Higher values produce smaller mutations
    pub fn new(distribution_index: f64) -> Self {
        PolynomialMutation {
            distribution_index,
            rng: RefCell::new(Random::new(seed_from_time())),
        }
    }

    /// Create with default distribution index (20.0)
    pub fn new_default() -> Self {
        Self::new(20.0)
    }

    /// Calculate the delta value for mutation
    fn calculate_delta(&self, u: f64) -> f64 {
        let eta = self.distribution_index;
        if u < 0.5 {
            let val = 2.0 * u;
            val.powf(1.0 / (eta + 1.0)) - 1.0
        } else {
            let val = 2.0 * (1.0 - u);
            1.0 - val.powf(1.0 / (eta + 1.0))
        }
    }
}

impl Operator for PolynomialMutation {
    fn name(&self) -> &str {
        "Polynomial Mutation"
    }
}

impl MutationOperator<f64, RealSolution> for PolynomialMutation {
    fn execute(&self, solution: &mut RealSolution, probability: f64) {
        let variables = solution.get_solution_info().get_variables();
        let mut new_variables = Vec::with_capacity(variables.len());
        
        // Single borrow for efficiency
        let mut rng = self.rng.borrow_mut();

        for &x in variables {
            if rng.next_f64() < probability {
                let u = rng.next_f64();
                let delta = self.calculate_delta(u);
                
                // Apply mutation
                let mutated = x + delta;
                
                // Ensure mutated value is in valid range [0, 1]
                new_variables.push(mutated.clamp(0.0, 1.0));
            } else {
                // No mutation
                new_variables.push(x);
            }
        }
        
        // Drop the borrow before mutating solution
        drop(rng);

        // Update solution with mutated variables
        let info = solution.get_solution_info_mut();
        *info.get_variables_mut() = new_variables;
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::solutions::traits::SolutionInfo;

    #[test]
    fn test_polynomial_mutation_zero_probability() {
        let mutation = PolynomialMutation::new(20.0);
        let original_vars = vec![0.3, 0.5, 0.7];
        let mut solution = RealSolution::new(SolutionInfo::new(original_vars.clone()));

        mutation.execute(&mut solution, 0.0);

        let mutated_vars = solution.get_solution_info().get_variables();
        assert_eq!(mutated_vars, &original_vars);
    }

    #[test]
    fn test_polynomial_mutation_preserves_length() {
        let mutation = PolynomialMutation::new(20.0);
        let mut solution = RealSolution::new(SolutionInfo::new(vec![0.5, 0.5, 0.5, 0.5]));

        mutation.execute(&mut solution, 1.0);

        assert_eq!(solution.get_solution_info().get_variables().len(), 4);
    }

    #[test]
    fn test_polynomial_mutation_valid_range() {
        let mutation = PolynomialMutation::new(20.0);
        let mut solution = RealSolution::new(SolutionInfo::new(vec![0.0, 0.5, 1.0]));

        // Apply mutation multiple times to test boundary conditions
        for _ in 0..10 {
            mutation.execute(&mut solution, 1.0);
            for &var in solution.get_solution_info().get_variables() {
                assert!(var >= 0.0 && var <= 1.0, "Variable out of bounds: {}", var);
            }
        }
    }

    #[test]
    fn test_polynomial_mutation_high_probability_changes_values() {
        let mutation = PolynomialMutation::new(20.0);
        let original_vars = vec![0.5; 10];
        let mut solution = RealSolution::new(SolutionInfo::new(original_vars.clone()));

        mutation.execute(&mut solution, 1.0);

        let mutated_vars = solution.get_solution_info().get_variables();
        // With probability 1.0 and 10 variables, at least some should change
        let changes = mutated_vars
            .iter()
            .zip(original_vars.iter())
            .filter(|(m, o)| (*m - *o).abs() > 1e-10)
            .count();

        // Not all might change due to randomness, but expect some changes
        assert!(changes > 0);
    }

    #[test]
    fn test_polynomial_mutation_name() {
        let mutation = PolynomialMutation::new(20.0);
        assert_eq!(mutation.name(), "Polynomial Mutation");
    }
}
