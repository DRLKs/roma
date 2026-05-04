use std::time::{Duration, Instant};

/// Measures execution time for a closure and returns `(elapsed, value)`.
pub fn measure<R, F>(task: F) -> (Duration, R)
where
    F: FnOnce() -> R,
{
    let start = Instant::now();
    let value = task();
    (start.elapsed(), value)
}

/// Measures execution time for a fallible closure and propagates errors.
pub fn measure_result<R, E, F>(task: F) -> Result<(Duration, R), E>
where
    F: FnOnce() -> Result<R, E>,
{
    let start = Instant::now();
    let value = task()?;
    Ok((start.elapsed(), value))
}

/// Computes speedup ratio as `base / candidate`.
///
/// Returns `+∞` when `candidate` duration is effectively zero.
pub fn speedup(base: Duration, candidate: Duration) -> f64 {
    let candidate_secs = candidate.as_secs_f64();
    if candidate_secs <= f64::EPSILON {
        return f64::INFINITY;
    }
    base.as_secs_f64() / candidate_secs
}

/// Returns process CPU time in milliseconds when the current platform exposes
/// it through a standard file interface.
///
/// This keeps the crate dependency-free. On Linux it reads `/proc/self/schedstat`,
/// whose first field is the CPU time spent on the processor in nanoseconds.
/// On unsupported platforms or parse failures it returns `None`.
pub fn process_cpu_time_ms() -> Option<f64> {
    #[cfg(target_os = "linux")]
    {
        let contents = std::fs::read_to_string("/proc/self/schedstat").ok()?;
        let cpu_time_ns = contents.split_whitespace().next()?.parse::<f64>().ok()?;
        return Some(cpu_time_ns / 1_000_000.0);
    }

    #[cfg(not(target_os = "linux"))]
    {
        None
    }
}

#[cfg(test)]
mod tests {
    use super::{measure, measure_result, process_cpu_time_ms, speedup};
    use std::time::Duration;

    #[test]
    fn measure_returns_value_and_elapsed() {
        let (elapsed, value) = measure(|| 42usize);
        assert_eq!(value, 42);
        assert!(elapsed >= Duration::ZERO);
    }

    #[test]
    fn measure_result_propagates_ok() {
        let output: Result<(Duration, i32), &str> = measure_result(|| Ok(7));
        let (elapsed, value) = output.expect("expected Ok result");
        assert_eq!(value, 7);
        assert!(elapsed >= Duration::ZERO);
    }

    #[test]
    fn measure_result_propagates_err() {
        let output: Result<(Duration, i32), &str> = measure_result(|| Err("boom"));
        assert_eq!(output.err(), Some("boom"));
    }

    #[test]
    fn speedup_handles_regular_and_zero_cases() {
        let x = speedup(Duration::from_secs(10), Duration::from_secs(5));
        assert!((x - 2.0).abs() <= 1e-12);

        let inf = speedup(Duration::from_secs(1), Duration::ZERO);
        assert!(inf.is_infinite());
    }

    #[test]
    fn process_cpu_time_is_optional_and_non_negative() {
        let value = process_cpu_time_ms();
        assert!(value.map(|x| x >= 0.0).unwrap_or(true));
    }
}
