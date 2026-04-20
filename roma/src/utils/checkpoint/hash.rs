/// Deterministic 64-bit FNV-1a hash used to build checkpoint signatures.
pub(crate) fn stable_hash64(text: &str) -> u64 {
    let mut hash: u64 = 0xcbf29ce484222325;
    for byte in text.as_bytes() {
        hash ^= u64::from(*byte);
        hash = hash.wrapping_mul(0x100000001b3);
    }
    hash
}

/// Computes algorithm/problem signature hashes from their names and parameters.
pub(crate) fn checkpoint_signature_hashes(
    algorithm_name: &str,
    algorithm_parameters: &str,
    problem_name: &str,
    problem_parameters: &str,
) -> (u64, u64) {
    let algorithm_signature_hash =
        stable_hash64(&format!("{}|{}", algorithm_name, algorithm_parameters));
    let problem_signature_hash = stable_hash64(&format!("{}|{}", problem_name, problem_parameters));
    (algorithm_signature_hash, problem_signature_hash)
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn stable_hash64_is_deterministic_for_same_input() {
        let input = "HillClimbing|mutation_probability=0.20";
        let first = stable_hash64(input);
        let second = stable_hash64(input);
        let third = stable_hash64(input);

        assert_eq!(first, second);
        assert_eq!(second, third);
    }

    #[test]
    fn stable_hash64_changes_for_different_input() {
        let a = stable_hash64("A");
        let b = stable_hash64("B");

        assert_ne!(a, b);
    }

    #[test]
    fn checkpoint_signature_hashes_are_deterministic() {
        let args = (
            "HillClimbing",
            "mutation_probability=0.20;termination=max_iterations:100",
            "DemoProblem",
            "items=52;seed=42",
        );

        let first = checkpoint_signature_hashes(args.0, args.1, args.2, args.3);
        let second = checkpoint_signature_hashes(args.0, args.1, args.2, args.3);

        assert_eq!(first, second);
    }
}
