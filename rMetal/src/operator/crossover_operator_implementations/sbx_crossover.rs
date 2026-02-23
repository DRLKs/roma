use crate::operator::traits::{CrossoverOperator, Operator};
use crate::solutions::implementations::real_solution::RealSolution;
use crate::solutions::traits::{Solution, SolutionInfo};
use crate::utils::random::{Random, seed_from_time};
use std::cell::RefCell;

const DEFAULT_DISTRIBUTION_INDEX: f64 = 20.0;

/// Simulated Binary Crossover (SBX)
///
/// A crossover operator for real-valued variables that simulates
/// the behavior of single-point crossover on binary strings.
/// Widely used in multi-objective optimization.
pub struct SBXCrossover {
    distribution_index: f64,
    rng: RefCell<Random>,
}

impl SBXCrossover {
    /// Create a new SBX crossover operator
    ///
    /// # Parameters
    /// - `distribution_index`: Controls the spread of offspring (typical value: 20.0)
    ///   Higher values produce offspring closer to parents
    pub fn new(distribution_index: f64) -> Self {
        SBXCrossover {
            distribution_index,
            rng: RefCell::new(Random::new(seed_from_time())),
        }
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
}

impl Operator for SBXCrossover {
    fn name(&self) -> &str {
        "SBX Crossover"
    }
}

impl CrossoverOperator<f64, RealSolution> for SBXCrossover {
    fn execute(&self, parent1: &RealSolution, parent2: &RealSolution) -> Vec<RealSolution> {
        let variables1 = parent1.get_solution_info().get_variables();
        let variables2 = parent2.get_solution_info().get_variables();

        if variables1.len() != variables2.len() {
            // If parents have different lengths, return copies
            return vec![parent1.copy(), parent2.copy()];
        }

        let mut offspring1_vars = Vec::with_capacity(variables1.len());
        let mut offspring2_vars = Vec::with_capacity(variables2.len());
        
        // Single borrow for efficiency
        let mut rng = self.rng.borrow_mut();

        for i in 0..variables1.len() {
            let x1 = variables1[i];
            let x2 = variables2[i];

            // Apply SBX with probability 0.5 per variable
            if rng.next_f64() <= 0.5 {
                let u = rng.next_f64();
                let beta = self.calculate_beta(u);

                let c1 = 0.5 * ((x1 + x2) - beta * (x2 - x1).abs());
                let c2 = 0.5 * ((x1 + x2) + beta * (x2 - x1).abs());

                // Ensure offspring are in valid range [0, 1]
                offspring1_vars.push(c1.clamp(0.0, 1.0));
                offspring2_vars.push(c2.clamp(0.0, 1.0));
            } else {
                // No crossover, copy parent values
                offspring1_vars.push(x1);
                offspring2_vars.push(x2);
            }
        }

        let offspring1 = RealSolution::new(SolutionInfo::new(offspring1_vars));
        let offspring2 = RealSolution::new(SolutionInfo::new(offspring2_vars));

        vec![offspring1, offspring2]
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_sbx_crossover_creates_two_offspring() {
        let crossover = SBXCrossover::new(20.0);
        let parent1 = RealSolution::new(SolutionInfo::new(vec![0.2, 0.5, 0.8]));
        let parent2 = RealSolution::new(SolutionInfo::new(vec![0.7, 0.3, 0.1]));

        let offspring = crossover.execute(&parent1, &parent2);

        assert_eq!(offspring.len(), 2);
        assert_eq!(
            offspring[0].get_solution_info().get_variables().len(),
            3
        );
        assert_eq!(
            offspring[1].get_solution_info().get_variables().len(),
            3
        );
    }

    #[test]
    fn test_sbx_offspring_in_valid_range() {
        let crossover = SBXCrossover::new(20.0);
        let parent1 = RealSolution::new(SolutionInfo::new(vec![0.0, 0.5, 1.0]));
        let parent2 = RealSolution::new(SolutionInfo::new(vec![1.0, 0.5, 0.0]));

        let offspring = crossover.execute(&parent1, &parent2);

        for solution in &offspring {
            for &var in solution.get_solution_info().get_variables() {
                assert!(var >= 0.0 && var <= 1.0);
            }
        }
    }

    #[test]
    fn test_sbx_different_parent_lengths() {
        let crossover = SBXCrossover::new(20.0);
        let parent1 = RealSolution::new(SolutionInfo::new(vec![0.5, 0.5]));
        let parent2 = RealSolution::new(SolutionInfo::new(vec![0.5, 0.5, 0.5]));

        let offspring = crossover.execute(&parent1, &parent2);

        assert_eq!(offspring.len(), 2);
        assert_eq!(
            offspring[0].get_solution_info().get_variables().len(),
            2
        );
        assert_eq!(
            offspring[1].get_solution_info().get_variables().len(),
            3
        );
    }

    #[test]
    fn test_sbx_name() {
        let crossover = SBXCrossover::new(20.0);
        assert_eq!(crossover.name(), "SBX Crossover");
    }
}
