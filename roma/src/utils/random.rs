use std::time::{SystemTime, UNIX_EPOCH};

pub fn seed_from_time() -> u64 {
    SystemTime::now()
        .duration_since(UNIX_EPOCH)
        .unwrap()
        .as_nanos() as u64
}

#[derive(Debug, Clone)]
pub struct Random {
    state: u64,
}

impl Random {
    /// Crea un RNG con semilla
    pub fn new(seed: u64) -> Self {
        Self { state: seed }
    }

    #[inline]
    pub fn state(&self) -> u64 {
        self.state
    }

    #[inline]
    pub fn set_state(&mut self, state: u64) {
        self.state = state;
    }

    #[inline]
    pub fn next_u64(&mut self) -> u64 {
        self.state = self.state.wrapping_add(0x9E3779B97F4A7C15);
        let mut z = self.state;
        z = (z ^ (z >> 30)).wrapping_mul(0xBF58476D1CE4E5B9);
        z = (z ^ (z >> 27)).wrapping_mul(0x94D049BB133111EB);
        z ^ (z >> 31)
    }

    #[inline]
    pub fn next_u32(&mut self) -> u32 {
        self.next_u64() as u32
    }

    /// [0, 1)
    #[inline]
    pub fn next_f64(&mut self) -> f64 {
        let x = self.next_u64() >> 11;
        (x as f64) * (1.0 / ((1u64 << 53) as f64))
    }

    /// [0, max)
    #[inline]
    pub fn range(&mut self, max: u64) -> u64 {
        if max == 0 {
            return 0;
        }
        self.next_u64() % max
    }

    /// [min, max)
    #[inline]
    pub fn range_between(&mut self, min: u64, max: u64) -> u64 {
        min + self.range(max - min)
    }

    /// p ∈ \[0,1\]
    #[inline]
    pub fn chance(&mut self, p: f64) -> bool {
        self.next_f64() < p
    }

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
}
