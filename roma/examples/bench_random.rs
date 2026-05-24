use std::time::Instant;

use roma_lib::utils::random::{seed_from_time, Random};

/// Legacy RNG reproduced from the previous implementation for baseline
/// (keeps modulo-based `range` and the older mixing sequence).
#[derive(Clone)]
struct LegacyRandom {
    state: u64,
}

impl LegacyRandom {
    pub fn new(seed: u64) -> Self {
        Self { state: seed }
    }

    pub fn next_u64(&mut self) -> u64 {
        self.state = self.state.wrapping_add(0x9E3779B97F4A7C15);
        let mut z = self.state;
        z = (z ^ (z >> 30)).wrapping_mul(0xBF58476D1CE4E5B9);
        z = (z ^ (z >> 27)).wrapping_mul(0x94D049BB133111EB);
        z ^ (z >> 31)
    }

    pub fn range(&mut self, max: u64) -> u64 {
        if max == 0 {
            return 0;
        }
        self.next_u64() % max
    }
}

fn parse_arg_or_env(default: usize) -> usize {
    let arg = std::env::args().nth(1);
    if let Some(s) = arg {
        if let Ok(n) = s.parse::<usize>() {
            return n;
        }
    }
    if let Ok(s) = std::env::var("ITER") {
        if let Ok(n) = s.parse::<usize>() {
            return n;
        }
    }
    default
}

fn bench_next_u64_new(iters: usize, seed: u64) -> (u128, u64) {
    let mut rng = Random::new(seed);
    let mut acc: u64 = 0;
    let start = Instant::now();
    for _ in 0..iters {
        acc = acc.wrapping_add(rng.next_u64());
    }
    (start.elapsed().as_nanos(), acc)
}

fn bench_next_u64_legacy(iters: usize, seed: u64) -> (u128, u64) {
    let mut rng = LegacyRandom::new(seed);
    let mut acc: u64 = 0;
    let start = Instant::now();
    for _ in 0..iters {
        acc = acc.wrapping_add(rng.next_u64());
    }
    (start.elapsed().as_nanos(), acc)
}

fn bench_range_new(iters: usize, max: u64, seed: u64) -> (u128, u64) {
    let mut rng = Random::new(seed);
    let mut acc: u64 = 0;
    let start = Instant::now();
    for _ in 0..iters {
        acc = acc.wrapping_add(rng.range(max));
    }
    (start.elapsed().as_nanos(), acc)
}

fn bench_range_legacy(iters: usize, max: u64, seed: u64) -> (u128, u64) {
    let mut rng = LegacyRandom::new(seed);
    let mut acc: u64 = 0;
    let start = Instant::now();
    for _ in 0..iters {
        acc = acc.wrapping_add(rng.range(max));
    }
    (start.elapsed().as_nanos(), acc)
}

fn print_result(label: &str, iters: usize, nanos: u128, acc: u64) {
    let secs = (nanos as f64) / 1e9;
    let ops_per_sec = (iters as f64) / secs;
    println!("{:<22} : {:>12} ops in {:>8.3} s -> {:>12.0} ops/s  (acc={})", label, iters, secs, ops_per_sec, acc);
}

fn main() {
    let iters = parse_arg_or_env(20_000_000);
    let range_max: u64 = 1_000_003; // prime-ish to exercise rejection/modulo

    let seed = seed_from_time();

    println!("Benchmark iterations = {}", iters);
    println!("Seed = {}", seed);

    // next_u64 throughput
    let (n_new, acc_new) = bench_next_u64_new(iters, seed);
    print_result("new::next_u64", iters, n_new, acc_new);

    let (n_old, acc_old) = bench_next_u64_legacy(iters, seed);
    print_result("legacy::next_u64", iters, n_old, acc_old);

    // range throughput
    let (rn_new, racc_new) = bench_range_new(iters, range_max, seed);
    print_result("new::range", iters, rn_new, racc_new);

    let (rn_old, racc_old) = bench_range_legacy(iters, range_max, seed);
    print_result("legacy::range", iters, rn_old, racc_old);

    println!("Done.");
}
