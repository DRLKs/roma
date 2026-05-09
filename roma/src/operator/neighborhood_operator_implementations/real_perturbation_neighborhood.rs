use std::fmt::Display;

use crate::operator::traits::{NeighborhoodOperator, Operator};
use crate::solution::Solution;
use crate::utils::random::Random;

/// Real-valued neighborhood that perturbs variables within a bounded radius.
///
/// When solution bounds are available, `radius` is interpreted as a fraction of the
/// variable span. Otherwise it is used as an absolute perturbation amplitude.
#[derive(Clone)]
pub struct RealPerturbationNeighborhood {
    radius: f64,
    per_variable_probability: f64,
}

impl RealPerturbationNeighborhood {
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

impl Operator for RealPerturbationNeighborhood {
    fn name(&self) -> &str {
        "RealPerturbationNeighborhood"
    }
}

impl<Q> NeighborhoodOperator<f64, Q> for RealPerturbationNeighborhood
where
    Q: Clone + Display,
{
    fn generate_neighbor(&self, solution: &Solution<f64, Q>, rng: &mut Random) -> Solution<f64, Q> {
        let mut neighbor = solution.copy();
        if neighbor.num_variables() == 0 {
            return neighbor;
        }

        let mut changed = false;
        for index in 0..neighbor.num_variables() {
            if rng.next_f64() < self.per_variable_probability {
                let current = neighbor
                    .get_variable(index)
                    .copied()
                    .expect("index must be valid within num_variables loop");
                let mutated = self.perturb_value(solution, index, current, rng);
                let _ = neighbor.set_variable(index, mutated);
                changed = true;
            }
        }

        if !changed {
            let index = rng.range(neighbor.num_variables() as u64) as usize;
            let current = neighbor
                .get_variable(index)
                .copied()
                .expect("random index must be valid within num_variables");
            let mutated = self.perturb_value(solution, index, current, rng);
            let _ = neighbor.set_variable(index, mutated);
        }

        neighbor
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::solution::RealSolutionBuilder;

    #[test]
    fn preserves_bounds_when_present() {
        let operator = RealPerturbationNeighborhood::new(0.2, 1.0);
        let solution = RealSolutionBuilder::from_variables(vec![-1.0, 0.0, 1.0])
            .with_bounds(-2.0, 2.0)
            .build();
        let mut rng = Random::new(7);

        let neighbor = operator.generate_neighbor(&solution, &mut rng);

        assert_eq!(neighbor.num_variables(), 3);
        assert!(neighbor.variables().iter().all(|value| (-2.0..=2.0).contains(value)));
    }

    #[test]
    fn forces_at_least_one_change() {
        let operator = RealPerturbationNeighborhood::new(0.1, 0.0);
        let solution = RealSolutionBuilder::from_variables(vec![0.5, 0.5, 0.5])
            .with_bounds(0.0, 1.0)
            .build();
        let mut rng = Random::new(17);

        let neighbor = operator.generate_neighbor(&solution, &mut rng);

        assert_ne!(neighbor.variables(), solution.variables());
    }
}