// Thin wrapper so Cargo builds the benchmark-local runner located under
// `benchmark_suite/knapsack_sa/runners/rust/knapsack_sa_benchmark.rs`.

include!(concat!(env!("CARGO_MANIFEST_DIR"), "/../benchmark_suite/knapsack_sa/runners/rust/knapsack_sa_benchmark.rs"));
