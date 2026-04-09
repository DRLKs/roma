#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum ImprovementDirection {
    Maximize,
    Minimize,
}

/// Returns `true` when `candidate` is better than `reference` according to
/// optimization `direction`.
pub fn is_better(candidate: f64, reference: f64, direction: ImprovementDirection) -> bool {
    match direction {
        ImprovementDirection::Maximize => candidate > reference,
        ImprovementDirection::Minimize => candidate < reference,
    }
}

/// Returns a non-negative loss value when moving from `current` to
/// `candidate` for the given `direction`.
///
/// - `0.0` means candidate is not worse than current.
/// - Positive values indicate how much worse the candidate is.
pub fn non_improving_loss(current: f64, candidate: f64, direction: ImprovementDirection) -> f64 {
    match direction {
        ImprovementDirection::Maximize => (current - candidate).max(0.0),
        ImprovementDirection::Minimize => (candidate - current).max(0.0),
    }
}

/// Computes scalar best and worst values according to optimization
/// `direction`.
pub fn best_worst(values: &[f64], direction: ImprovementDirection) -> (f64, f64) {
    if values.is_empty() {
        return (0.0, 0.0);
    }

    match direction {
        ImprovementDirection::Maximize => (
            values.iter().copied().fold(f64::NEG_INFINITY, f64::max),
            values.iter().copied().fold(f64::INFINITY, f64::min),
        ),
        ImprovementDirection::Minimize => (
            values.iter().copied().fold(f64::INFINITY, f64::min),
            values.iter().copied().fold(f64::NEG_INFINITY, f64::max),
        ),
    }
}
