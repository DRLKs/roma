use super::report::Objective;

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