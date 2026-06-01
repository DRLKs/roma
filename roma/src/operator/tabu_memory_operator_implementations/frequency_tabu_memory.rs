use std::collections::HashMap;
use std::fmt::Display;

use crate::operator::traits::{Operator, TabuMemoryOperator};
use crate::solution::Solution;

/// Frequency-based tabu memory that tracks how often each solution or move
/// has been visited and penalizes frequently visited regions.
///
/// This implements a long-term diversification strategy: instead of (or in
/// addition to) forbidding recently visited solutions, it assigns a
/// frequency count that algorithms can use to bias exploration toward
/// under-visited areas.
///
/// The classical tabu tenure mechanics are still available via
/// [`TabuMemoryOperator`], but the frequency information is stored in a
/// parallel structure accessible through dedicated methods.
#[derive(Clone, Debug)]
pub struct FrequencyTabuMemory {
    /// Frequency counts for visited signatures.
    frequency: HashMap<String, usize>,
    /// Weight controlling how strongly frequency penalizes revisits.
    diversification_weight: f64,
}

impl FrequencyTabuMemory {
    /// Creates a new frequency-based memory with the given diversification weight.
    ///
    /// The weight is used by `penalty()` to scale the frequency count into a
    /// fitness penalty.
    pub fn new(diversification_weight: f64) -> Self {
        assert!(
            diversification_weight >= 0.0,
            "diversification_weight must be >= 0"
        );
        Self {
            frequency: HashMap::new(),
            diversification_weight,
        }
    }

    /// Returns the visit frequency of a solution.
    pub fn visit_count<T, Q>(&self, solution: &Solution<T, Q>) -> usize
    where
        T: Clone + Display,
        Q: Clone + Display,
    {
        let sig = solution.encode();
        self.frequency.get(&sig).copied().unwrap_or(0)
    }

    /// Returns a diversification penalty for the given solution based on its
    /// visit frequency: `diversification_weight * frequency`.
    ///
    /// Algorithms can add this penalty to the fitness to discourage revisiting
    /// already-explored regions.
    pub fn penalty<T, Q>(&self, solution: &Solution<T, Q>) -> f64
    where
        T: Clone + Display,
        Q: Clone + Display,
    {
        self.diversification_weight * self.visit_count(solution) as f64
    }

    /// Increments the frequency count for the given solution signature.
    pub fn record_visit<T, Q>(&mut self, solution: &Solution<T, Q>)
    where
        T: Clone + Display,
        Q: Clone + Display,
    {
        let sig = solution.encode();
        *self.frequency.entry(sig).or_insert(0) += 1;
    }

    /// Returns the diversification weight.
    pub fn diversification_weight(&self) -> f64 {
        self.diversification_weight
    }

    /// Returns the total number of distinct signatures recorded.
    pub fn distinct_visits(&self) -> usize {
        self.frequency.len()
    }

    /// Resets all frequency counts.
    pub fn reset(&mut self) {
        self.frequency.clear();
    }
}

impl Default for FrequencyTabuMemory {
    fn default() -> Self {
        Self::new(1.0)
    }
}

impl Operator for FrequencyTabuMemory {
    fn name(&self) -> &str {
        "FrequencyTabuMemory"
    }
}

impl<T, Q> TabuMemoryOperator<T, Q> for FrequencyTabuMemory
where
    T: Clone + Display,
    Q: Clone + Display,
{
    fn signature(&self, solution: &Solution<T, Q>) -> String {
        solution.encode()
    }

    fn initialize_memory(
        &self,
        initial_solution: &Solution<T, Q>,
        iteration: usize,
        tabu_tenure: usize,
    ) -> HashMap<String, usize> {
        let mut memory = HashMap::new();
        self.remember(&mut memory, initial_solution, iteration, tabu_tenure);
        memory
    }

    fn remember(
        &self,
        tabu_memory: &mut HashMap<String, usize>,
        solution: &Solution<T, Q>,
        iteration: usize,
        tabu_tenure: usize,
    ) {
        let sig = self.signature(solution);
        tabu_memory.insert(sig, iteration + tabu_tenure);
        // NOTE: frequency tracking requires mutable self, which the trait
        // signature doesn't allow. Users should call `record_visit` explicitly
        // after each accepted move.
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::solution::Solution;

    #[test]
    fn default_weight_is_one() {
        let mem = FrequencyTabuMemory::default();
        assert!((mem.diversification_weight() - 1.0).abs() < f64::EPSILON);
    }

    #[test]
    fn records_and_counts_visits() {
        let mut mem = FrequencyTabuMemory::new(2.0);
        let sol: Solution<usize> = Solution::new(vec![0, 1, 2]);

        assert_eq!(mem.visit_count(&sol), 0);
        mem.record_visit(&sol);
        assert_eq!(mem.visit_count(&sol), 1);
        mem.record_visit(&sol);
        assert_eq!(mem.visit_count(&sol), 2);
    }

    #[test]
    fn penalty_scales_with_weight() {
        let mut mem = FrequencyTabuMemory::new(3.5);
        let sol: Solution<f64> = Solution::new(vec![1.0, 2.0]);

        mem.record_visit(&sol);
        mem.record_visit(&sol);
        mem.record_visit(&sol);

        let expected = 3.5 * 3.0;
        assert!((mem.penalty(&sol) - expected).abs() < f64::EPSILON);
    }

    #[test]
    fn different_solutions_have_independent_counts() {
        let mut mem = FrequencyTabuMemory::new(1.0);
        let sol_a: Solution<usize> = Solution::new(vec![0, 1, 2]);
        let sol_b: Solution<usize> = Solution::new(vec![2, 1, 0]);

        mem.record_visit(&sol_a);
        mem.record_visit(&sol_a);
        mem.record_visit(&sol_b);

        assert_eq!(mem.visit_count(&sol_a), 2);
        assert_eq!(mem.visit_count(&sol_b), 1);
    }

    #[test]
    fn reset_clears_frequency() {
        let mut mem = FrequencyTabuMemory::new(1.0);
        let sol: Solution<usize> = Solution::new(vec![0, 1]);

        mem.record_visit(&sol);
        assert_eq!(mem.distinct_visits(), 1);

        mem.reset();
        assert_eq!(mem.distinct_visits(), 0);
        assert_eq!(mem.visit_count(&sol), 0);
    }

    #[test]
    fn tabu_memory_trait_works() {
        let mem = FrequencyTabuMemory::new(1.0);
        let sol: Solution<usize> = Solution::new(vec![0, 1, 2]);

        let tabu = mem.initialize_memory(&sol, 0, 5);
        assert!(mem.is_tabu(&tabu, &sol, 3)); // 0 + 5 = 5 > 3
        assert!(!mem.is_tabu(&tabu, &sol, 5)); // 5 is not > 5
    }
}
