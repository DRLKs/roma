use std::sync::atomic::{AtomicU64, Ordering};
use std::time::{SystemTime, UNIX_EPOCH};

static SEED_COUNTER: AtomicU64 = AtomicU64::new(0xA0761D6478BD642F);

/// A small non-cryptographic 64-bit mixer based on SplitMix-like transforms.
/// Kept private so the public API of `Random` is unchanged while we reuse
/// the same mixing routine for seeding and output generation.
#[inline]
fn mix64(mut z: u64) -> u64 {
    z = (z ^ (z >> 30)).wrapping_mul(0xBF58476D1CE4E5B9);
    z = (z ^ (z >> 27)).wrapping_mul(0x94D049BB133111EB);
    z ^ (z >> 31)
}

/// Generates a time-based seed and folds in cheap per-process variability.
///
/// A monotonic atomic counter is mixed in so back-to-back calls still produce
/// distinct seeds even when the system clock resolution is coarse.
pub fn seed_from_time() -> u64 {
    // Gather several cheap, system-dependent sources of variability and
    // fold them into a single 64-bit value before mixing.
    let now = SystemTime::now()
        .duration_since(UNIX_EPOCH)
        .unwrap()
        .as_nanos(); // u128

    let now_lo = now as u64;
    let now_hi = (now >> 64) as u64;
    let pid = std::process::id() as u64;
    let counter = SEED_COUNTER.fetch_add(0x9E3779B97F4A7C15, Ordering::Relaxed);
    // address of a local value gives additional low-cost entropy between
    // rapidly repeated calls (stack pointer / ASLR differences)
    let stack_addr = (&now as *const _ as usize) as u64;

    let mut seed = now_lo ^ now_hi;
    seed = seed.wrapping_add(pid.wrapping_mul(0x9E3779B97F4A7C15));
    seed ^= counter;
    seed ^= stack_addr.wrapping_mul(0xBF58476D1CE4E5B9);
    mix64(seed)
}

/// Small deterministic pseudo-random number generator used across Roma.
///
/// The generator is intentionally dependency-free and exposes its internal
/// state so algorithms can checkpoint and resume runs exactly.
#[derive(Debug, Clone)]
pub struct Random {
    state: u64,
}

impl Random {
    /// Creates a generator initialized with `seed`.
    pub fn new(seed: u64) -> Self {
        Self { state: seed }
    }

    /// Derives a reproducible stream seed from a base seed and stream id.
    #[inline]
    pub fn derive_seed(base_seed: u64, stream: u64) -> u64 {
        // Use the same mixer as the generator to ensure small changes in
        // (base_seed, stream) produce dramatically different derived seeds.
        mix64(base_seed ^ stream.wrapping_mul(0x9E3779B97F4A7C15))
    }

    /// Returns the configured seed or a fresh time-based seed when absent.
    #[inline]
    pub fn resolve_seed(random_seed: Option<u64>) -> u64 {
        random_seed.unwrap_or_else(seed_from_time)
    }

    /// Returns the current internal generator state.
    #[inline]
    pub fn state(&self) -> u64 {
        self.state
    }

    /// Replaces the internal generator state.
    #[inline]
    pub fn set_state(&mut self, state: u64) {
        self.state = state;
    }

    /// Advances the generator and returns a 64-bit pseudo-random value.
    #[inline]
    pub fn next_u64(&mut self) -> u64 {
        // Increment state (as in SplitMix) then mix.
        self.state = self.state.wrapping_add(0x9E3779B97F4A7C15);
        mix64(self.state)
    }

    /// Advances the generator and returns the low 32 bits.
    #[inline]
    pub fn next_u32(&mut self) -> u32 {
        self.next_u64() as u32
    }

    /// Returns a floating-point value in the half-open interval `[0, 1)`.
    #[inline]
    pub fn next_f64(&mut self) -> f64 {
        // Take the top 53 bits and scale into [0,1).
        let x = (self.next_u64() >> 11) as u64;
        (x as f64) * (1.0 / 9007199254740992.0) // 1 / 2^53
    }

    /// Returns an integer in the half-open interval `[0, max)`.
    ///
    /// When `max` is zero the function returns `0`.
    #[inline]
    pub fn range(&mut self, max: u64) -> u64 {
        if max == 0 {
            return 0;
        }
        if max == 1 {
            return 0;
        }

        if max.is_power_of_two() {
            return self.next_u64() & (max - 1);
        }

        // Lemire-style unbiased reduction: use the high half of a 128-bit
        // product and only retry when the low half falls in the small biased
        // zone near zero.
        let threshold = max.wrapping_neg() % max;
        loop {
            let random = self.next_u64();
            let product = (random as u128) * (max as u128);
            let low = product as u64;
            if low >= threshold {
                return (product >> 64) as u64;
            }
        }
    }

    /// Returns an integer in the half-open interval `[min, max)`.
    ///
    /// In debug builds, invalid or empty intervals trigger a debug assertion.
    /// In release builds, `min` is returned as a defensive fallback.
    #[inline]
    pub fn range_between(&mut self, min: u64, max: u64) -> u64 {
        debug_assert!(max > min, "Random::range_between requires max > min");
        if max <= min {
            return min;
        }
        min + self.range(max - min)
    }

    /// Returns `true` with probability `p`.
    #[inline]
    pub fn chance(&mut self, p: f64) -> bool {
        self.next_f64() < p
    }

    /// Returns `true` with probability `0.5`.
    #[inline]
    pub fn coin_flip(&mut self) -> bool {
        self.chance(0.5)
    }
}

#[cfg(test)]
mod test {

    use crate::utils::random::{seed_from_time, Random};

    #[test]
    fn range_between_test() {
        let min: u64 = 100;
        let max: u64 = 200;
        let mut rng: Random = Random::new(seed_from_time());

        let x: u64 = rng.range_between(min, max);
        assert!(x >= min && x < max);
    }

    #[test]
    fn coin_flip_test() {
        let mut rng_seed_generator = Random::new(seed_from_time());
        let seed: u64 = rng_seed_generator.next_u64();

        let mut rng: Random = Random::new(seed);
        let prob_chance: f64 = 0.0;

        let x: bool = rng.chance(prob_chance);
        assert!(!x);

        let prob_chance = 1.0;
        let x: bool = rng.chance(prob_chance);
        assert!(x);
    }

    #[test]
    fn random_determinism_test() {
        let mut rng_seed_generator = Random::new(seed_from_time());
        let seed: u64 = rng_seed_generator.next_u64();

        let mut rng_1: Random = Random::new(seed);
        let mut rng_2: Random = Random::new(seed);

        assert_eq!(
            rng_1.coin_flip(),
            rng_2.coin_flip(),
            "Structure Random with the same seed should give the same result"
        );
        assert_eq!(
            rng_1.next_f64(),
            rng_2.next_f64(),
            "Structure Random with the same seed should give the same result"
        );
        assert_eq!(
            rng_1.next_u32(),
            rng_2.next_u32(),
            "Structure Random with the same seed should give the same result"
        );
        assert_eq!(
            rng_1.next_u64(),
            rng_2.next_u64(),
            "Structure Random with the same seed should give the same result"
        );
    }

    #[test]
    fn zero_seed_test() {
        let seed: u64 = 0;
        let mut rng: Random = Random::new(seed);

        let max: u64 = 200;
        let x: u64 = rng.range(max);
        assert!(x < max);
    }

    #[test]
    fn highest_seed_test() {
        let seed = u64::MAX;
        let mut rng: Random = Random::new(seed);

        let min: u64 = 100;
        let max: u64 = 200;
        let x: u64 = rng.range_between(min, max);
        assert!(x >= min && x < max);
    }

    #[test]
    fn seed_from_time_back_to_back_calls_produce_distinct_seeds() {
        let first = seed_from_time();
        let second = seed_from_time();

        assert_ne!(first, second);
    }

    #[test]
    fn range_handles_power_of_two_upper_bound() {
        let mut rng = Random::new(42);
        let upper_bound = 1024;

        for _ in 0..1024 {
            assert!(rng.range(upper_bound) < upper_bound);
        }
    }

    #[test]
    fn range_handles_large_upper_bound() {
        let mut rng = Random::new(42);
        let upper_bound = u64::MAX;

        for _ in 0..1024 {
            assert!(rng.range(upper_bound) < upper_bound);
        }
    }
}
