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
