use std::fmt::Display;

use crate::operator::traits::{CrossoverOperator, Operator};
use crate::solution::RealBounds;
use crate::solution::Solution;
use crate::utils::random::Random;

const DEFAULT_DISTRIBUTION_INDEX: f64 = 20.0;

/// Simulated Binary Crossover (SBX)
///
/// A crossover operator for real-valued variables that simulates
/// the behavior of single-point crossover on binary strings.
/// Widely used in multi-objective optimization.
pub struct SBXCrossover {
    distribution_index: f64,
}

impl SBXCrossover {
    /// Create a new SBX crossover operator
    ///
    /// # Parameters
    /// - `distribution_index`: Controls the spread of offspring (typical value: 20.0)
    ///   Higher values produce offspring closer to parents
    pub fn new(distribution_index: f64) -> Self {
        SBXCrossover { distribution_index }
    }

    /// Create with default distribution index
    pub fn new_default() -> Self {
        Self::new(DEFAULT_DISTRIBUTION_INDEX)
    }

    /// Calculate the spread factor beta
    fn calculate_beta(&self, u: f64) -> f64 {
        let eta = self.distribution_index;
        if u <= 0.5 {
            (2.0 * u).powf(1.0 / (eta + 1.0))
        } else {
            (1.0 / (2.0 * (1.0 - u))).powf(1.0 / (eta + 1.0))
        }
    }

    fn execute_bounded<Q>(
        &self,
        parent1: &Solution<f64, Q>,
        parent2: &Solution<f64, Q>,
        bounds: Option<&RealBounds>,
        rng: &mut Random,
    ) -> Vec<Solution<f64, Q>>
    where
        Q: Clone + Display,
    {
        let variables1 = parent1.variables();
        let variables2 = parent2.variables();

        if variables1.len() != variables2.len() {
            // If parents have different lengths, return copies
            return vec![parent1.clone(), parent2.clone()];
        }

        let mut offspring1_vars = Vec::with_capacity(variables1.len());
        let mut offspring2_vars = Vec::with_capacity(variables2.len());

        match bounds {
            None => self.execute_default_unit_bounds(
                variables1,
                variables2,
                &mut offspring1_vars,
                &mut offspring2_vars,
                rng,
            ),
            Some(RealBounds::Uniform {
                lower,
                upper,
                dimensions,
            }) => self.execute_uniform_bounds(
                variables1,
                variables2,
                *lower,
                *upper,
                *dimensions,
                &mut offspring1_vars,
                &mut offspring2_vars,
                rng,
            ),
            Some(RealBounds::PerVariable {
                lower_bounds,
                upper_bounds,
            }) => self.execute_per_variable_bounds(
                variables1,
                variables2,
                lower_bounds,
                upper_bounds,
                &mut offspring1_vars,
                &mut offspring2_vars,
                rng,
            ),
        }

        let offspring1: Solution<f64, Q> = Solution::new(offspring1_vars);
        let offspring2: Solution<f64, Q> = Solution::new(offspring2_vars);

        vec![offspring1, offspring2]
    }

    fn execute_default_unit_bounds(
        &self,
        variables1: &[f64],
        variables2: &[f64],
        offspring1_vars: &mut Vec<f64>,
        offspring2_vars: &mut Vec<f64>,
        rng: &mut Random,
    ) {
        for i in 0..variables1.len() {
            let x1 = variables1[i];
            let x2 = variables2[i];

            if rng.next_f64() <= 0.5 {
                let u = rng.next_f64();
                let beta = self.calculate_beta(u);
                let c1 = 0.5 * ((x1 + x2) - beta * (x2 - x1).abs());
                let c2 = 0.5 * ((x1 + x2) + beta * (x2 - x1).abs());
                offspring1_vars.push(c1.clamp(0.0, 1.0));
                offspring2_vars.push(c2.clamp(0.0, 1.0));
            } else {
                offspring1_vars.push(x1);
                offspring2_vars.push(x2);
            }
        }
    }

    fn execute_uniform_bounds(
        &self,
        variables1: &[f64],
        variables2: &[f64],
        lower: f64,
        upper: f64,
        dimensions: usize,
        offspring1_vars: &mut Vec<f64>,
        offspring2_vars: &mut Vec<f64>,
        rng: &mut Random,
    ) {
        for i in 0..variables1.len() {
            let x1 = variables1[i];
            let x2 = variables2[i];

            if rng.next_f64() <= 0.5 {
                let u = rng.next_f64();
                let beta = self.calculate_beta(u);
                let c1 = 0.5 * ((x1 + x2) - beta * (x2 - x1).abs());
                let c2 = 0.5 * ((x1 + x2) + beta * (x2 - x1).abs());
                if i < dimensions {
                    offspring1_vars.push(c1.clamp(lower, upper));
                    offspring2_vars.push(c2.clamp(lower, upper));
                } else {
                    offspring1_vars.push(c1.clamp(0.0, 1.0));
                    offspring2_vars.push(c2.clamp(0.0, 1.0));
                }
            } else {
                offspring1_vars.push(x1);
                offspring2_vars.push(x2);
            }
        }
    }

    fn execute_per_variable_bounds(
        &self,
        variables1: &[f64],
        variables2: &[f64],
        lower_bounds: &[f64],
        upper_bounds: &[f64],
        offspring1_vars: &mut Vec<f64>,
        offspring2_vars: &mut Vec<f64>,
        rng: &mut Random,
    ) {
        for i in 0..variables1.len() {
            let x1 = variables1[i];
            let x2 = variables2[i];

            if rng.next_f64() <= 0.5 {
                let u = rng.next_f64();
                let beta = self.calculate_beta(u);
                let c1 = 0.5 * ((x1 + x2) - beta * (x2 - x1).abs());
                let c2 = 0.5 * ((x1 + x2) + beta * (x2 - x1).abs());
                match (lower_bounds.get(i), upper_bounds.get(i)) {
                    (Some(&lower), Some(&upper)) => {
                        offspring1_vars.push(c1.clamp(lower, upper));
                        offspring2_vars.push(c2.clamp(lower, upper));
                    }
                    _ => {
                        offspring1_vars.push(c1.clamp(0.0, 1.0));
                        offspring2_vars.push(c2.clamp(0.0, 1.0));
                    }
                }
            } else {
                offspring1_vars.push(x1);
                offspring2_vars.push(x2);
            }
        }
    }
}

impl Operator for SBXCrossover {
    fn name(&self) -> &str {
        "SBX Crossover"
    }
}

impl<Q> CrossoverOperator<f64, Q> for SBXCrossover
where
    Q: Clone + Display,
{
    fn execute(
        &self,
        parent1: &Solution<f64, Q>,
        parent2: &Solution<f64, Q>,
        bounds: Option<&RealBounds>,
        rng: &mut Random,
    ) -> Vec<Solution<f64, Q>> {
        self.execute_bounded(parent1, parent2, bounds, rng)
    }

    fn execute_several(
        &self,
        solutions: Vec<Solution<f64, Q>>,
        bounds: Option<&RealBounds>,
        rng: &mut Random,
    ) -> Vec<Solution<f64, Q>> {
        if solutions.len() < 2 {
            return solutions;
        }

        let mut offspring_result = Vec::new();
        let mut i = 0;

        while i + 1 < solutions.len() {
            let offspring = self.execute(&solutions[i], &solutions[i + 1], bounds, rng);
            offspring_result.extend(offspring);
            i += 2;
        }

        // Keep last parent if the number of solutions is odd.
        if i < solutions.len() {
            offspring_result.push(solutions[i].clone());
        }

        offspring_result
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::solution::RealSolutionBuilder;

    #[test]
    fn creates_two_offspring_with_matching_lengths() {
        let crossover = SBXCrossover::new(20.0);
        let parent1 = RealSolutionBuilder::from_variables(vec![0.2, 0.5, 0.8]).build();
        let parent2 = RealSolutionBuilder::from_variables(vec![0.7, 0.3, 0.1]).build();
        let mut rng = Random::new(42);

        let offspring = crossover.execute(&parent1, &parent2, None, &mut rng);

        assert_eq!(offspring.len(), 2);
        assert_eq!(offspring[0].num_variables(), 3);
        assert_eq!(offspring[1].num_variables(), 3);
    }

    #[test]
    fn offspring_stay_within_default_unit_bounds() {
        let crossover = SBXCrossover::new(20.0);
        let parent1 = RealSolutionBuilder::from_variables(vec![0.0, 0.5, 1.0]).build();
        let parent2 = RealSolutionBuilder::from_variables(vec![1.0, 0.5, 0.0]).build();
        let mut rng = Random::new(42);

        let offspring = crossover.execute(&parent1, &parent2, None, &mut rng);

        for solution in &offspring {
            for &var in solution.variables() {
                assert!(var >= 0.0 && var <= 1.0);
            }
        }
    }

    #[test]
    fn mismatched_parent_lengths_return_parent_copies() {
        let crossover = SBXCrossover::new(20.0);
        let parent1 = RealSolutionBuilder::from_variables(vec![0.5, 0.5]).build();
        let parent2 = RealSolutionBuilder::from_variables(vec![0.5, 0.5, 0.5]).build();
        let mut rng = Random::new(42);

        let offspring = crossover.execute(&parent1, &parent2, None, &mut rng);

        assert_eq!(offspring.len(), 2);
        assert_eq!(offspring[0].variables().len(), 2);
        assert_eq!(offspring[1].variables().len(), 3);
    }

    #[test]
    fn uses_problem_bounds_when_provided() {
        let crossover = SBXCrossover::new(20.0);
        let parent1 = RealSolutionBuilder::from_variables(vec![-5.12, 0.0, 5.12]).build();
        let parent2 = RealSolutionBuilder::from_variables(vec![5.12, 0.0, -5.12]).build();
        let bounds = RealBounds::uniform(-5.12, 5.12, 3);
        let mut rng = Random::new(42);

        let offspring = crossover.execute(&parent1, &parent2, Some(&bounds), &mut rng);

        for solution in &offspring {
            for &var in solution.variables() {
                assert!((-5.12..=5.12).contains(&var));
            }
        }
    }

    #[test]
    fn execute_several_returns_two_children_per_pair() {
        let crossover = SBXCrossover::new(20.0);
        let parents = vec![
            RealSolutionBuilder::from_variables(vec![0.1, 0.2]).build(),
            RealSolutionBuilder::from_variables(vec![0.8, 0.9]).build(),
            RealSolutionBuilder::from_variables(vec![0.3, 0.4]).build(),
            RealSolutionBuilder::from_variables(vec![0.6, 0.7]).build(),
        ];
        let mut rng = Random::new(42);

        let offspring = crossover.execute_several(parents, None, &mut rng);
        assert_eq!(offspring.len(), 4);
    }

    #[test]
    fn execute_several_keeps_last_parent_when_count_is_odd() {
        let crossover = SBXCrossover::new(20.0);
        let parents = vec![
            RealSolutionBuilder::from_variables(vec![0.1, 0.2]).build(),
            RealSolutionBuilder::from_variables(vec![0.8, 0.9]).build(),
            RealSolutionBuilder::from_variables(vec![0.3, 0.4]).build(),
        ];
        let mut rng = Random::new(42);

        let offspring = crossover.execute_several(parents, None, &mut rng);
        assert_eq!(offspring.len(), 3);
        assert_eq!(offspring[2].variables(), &[0.3, 0.4]);
    }

    #[test]
    fn name_is_exposed() {
        let crossover = SBXCrossover::new(20.0);
        assert_eq!(crossover.name(), "SBX Crossover");
    }

    #[test]
    fn identical_parents_produce_identical_offspring() {
        let crossover = SBXCrossover::new(20.0);
        let parent = RealSolutionBuilder::from_variables(vec![0.25, 0.5, 0.75]).build();
        let mut rng = Random::new(11);

        let offspring = crossover.execute(&parent, &parent, None, &mut rng);

        assert_eq!(offspring.len(), 2);
        assert_eq!(offspring[0].variables(), parent.variables());
        assert_eq!(offspring[1].variables(), parent.variables());
    }
}
