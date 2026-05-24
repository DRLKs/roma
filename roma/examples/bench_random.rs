use std::hint::black_box;
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

fn parse_rounds_or_env(default: usize) -> usize {
    let arg = std::env::args().nth(2);
    if let Some(s) = arg {
        if let Ok(n) = s.parse::<usize>() {
            return n.max(1);
        }
    }
    if let Ok(s) = std::env::var("ROUNDS") {
        if let Ok(n) = s.parse::<usize>() {
            return n.max(1);
        }
    }
    default
}

fn bench_next_u64_new(iters: usize, seed: u64) -> (u128, u64) {
    let mut rng = Random::new(seed);
    let mut acc: u64 = 0;
    let start = Instant::now();
    for _ in 0..iters {
        acc = acc.wrapping_add(black_box(rng.next_u64()));
    }
    (start.elapsed().as_nanos(), black_box(acc))
}

fn bench_next_u64_legacy(iters: usize, seed: u64) -> (u128, u64) {
    let mut rng = LegacyRandom::new(seed);
    let mut acc: u64 = 0;
    let start = Instant::now();
    for _ in 0..iters {
        acc = acc.wrapping_add(black_box(rng.next_u64()));
    }
    (start.elapsed().as_nanos(), black_box(acc))
}

fn bench_range_new(iters: usize, max: u64, seed: u64) -> (u128, u64) {
    let mut rng = Random::new(seed);
    let mut acc: u64 = 0;
    let max = black_box(max);
    let start = Instant::now();
    for _ in 0..iters {
        acc = acc.wrapping_add(black_box(rng.range(max)));
    }
    (start.elapsed().as_nanos(), black_box(acc))
}

fn bench_range_legacy(iters: usize, max: u64, seed: u64) -> (u128, u64) {
    let mut rng = LegacyRandom::new(seed);
    let mut acc: u64 = 0;
    let max = black_box(max);
    let start = Instant::now();
    for _ in 0..iters {
        acc = acc.wrapping_add(black_box(rng.range(max)));
    }
    (start.elapsed().as_nanos(), black_box(acc))
}

fn print_result(label: &str, iters: usize, nanos: u128, acc: u64) {
    let secs = (nanos as f64) / 1e9;
    let ops_per_sec = (iters as f64) / secs;
    println!("{:<22} : {:>12} ops in {:>8.3} s -> {:>12.0} ops/s  (acc={})", label, iters, secs, ops_per_sec, acc);
}

fn median_nanos(samples: &mut [u128]) -> u128 {
    samples.sort_unstable();
    samples[samples.len() / 2]
}

fn print_summary(label: &str, iters: usize, samples: &mut [u128]) {
    let median = median_nanos(samples);
    let secs = (median as f64) / 1e9;
    let ops_per_sec = (iters as f64) / secs;
    println!("{:<22} : median {:>8.3} s -> {:>12.0} ops/s", label, secs, ops_per_sec);
}

fn warm_up(iters: usize, base_seed: u64, range_max: u64) {
    let warmup_iters = iters.clamp(10_000, 1_000_000);
    let _ = bench_next_u64_new(warmup_iters, Random::derive_seed(base_seed, 1));
    let _ = bench_next_u64_legacy(warmup_iters, Random::derive_seed(base_seed, 1));
    let _ = bench_range_new(warmup_iters, range_max, Random::derive_seed(base_seed, 2));
    let _ = bench_range_legacy(warmup_iters, range_max, Random::derive_seed(base_seed, 2));
}

fn main() {
    let iters = parse_arg_or_env(20_000_000);
    let rounds = parse_rounds_or_env(5);
    let range_max: u64 = 1_000_003; // prime-ish to exercise rejection/modulo

    let seed = seed_from_time();

    println!("Benchmark iterations = {}", iters);
    println!("Rounds = {}", rounds);
    println!("Seed = {}", seed);

    warm_up(iters, seed, range_max);

    let mut next_new_samples = Vec::with_capacity(rounds);
    let mut next_old_samples = Vec::with_capacity(rounds);
    let mut range_new_samples = Vec::with_capacity(rounds);
    let mut range_old_samples = Vec::with_capacity(rounds);

    let mut last_next_new_acc = 0;
    let mut last_next_old_acc = 0;
    let mut last_range_new_acc = 0;
    let mut last_range_old_acc = 0;

    for round in 0..rounds {
        let round_seed = Random::derive_seed(seed, round as u64);

        if round % 2 == 0 {
            let (n_new, acc_new) = bench_next_u64_new(iters, round_seed);
            let (n_old, acc_old) = bench_next_u64_legacy(iters, round_seed);
            next_new_samples.push(n_new);
            next_old_samples.push(n_old);
            last_next_new_acc = acc_new;
            last_next_old_acc = acc_old;

            let (rn_new, racc_new) = bench_range_new(iters, range_max, round_seed);
            let (rn_old, racc_old) = bench_range_legacy(iters, range_max, round_seed);
            range_new_samples.push(rn_new);
            range_old_samples.push(rn_old);
            last_range_new_acc = racc_new;
            last_range_old_acc = racc_old;
        } else {
            let (n_old, acc_old) = bench_next_u64_legacy(iters, round_seed);
            let (n_new, acc_new) = bench_next_u64_new(iters, round_seed);
            next_old_samples.push(n_old);
            next_new_samples.push(n_new);
            last_next_old_acc = acc_old;
            last_next_new_acc = acc_new;

            let (rn_old, racc_old) = bench_range_legacy(iters, range_max, round_seed);
            let (rn_new, racc_new) = bench_range_new(iters, range_max, round_seed);
            range_old_samples.push(rn_old);
            range_new_samples.push(rn_new);
            last_range_old_acc = racc_old;
            last_range_new_acc = racc_new;
        }
    }

    print_result(
        "new::next_u64(last)",
        iters,
        *next_new_samples.last().unwrap(),
        last_next_new_acc,
    );
    print_result(
        "legacy::next_u64(last)",
        iters,
        *next_old_samples.last().unwrap(),
        last_next_old_acc,
    );
    print_summary("new::next_u64", iters, &mut next_new_samples);
    print_summary("legacy::next_u64", iters, &mut next_old_samples);

    print_result(
        "new::range(last)",
        iters,
        *range_new_samples.last().unwrap(),
        last_range_new_acc,
    );
    print_result(
        "legacy::range(last)",
        iters,
        *range_old_samples.last().unwrap(),
        last_range_old_acc,
    );
    print_summary("new::range", iters, &mut range_new_samples);
    print_summary("legacy::range", iters, &mut range_old_samples);

    println!("Done.");
}
