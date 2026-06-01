use std::fmt::Display;

use crate::operator::traits::{NeighborhoodOperator, Operator};
use crate::solution::{RealBounds, Solution};
use crate::utils::random::Random;

/// Gaussian neighborhood operator for continuous (real-valued) solutions.
///
/// Each variable is perturbed by adding a sample from a normal distribution
/// with mean 0 and standard deviation controlled by the configured `sigma`
/// scaled by `exploration_strength`.
///
/// Unlike `RealPerturbationMutation` which uses uniform perturbations, this
/// operator produces moves concentrated near the current position with
/// occasional larger steps, better matching the assumptions of many
/// continuous local-search landscapes.
#[derive(Clone, Debug)]
pub struct GaussianNeighborhood {
    /// Base standard deviation of the Gaussian perturbation.
    sigma: f64,
    /// Per-variable probability of being perturbed.
    per_variable_probability: f64,
}

impl GaussianNeighborhood {
    /// Creates a new Gaussian neighborhood operator.
    ///
    /// All variables are perturbed by default. Use [`Self::with_per_variable_probability`]
    /// to restrict perturbation to a subset of dimensions.
    ///
    /// # Arguments
    /// * `sigma` - Base standard deviation (must be > 0)
    pub fn new(sigma: f64) -> Self {
        assert!(sigma > 0.0, "sigma must be > 0");
        Self {
            sigma,
            per_variable_probability: 1.0,
        }
    }

    /// Sets the per-variable probability of being perturbed.
    ///
    /// This controls the neighborhood structure: with probability < 1.0,
    /// only a subset of dimensions are explored per step.
    pub fn with_per_variable_probability(mut self, probability: f64) -> Self {
        assert!(
            (0.0..=1.0).contains(&probability),
            "per_variable_probability must be in [0, 1]"
        );
        self.per_variable_probability = probability;
        self
    }

    /// Returns the configured base sigma.
    pub fn sigma(&self) -> f64 {
        self.sigma
    }

    /// Generates a sample from a standard normal distribution using Box-Muller.
    fn sample_normal(rng: &mut Random) -> f64 {
        let u1 = rng.next_f64().max(f64::MIN_POSITIVE);
        let u2 = rng.next_f64();
        (-2.0 * u1.ln()).sqrt() * (2.0 * std::f64::consts::PI * u2).cos()
    }
}

impl Operator for GaussianNeighborhood {
    fn name(&self) -> &str {
        "GaussianNeighborhood"
    }
}

impl<Q> NeighborhoodOperator<f64, Q> for GaussianNeighborhood
where
    Q: Clone + Display,
{
    fn neighborhood_size(&self, _solution: &Solution<f64, Q>) -> Option<usize> {
        None // Continuous neighborhood is infinite
    }

    fn random_neighbor(
        &self,
        solution: &Solution<f64, Q>,
        bounds: Option<&RealBounds>,
        rng: &mut Random,
    ) -> Solution<f64, Q> {
        let n = solution.num_variables();
        if n == 0 {
            return solution.clone();
        }

        let mut neighbor = solution.clone();
        let variables = neighbor.variables_mut();
        let mut applied = false;

        for i in 0..n {
            if rng.next_f64() < self.per_variable_probability {
                let perturbation = Self::sample_normal(rng) * self.sigma;
                let new_value = variables[i] + perturbation;

                variables[i] = if let Some(real_bounds) = bounds {
                    if let Some((lo, hi)) = real_bounds.bounds_at(i) {
                        new_value.clamp(lo, hi)
                    } else {
                        new_value
                    }
                } else {
                    new_value
                };

                applied = true;
            }
        }

        // Guarantee at least one variable is perturbed
        if !applied {
            let idx = rng.range(n as u64) as usize;
            let perturbation = Self::sample_normal(rng) * self.sigma;
            let new_value = variables[idx] + perturbation;

            variables[idx] = if let Some(real_bounds) = bounds {
                if let Some((lo, hi)) = real_bounds.bounds_at(idx) {
                    new_value.clamp(lo, hi)
                } else {
                    new_value
                }
            } else {
                new_value
            };
        }

        neighbor
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn name_is_exposed() {
        let op = GaussianNeighborhood::new(0.1);
        assert_eq!(op.name(), "GaussianNeighborhood");
    }

    #[test]
    fn preserves_variable_count() {
        let op = GaussianNeighborhood::new(0.5).with_per_variable_probability(0.8);
        let mut rng = Random::new(42);
        let solution: Solution<f64> = Solution::new(vec![1.0, 2.0, 3.0, 4.0]);

        let neighbor = op.random_neighbor(&solution, None, &mut rng);
        assert_eq!(neighbor.num_variables(), 4);
    }

    #[test]
    fn respects_bounds() {
        let op = GaussianNeighborhood::new(10.0);
        let mut rng = Random::new(7);
        let bounds = RealBounds::uniform(-1.0, 1.0, 3);
        let solution: Solution<f64> = Solution::new(vec![0.0, 0.0, 0.0]);

        for _ in 0..100 {
            let neighbor = op.random_neighbor(&solution, Some(&bounds), &mut rng);
            for v in neighbor.variables() {
                assert!(
                    (-1.0..=1.0).contains(v),
                    "value {} out of bounds [-1, 1]",
                    v
                );
            }
        }
    }

    #[test]
    fn always_modifies_at_least_one_variable() {
        let op = GaussianNeighborhood::new(0.1).with_per_variable_probability(0.01);
        let mut rng = Random::new(99);
        let source: Solution<f64> = Solution::new(vec![0.0; 10]);

        for _ in 0..50 {
            let neighbor = op.random_neighbor(&source, None, &mut rng);
            let changed = neighbor
                .variables()
                .iter()
                .zip(source.variables())
                .any(|(a, b)| (a - b).abs() > f64::EPSILON);
            assert!(changed, "at least one variable should change");
        }
    }

    #[test]
    fn neighborhood_size_is_none_for_continuous() {
        let op = GaussianNeighborhood::new(0.5);
        let solution: Solution<f64> = Solution::new(vec![1.0, 2.0, 3.0]);
        assert_eq!(op.neighborhood_size(&solution), None);
    }
}
