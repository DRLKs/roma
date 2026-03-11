use super::report::Objective;

#[inline(always)]
pub(crate) fn derive_seed(
    base_seed: u64,
    algorithm: &str,
    configuration: &str,
    problem: &str,
    run: u64,
) -> u64 {
    let mut z = base_seed
        ^ hash64(algorithm)
        ^ hash64(configuration).rotate_left(13)
        ^ hash64(problem).rotate_left(27)
        ^ run.wrapping_mul(0x9E37_79B9_7F4A_7C15);

    z = (z ^ (z >> 30)).wrapping_mul(0xBF58_476D_1CE4_E5B9);
    z = (z ^ (z >> 27)).wrapping_mul(0x94D0_49BB_1331_11EB);
    z ^ (z >> 31)
}

fn hash64(s: &str) -> u64 {
    // FNV-1a 64-bit
    let mut hash: u64 = 0xcbf2_9ce4_8422_2325;
    for b in s.as_bytes() {
        hash ^= *b as u64;
        hash = hash.wrapping_mul(0x1000_0000_01b3);
    }
    hash
}

#[inline(always)]
pub(crate) fn mean(values: &[f64]) -> f64 {
    if values.is_empty() {
        return 0.0;
    }

    values.iter().sum::<f64>() / values.len() as f64
}

#[inline(always)]
pub(crate) fn variance(values: &[f64], mean: f64) -> f64 {
    if values.is_empty() {
        return 0.0;
    }

    values
        .iter()
        .map(|v| {
            let d = *v - mean;
            d * d
        })
        .sum::<f64>()
        / values.len() as f64
}

/// Extrae el mejor y peor valor desde una lista ordenada según el objetivo.
#[inline(always)]
pub(crate) fn best_and_worst(sorted_values: &[f64], objective: Objective) -> (f64, f64) {
    debug_assert!(!sorted_values.is_empty());

    match objective {
        Objective::Maximize => (
            *sorted_values.last().unwrap_or(&0.0),
            *sorted_values.first().unwrap_or(&0.0),
        ),
        Objective::Minimize => (
            *sorted_values.first().unwrap_or(&0.0),
            *sorted_values.last().unwrap_or(&0.0),
        ),
    }
}