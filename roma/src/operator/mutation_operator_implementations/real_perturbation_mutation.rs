use std::fmt::Display;

use crate::operator::traits::{MutationOperator, Operator};
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

    pub fn radius(&self) -> f64 {
        self.radius
    }

    pub fn per_variable_probability(&self) -> f64 {
        self.per_variable_probability
    }

    fn perturb_value<Q>(
        &self,
        solution: &Solution<f64, Q>,
        index: usize,
        value: f64,
        rng: &mut Random,
    ) -> f64
    where
        Q: Clone + Display,
    {
        if let Some((lower, upper)) = solution.bounds_at(index) {
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
    fn execute(&self, solution: &mut Solution<f64, Q>, probability: f64, rng: &mut Random) {
        let probability = probability.clamp(0.0, 1.0);
        if probability <= 0.0 || solution.num_variables() == 0 {
            return;
        }

        let effective_probability = (probability * self.per_variable_probability).clamp(0.0, 1.0);
        let reference = solution.clone();
        let mut changed = false;

        for index in 0..solution.num_variables() {
            if rng.next_f64() < effective_probability {
                let current = solution
                    .get_variable(index)
                    .copied()
                    .expect("index must be valid within num_variables loop");
                let mutated = self.perturb_value(&reference, index, current, rng);
                let _ = solution.set_variable(index, mutated);
                changed = true;
            }
        }

        if !changed {
            let index = rng.range(solution.num_variables() as u64) as usize;
            let current = solution
                .get_variable(index)
                .copied()
                .expect("random index must be valid within num_variables");
            let mutated = self.perturb_value(&reference, index, current, rng);
            let _ = solution.set_variable(index, mutated);
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
        let mut rng = Random::new(7);

        operator.execute(&mut solution, 1.0, &mut rng);

        assert_eq!(solution.num_variables(), 3);
        assert!(solution.variables().iter().all(|value| (-2.0..=2.0).contains(value)));
    }

    #[test]
    fn forces_at_least_one_change_when_applied() {
        let operator = RealPerturbationMutation::new(0.1, 0.0);
        let mut solution = RealSolutionBuilder::from_variables(vec![0.5, 0.5, 0.5])
            .with_bounds(0.0, 1.0)
            .build();
        let original = solution.variables().to_vec();
        let mut rng = Random::new(17);

        operator.execute(&mut solution, 1.0, &mut rng);

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

        operator.execute(&mut solution, 0.0, &mut rng);

        assert_eq!(solution.variables(), original.as_slice());
    }
}