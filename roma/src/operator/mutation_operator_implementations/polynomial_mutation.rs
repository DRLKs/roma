use std::fmt::Display;

use crate::operator::traits::{MutationOperator, Operator};
use crate::solution::RealBounds;
use crate::solution::Solution;
use crate::utils::random::Random;

/// Polynomial Mutation
///
/// A mutation operator for real-valued variables that applies
/// polynomial-based perturbation. Commonly used in multi-objective
/// evolutionary algorithms like NSGA-II.
pub struct PolynomialMutation {
    distribution_index: f64,
}

impl PolynomialMutation {
    /// Create a new polynomial mutation operator
    ///
    /// # Parameters
    /// - `distribution_index`: Controls the mutation spread (typical value: 20.0)
    ///   Higher values produce smaller mutations
    pub fn new(distribution_index: f64) -> Self {
        PolynomialMutation { distribution_index }
    }

    /// Create with default distribution index (20.0)
    pub fn new_default() -> Self {
        Self::new(20.0)
    }

    /// Calculate the bounded polynomial mutation delta in the normalized domain.
    fn calculate_delta_q(&self, x: f64, lower: f64, upper: f64, u: f64) -> f64 {
        let eta = self.distribution_index;
        let span = upper - lower;
        if span <= f64::EPSILON {
            return 0.0;
        }

        let delta1 = (x - lower) / span;
        let delta2 = (upper - x) / span;
        let mut_pow = 1.0 / (eta + 1.0);

        if u <= 0.5 {
            let xy = 1.0 - delta1;
            let value = 2.0 * u + (1.0 - 2.0 * u) * xy.powf(eta + 1.0);
            value.powf(mut_pow) - 1.0
        } else {
            let xy = 1.0 - delta2;
            let value = 2.0 * (1.0 - u) + 2.0 * (u - 0.5) * xy.powf(eta + 1.0);
            1.0 - value.powf(mut_pow)
        }
    }

    fn execute_with_bounds<Q>(
        &self,
        solution: &mut Solution<f64, Q>,
        probability: f64,
        bounds: Option<&RealBounds>,
        rng: &mut Random,
    )
    where
        Q: Clone + Display,
    {
        match bounds {
            None => self.execute_uniform_bounds(solution, probability, 0.0, 1.0, 0, rng),
            Some(RealBounds::Uniform {
                lower,
                upper,
                dimensions,
            }) => self.execute_uniform_bounds(
                solution,
                probability,
                *lower,
                *upper,
                *dimensions,
                rng,
            ),
            Some(RealBounds::PerVariable {
                lower_bounds,
                upper_bounds,
            }) => self.execute_per_variable_bounds(
                solution,
                probability,
                lower_bounds,
                upper_bounds,
                rng,
            ),
        }
    }

    fn execute_uniform_bounds<Q>(
        &self,
        solution: &mut Solution<f64, Q>,
        probability: f64,
        lower: f64,
        upper: f64,
        bounded_dimensions: usize,
        rng: &mut Random,
    ) where
        Q: Clone + Display,
    {
        let variable_count = solution.num_variables();
        for i in 0..variable_count {
            if rng.next_f64() >= probability {
                continue;
            }

            let (effective_lower, effective_upper) = if bounded_dimensions == 0 || i < bounded_dimensions
            {
                (lower, upper)
            } else {
                (0.0, 1.0)
            };
            if effective_upper <= effective_lower {
                continue;
            }

            let u = rng.next_f64();
            let x = solution
                .get_variable(i)
                .copied()
                .expect("index must be valid within num_variables loop");
            let bounded_x = x.clamp(effective_lower, effective_upper);
            let delta_q = self.calculate_delta_q(bounded_x, effective_lower, effective_upper, u);
            let mutated = bounded_x + delta_q * (effective_upper - effective_lower);

            solution.set_variable(i, mutated.clamp(effective_lower, effective_upper));
        }
    }

    fn execute_per_variable_bounds<Q>(
        &self,
        solution: &mut Solution<f64, Q>,
        probability: f64,
        lower_bounds: &[f64],
        upper_bounds: &[f64],
        rng: &mut Random,
    ) where
        Q: Clone + Display,
    {
        let variable_count = solution.num_variables();
        for i in 0..variable_count {
            if rng.next_f64() >= probability {
                continue;
            }

            let (lower, upper) = match (lower_bounds.get(i), upper_bounds.get(i)) {
                (Some(&lower), Some(&upper)) => (lower, upper),
                _ => (0.0, 1.0),
            };
            if upper <= lower {
                continue;
            }

            let u = rng.next_f64();
            let x = solution
                .get_variable(i)
                .copied()
                .expect("index must be valid within num_variables loop");
            let bounded_x = x.clamp(lower, upper);
            let delta_q = self.calculate_delta_q(bounded_x, lower, upper, u);
            let mutated = bounded_x + delta_q * (upper - lower);

            solution.set_variable(i, mutated.clamp(lower, upper));
        }
    }
}

impl Operator for PolynomialMutation {
    fn name(&self) -> &str {
        "Polynomial Mutation"
    }
}

impl<Q> MutationOperator<f64, Q> for PolynomialMutation
where
    Q: Clone + Display,
{
    fn execute(
        &self,
        solution: &mut Solution<f64, Q>,
        probability: f64,
        bounds: Option<&RealBounds>,
        rng: &mut Random,
    ) {
        self.execute_with_bounds(solution, probability, bounds, rng);
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::solution::RealSolutionBuilder;

    #[test]
    fn test_polynomial_mutation_zero_probability() {
        let mutation = PolynomialMutation::new(20.0);
        let original_vars = vec![0.3, 0.5, 0.7];
        let mut solution = RealSolutionBuilder::from_variables(original_vars.clone()).build();
        let mut rng = Random::new(42);

        mutation.execute(&mut solution, 0.0, None, &mut rng);

        assert_eq!(solution.variables(), original_vars.as_slice());
    }

    #[test]
    fn test_polynomial_mutation_preserves_length() {
        let mutation = PolynomialMutation::new(20.0);
        let mut solution = RealSolutionBuilder::from_variables(vec![0.5, 0.5, 0.5, 0.5]).build();
        let mut rng = Random::new(42);

        mutation.execute(&mut solution, 1.0, None, &mut rng);

        assert_eq!(solution.num_variables(), 4);
    }

    #[test]
    fn test_polynomial_mutation_valid_range() {
        let mutation = PolynomialMutation::new(20.0);
        let mut solution = RealSolutionBuilder::from_variables(vec![0.0, 0.5, 1.0]).build();
        let mut rng = Random::new(42);

        // Apply mutation multiple times to test boundary conditions
        for _ in 0..10 {
            mutation.execute(&mut solution, 1.0, None, &mut rng);
            for &var in solution.variables() {
                assert!(var >= 0.0 && var <= 1.0, "Variable out of bounds: {}", var);
            }
        }
    }

    #[test]
    fn test_polynomial_mutation_respects_custom_bounds() {
        let mutation = PolynomialMutation::new(20.0);
        let mut solution = RealSolutionBuilder::from_variables(vec![-5.0, 0.0, 5.0])
            .with_bounds(-5.12, 5.12)
            .build();
        let bounds = RealBounds::uniform(-5.12, 5.12, solution.num_variables());
        let mut rng = Random::new(42);

        for _ in 0..20 {
            mutation.execute(&mut solution, 1.0, Some(&bounds), &mut rng);
            for &var in solution.variables() {
                assert!(
                    (-5.12..=5.12).contains(&var),
                    "Variable out of bounds: {}",
                    var
                );
            }
        }
    }

    #[test]
    fn test_polynomial_mutation_high_probability_changes_values() {
        let mutation = PolynomialMutation::new(20.0);
        let original_vars = vec![0.5; 10];
        let mut solution = RealSolutionBuilder::from_variables(original_vars.clone()).build();
        let mut rng = Random::new(42);

        mutation.execute(&mut solution, 1.0, None, &mut rng);

        let mutated_vars = solution.variables();
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
