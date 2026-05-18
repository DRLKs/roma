use std::collections::HashSet;

use crate::solution::RealBounds;
use crate::operator::traits::{CrossoverOperator, Operator};
use crate::solution::Solution;
use crate::utils::random::Random;

/// Order crossover for permutation solutions.
#[derive(Clone)]
pub struct OrderCrossover {
    name: String,
    offspring_count: usize,
}

impl OrderCrossover {
    pub fn new() -> Self {
        Self {
            name: "OrderCrossover".to_string(),
            offspring_count: 2,
        }
    }

    pub fn with_one_offspring() -> Self {
        Self {
            name: "OrderCrossover".to_string(),
            offspring_count: 1,
        }
    }

    fn build_child(
        &self,
        parent1: &Solution<usize>,
        parent2: &Solution<usize>,
        start: usize,
        end: usize,
    ) -> Solution<usize> {
        let length = parent1.num_variables().min(parent2.num_variables());
        let mut child = vec![usize::MAX; length];
        let mut used = HashSet::with_capacity(length);

        for index in start..end {
            let gene = parent1
                .get_variable(index)
                .copied()
                .expect("index must be valid within crossover slice");
            child[index] = gene;
            used.insert(gene);
        }

        let mut insertion_index = end % length;
        for offset in 0..length {
            let parent_index = (end + offset) % length;
            let gene = parent2
                .get_variable(parent_index)
                .copied()
                .expect("index must be valid within crossover length");

            if used.contains(&gene) {
                continue;
            }

            child[insertion_index] = gene;
            insertion_index = (insertion_index + 1) % length;
        }

        Solution::new(child)
    }
}

impl Default for OrderCrossover {
    fn default() -> Self {
        Self::new()
    }
}

impl Operator for OrderCrossover {
    fn name(&self) -> &str {
        &self.name
    }
}

impl CrossoverOperator<usize> for OrderCrossover {
    fn execute(
        &self,
        parent1: &Solution<usize>,
        parent2: &Solution<usize>,
        _bounds: Option<&RealBounds>,
        rng: &mut Random,
    ) -> Vec<Solution<usize>> {
        let length = parent1.num_variables().min(parent2.num_variables());
        if length <= 1 {
            return vec![parent1.clone(), parent2.clone()];
        }

        let start = rng.range((length - 1) as u64) as usize;
        let end = rng.range_between((start + 1) as u64, (length + 1) as u64) as usize;

        let child1 = self.build_child(parent1, parent2, start, end);
        if self.offspring_count == 1 {
            return vec![child1];
        }

        let child2 = self.build_child(parent2, parent1, start, end);
        vec![child1, child2]
    }

    fn number_of_offspring(&self) -> usize {
        self.offspring_count
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::solution::PermutationSolutionBuilder;

    #[test]
    fn name_is_exposed() {
        let crossover = OrderCrossover::new();
        assert_eq!(crossover.name(), "OrderCrossover");
    }

    #[test]
    fn preserves_permutation_membership() {
        let crossover = OrderCrossover::new();
        let parent1 = PermutationSolutionBuilder::from_variables(vec![0, 1, 2, 3, 4, 5]).build();
        let parent2 = PermutationSolutionBuilder::from_variables(vec![2, 4, 1, 5, 3, 0]).build();
        let mut rng = Random::new(42);

        let offspring = crossover.execute(&parent1, &parent2, None, &mut rng);

        assert_eq!(offspring.len(), 2);
        for child in offspring {
            let mut genes = child.variables().to_vec();
            genes.sort_unstable();
            assert_eq!(genes, vec![0, 1, 2, 3, 4, 5]);
        }
    }
}