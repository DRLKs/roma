use std::sync::atomic::{AtomicU8, Ordering};

/// Direction used to compare scalar quality values.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum ScalarDominanceDirection {
    /// Larger values dominate smaller values.
    Maximize,
    /// Smaller values dominate larger values.
    Minimize,
}

const SCALAR_DOMINANCE_MAXIMIZE: u8 = 0;
const SCALAR_DOMINANCE_MINIMIZE: u8 = 1;

static SCALAR_DOMINANCE_DIRECTION: AtomicU8 = AtomicU8::new(SCALAR_DOMINANCE_MAXIMIZE);

/// Sets the global dominance direction used by scalar quality (`f64`).
pub fn set_scalar_dominance_direction(direction: ScalarDominanceDirection) {
    let value = match direction {
        ScalarDominanceDirection::Maximize => SCALAR_DOMINANCE_MAXIMIZE,
        ScalarDominanceDirection::Minimize => SCALAR_DOMINANCE_MINIMIZE,
    };
    SCALAR_DOMINANCE_DIRECTION.store(value, Ordering::Relaxed);
}

/// Returns the current global dominance direction for scalar quality (`f64`).
pub fn scalar_dominance_direction() -> ScalarDominanceDirection {
    match SCALAR_DOMINANCE_DIRECTION.load(Ordering::Relaxed) {
        SCALAR_DOMINANCE_MINIMIZE => ScalarDominanceDirection::Minimize,
        _ => ScalarDominanceDirection::Maximize,
    }
}
