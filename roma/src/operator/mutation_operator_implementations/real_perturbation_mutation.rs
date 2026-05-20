use std::fmt::Display;

use crate::operator::traits::{MutationOperator, Operator};
use crate::solution::RealBounds;
use crate::solution::Solution;
use crate::utils::random::Random;

/// Real-valued mutation that perturbs variables within a bounded radius.
///
/// When solution bounds are available, `radius` is interpreted as a fraction of the
/// variable span. Otherwise it is used as an absolute perturbation amplitude.
#[derive(Clone)]
pub struct RealPerturbationMutation {
    radius: f64,
    per_variable_probability: f64,
}

impl RealPerturbationMutation {
    /// Creates a mutation with a bounded perturbation radius.
    ///
    /// `radius` must be greater than zero. `per_variable_probability` is clamped
    /// by validation to the inclusive range `[0, 1]`.
    pub fn new(radius: f64, per_variable_probability: f64) -> Self {
        assert!(radius > 0.0, "radius must be > 0");
        assert!(
            (0.0..=1.0).contains(&per_variable_probability),
            "per_variable_probability must be in [0,1]"
        );

        Self {
            radius,
            per_variable_probability,
        }
    }

    /// Returns the configured perturbation radius.
    pub fn radius(&self) -> f64 {
        self.radius
    }

    /// Returns the probability applied per variable before the outer mutation probability.
    pub fn per_variable_probability(&self) -> f64 {
        self.per_variable_probability
    }

    fn perturb_value(
        &self,
        bounds: Option<&RealBounds>,
        index: usize,
        value: f64,
        rng: &mut Random,
    ) -> f64 {
        if let Some((lower, upper)) = bounds.and_then(|problem_bounds| problem_bounds.bounds_at(index)) {
            let span = upper - lower;
            if span <= f64::EPSILON {
                return lower;
            }

            let delta = (rng.next_f64() * 2.0 - 1.0) * self.radius * span;
            (value + delta).clamp(lower, upper)
        } else {
            let delta = (rng.next_f64() * 2.0 - 1.0) * self.radius;
            value + delta
        }
    }
}

impl Operator for RealPerturbationMutation {
    fn name(&self) -> &str {
        "RealPerturbationMutation"
    }
}

impl<Q> MutationOperator<f64, Q> for RealPerturbationMutation
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
        let probability = probability.clamp(0.0, 1.0);
        if probability <= 0.0 || solution.num_variables() == 0 {
            return;
        }

        let effective_probability = (probability * self.per_variable_probability).clamp(0.0, 1.0);
        let variable_count = solution.num_variables();
        let variables = solution.variables_mut();
        let mut changed = false;

        match bounds {
            None => {
                for value in variables.iter_mut() {
                    if rng.next_f64() < effective_probability {
                        let delta = (rng.next_f64() * 2.0 - 1.0) * self.radius;
                        *value += delta;
                        changed = true;
                    }
                }
            }
            Some(RealBounds::Uniform {
                lower,
                upper,
                dimensions,
            }) => {
                let lower = *lower;
                let upper = *upper;
                let dimensions = *dimensions;
                let bounded_span = upper - lower;
                for (index, value) in variables.iter_mut().enumerate() {
                    if rng.next_f64() >= effective_probability {
                        continue;
                    }

                    let mutated = if index < dimensions {
                        if bounded_span <= f64::EPSILON {
                            lower
                        } else {
                            let delta = (rng.next_f64() * 2.0 - 1.0) * self.radius * bounded_span;
                            (*value + delta).clamp(lower, upper)
                        }
                    } else {
                        let delta = (rng.next_f64() * 2.0 - 1.0) * self.radius;
                        *value + delta
                    };
                    *value = mutated;
                    changed = true;
                }
            }
            Some(RealBounds::PerVariable {
                lower_bounds,
                upper_bounds,
            }) => {
                for (index, value) in variables.iter_mut().enumerate() {
                    if rng.next_f64() >= effective_probability {
                        continue;
                    }

                    let mutated = match (lower_bounds.get(index), upper_bounds.get(index)) {
                        (Some(&lower), Some(&upper)) => {
                            let span = upper - lower;
                            if span <= f64::EPSILON {
                                lower
                            } else {
                                let delta = (rng.next_f64() * 2.0 - 1.0) * self.radius * span;
                                (*value + delta).clamp(lower, upper)
                            }
                        }
                        _ => {
                            let delta = (rng.next_f64() * 2.0 - 1.0) * self.radius;
                            *value + delta
                        }
                    };
                    *value = mutated;
                    changed = true;
                }
            }
        }

        if !changed {
            let index = rng.range(variable_count as u64) as usize;
            let current = variables[index];
            variables[index] = self.perturb_value(bounds, index, current, rng);
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::solution::RealSolutionBuilder;

    #[test]
    fn preserves_bounds_when_present() {
        let operator = RealPerturbationMutation::new(0.2, 1.0);
        let mut solution = RealSolutionBuilder::from_variables(vec![-1.0, 0.0, 1.0])
            .with_bounds(-2.0, 2.0)
            .build();
        let bounds = RealBounds::uniform(-2.0, 2.0, solution.num_variables());
        let mut rng = Random::new(7);

        operator.execute(&mut solution, 1.0, Some(&bounds), &mut rng);

        assert_eq!(solution.num_variables(), 3);
        assert!(solution.variables().iter().all(|value| (-2.0..=2.0).contains(value)));
    }

    #[test]
    fn forces_at_least_one_change_when_applied() {
        let operator = RealPerturbationMutation::new(0.1, 0.0);
        let mut solution = RealSolutionBuilder::from_variables(vec![0.5, 0.5, 0.5])
            .with_bounds(0.0, 1.0)
            .build();
        let bounds = RealBounds::uniform(0.0, 1.0, solution.num_variables());
        let original = solution.variables().to_vec();
        let mut rng = Random::new(17);

        operator.execute(&mut solution, 1.0, Some(&bounds), &mut rng);

        assert_ne!(solution.variables(), original.as_slice());
    }

    #[test]
    fn zero_probability_leaves_solution_unchanged() {
        let operator = RealPerturbationMutation::new(0.2, 1.0);
        let mut solution = RealSolutionBuilder::from_variables(vec![0.1, 0.2, 0.3])
            .with_bounds(0.0, 1.0)
            .build();
        let original = solution.variables().to_vec();
        let mut rng = Random::new(23);

        operator.execute(&mut solution, 0.0, None, &mut rng);

        assert_eq!(solution.variables(), original.as_slice());
    }
}