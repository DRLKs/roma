use std::collections::HashMap;
use std::fmt::Display;

use crate::operator::traits::{Operator, TabuMemoryOperator};
use crate::solution::Solution;

/// Attribute-based tabu memory that tracks swap moves rather than full solutions.
///
/// Instead of storing the entire solution signature, this policy stores the
/// pair of positions `(i, j)` that were involved in the last move as the
/// tabu attribute. This is more suitable for permutation problems where
/// full-solution signatures are expensive and two solutions that differ
/// only by a single swap should share tabu status.
///
/// The signature format is `"swap:i:j"` where `i < j`.
#[derive(Clone, Debug, Default)]
pub struct AttributeTabuMemory;

impl AttributeTabuMemory {
    pub const fn new() -> Self {
        Self
    }

    /// Builds a tabu signature for the swap of positions `i` and `j`.
    ///
    /// Normalizes to ensure `i < j` so that `swap(2,5)` and `swap(5,2)` produce
    /// the same signature.
    pub fn swap_signature(i: usize, j: usize) -> String {
        let (lo, hi) = if i < j { (i, j) } else { (j, i) };
        format!("swap:{}:{}", lo, hi)
    }

    /// Detects which positions differ between two solutions and produces the
    /// attribute signature for the move. Falls back to full-solution encoding
    /// when the move cannot be described as a single swap.
    fn detect_move_signature<T: Clone + Display + PartialEq, Q: Clone + Display>(
        from: &Solution<T, Q>,
        to: &Solution<T, Q>,
    ) -> String {
        let from_vars = from.variables();
        let to_vars = to.variables();

        if from_vars.len() != to_vars.len() {
            return to.encode();
        }

        let diffs: Vec<usize> = from_vars
            .iter()
            .zip(to_vars.iter())
            .enumerate()
            .filter(|(_, (a, b))| a != b)
            .map(|(idx, _)| idx)
            .collect();

        if diffs.len() == 2 {
            Self::swap_signature(diffs[0], diffs[1])
        } else {
            to.encode()
        }
    }
}

impl Operator for AttributeTabuMemory {
    fn name(&self) -> &str {
        "AttributeTabuMemory"
    }
}

impl<T, Q> TabuMemoryOperator<T, Q> for AttributeTabuMemory
where
    T: Clone + Display + PartialEq,
    Q: Clone + Display,
{
    fn signature(&self, solution: &Solution<T, Q>) -> String {
        // For attribute-based memory, the default signature is the full encoding.
        // The actual attribute-based signature is computed via detect_move_signature
        // which requires both source and target. See `remember` override.
        solution.encode()
    }

    fn initialize_memory(
        &self,
        _initial_solution: &Solution<T, Q>,
        _iteration: usize,
        _tabu_tenure: usize,
    ) -> HashMap<String, usize> {
        // No initial tabu attributes since no move has been made yet.
        HashMap::new()
    }

    fn is_tabu(
        &self,
        tabu_memory: &HashMap<String, usize>,
        candidate: &Solution<T, Q>,
        iteration: usize,
    ) -> bool {
        // Check full-solution signature as well as any attribute signature
        let full_sig = candidate.encode();
        if tabu_memory
            .get(&full_sig)
            .is_some_and(|expiry| *expiry > iteration)
        {
            return true;
        }

        // If the candidate is stored by move attributes, it would already be
        // caught if the caller used `remember_move`.
        false
    }

    fn remember(
        &self,
        tabu_memory: &mut HashMap<String, usize>,
        solution: &Solution<T, Q>,
        iteration: usize,
        tabu_tenure: usize,
    ) {
        // Default remember stores the full solution signature.
        tabu_memory.insert(solution.encode(), iteration + tabu_tenure);
    }
}

impl AttributeTabuMemory {
    /// Records a move between two solutions using attribute-based signature.
    ///
    /// This should be called instead of the trait's `remember` when the caller
    /// has both the source and target solutions, enabling swap detection.
    pub fn remember_move<T, Q>(
        &self,
        tabu_memory: &mut HashMap<String, usize>,
        from: &Solution<T, Q>,
        to: &Solution<T, Q>,
        iteration: usize,
        tabu_tenure: usize,
    ) where
        T: Clone + Display + PartialEq,
        Q: Clone + Display,
    {
        let sig = Self::detect_move_signature(from, to);
        tabu_memory.insert(sig, iteration + tabu_tenure);
    }

    /// Checks whether the move from `source` to `candidate` is tabu.
    pub fn is_move_tabu<T, Q>(
        &self,
        tabu_memory: &HashMap<String, usize>,
        from: &Solution<T, Q>,
        candidate: &Solution<T, Q>,
        iteration: usize,
    ) -> bool
    where
        T: Clone + Display + PartialEq,
        Q: Clone + Display,
    {
        let sig = Self::detect_move_signature(from, candidate);
        tabu_memory
            .get(&sig)
            .is_some_and(|expiry| *expiry > iteration)
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::solution::Solution;

    #[test]
    fn swap_signature_is_normalized() {
        assert_eq!(
            AttributeTabuMemory::swap_signature(3, 1),
            AttributeTabuMemory::swap_signature(1, 3)
        );
        assert_eq!(AttributeTabuMemory::swap_signature(2, 5), "swap:2:5");
    }

    #[test]
    fn detects_swap_move() {
        let from: Solution<usize> = Solution::new(vec![0, 1, 2, 3, 4]);
        let mut to = from.clone();
        to.swap_variables(1, 3);

        let sig = AttributeTabuMemory::detect_move_signature(&from, &to);
        assert_eq!(sig, "swap:1:3");
    }

    #[test]
    fn falls_back_to_full_signature_for_non_swap() {
        let from: Solution<usize> = Solution::new(vec![0, 1, 2, 3, 4]);
        // Three positions differ — not a simple swap
        let to: Solution<usize> = Solution::new(vec![4, 3, 2, 1, 0]);

        let sig = AttributeTabuMemory::detect_move_signature(&from, &to);
        assert_eq!(sig, to.encode());
    }

    #[test]
    fn remember_move_and_check_tabu() {
        let mem = AttributeTabuMemory::new();
        let mut tabu: HashMap<String, usize> = HashMap::new();

        let from: Solution<usize> = Solution::new(vec![0, 1, 2, 3]);
        let mut to = from.clone();
        to.swap_variables(0, 2);

        mem.remember_move(&mut tabu, &from, &to, 5, 3);

        // Should be tabu at iteration 7 (5 + 3 = 8 > 7)
        assert!(mem.is_move_tabu(&tabu, &from, &to, 7));
        // Should NOT be tabu at iteration 8 (8 is not < 8)
        assert!(!mem.is_move_tabu(&tabu, &from, &to, 8));
    }

    #[test]
    fn empty_initial_memory() {
        let mem = AttributeTabuMemory::new();
        let sol: Solution<usize> = Solution::new(vec![0, 1, 2]);
        let memory = mem.initialize_memory(&sol, 0, 5);
        assert!(memory.is_empty());
    }
}
