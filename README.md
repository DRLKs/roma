<p align="center">
  <a href="https://www.rust-lang.org/"><img src="https://img.shields.io/badge/Made%20with-Rust-black?style=for-the-badge&logo=rust" alt="Made with Rust"></a>
  <a href="https://crates.io/crates/roma_lib"><img src="https://img.shields.io/crates/v/roma_lib?style=for-the-badge&logo=rust&color=orange" alt="Crates.io"></a>
  <a href="https://docs.rs/roma_lib"><img src="https://img.shields.io/docsrs/roma_lib?style=for-the-badge&logo=readthedocs" alt="docs.rs"></a>
  <a href="https://github.com/DRLKs/roma/actions/workflows/rust-tests.yml"><img src="https://img.shields.io/github/actions/workflow/status/DRLKs/roma/rust-tests.yml?branch=main&style=for-the-badge" alt="CI"></a>
  <a href="https://github.com/DRLKs/roma/blob/main/roma/LICENSE"><img src="https://img.shields.io/badge/license-MIT%2FApache--2.0-blue?style=for-the-badge" alt="License"></a>
</p>

# Roma

**Roma** is a high-performance, zero-dependency metaheuristic optimization library written entirely in Rust. It provides a modular and extensible framework for solving single-objective and multi-objective optimization problems using population-based and trajectory-based algorithms.

Designed as the foundation of a Bachelor's Thesis in Software Engineering (University of Málaga), Roma demonstrates that Rust's ownership model, static dispatch, and zero-cost abstractions can deliver C++-class performance with full memory safety — no garbage collector, no runtime overhead.

## Key Features

- **Zero external dependencies** — the entire library (PRNG, serialization, SVG rendering, CLI parsing) is self-contained.
- **Generic abstractions** — `Problem<T, Q>` and `Solution<T, Q>` decouple domain logic from algorithmic machinery.
- **Built-in algorithms** — Hill Climbing, Simulated Annealing, Genetic Algorithm, Particle Swarm Optimization (PSO), Differential Evolution, and NSGA-II.
- **Composable operators** — mutation, crossover, selection, neighborhood, and tabu memory operators are fully interchangeable via traits.
- **Experiment engine** — automated parallel execution of repeated runs with configurable thread pools and statistical reporting.
- **Observer system** — real-time monitoring via channels with built-in console, SVG chart, and HTML report observers.
- **Checkpoint / fault tolerance** — periodic state persistence to disk with automatic resume support.
- **Cross-platform** — Linux, macOS, and Windows support with OS-aware storage paths.

## Performance

Roma has been rigorously benchmarked against jMetal, jMetalPy, DEAP, mealpy, pagmo2 (C++), and SciPy across standardized problems (Rastrigin, TSP, Knapsack, ZDT1, Ackley). Key findings:

| Metric | Result |
|--------|--------|
| Hill Climbing (Rastrigin D=80) | Best convergence among all libraries; 25 ms median vs 363 ms (DEAP) |
| GA on TSP-48 (5s budget) | Statistically tied with pagmo2 (C++); 10–25× faster than Python alternatives |
| NSGA-II on ZDT1 (D=30) | Best hypervolume; 175 ms vs 4400 ms (DEAP), 25× speedup |
| Differential Evolution (Ackley D=35) | 34× faster than DEAP with identical solution quality |

All comparisons validated with Friedman + Nemenyi post-hoc tests at α = 0.05.

## Installation

Add to your `Cargo.toml`:

```toml
[dependencies]
roma_lib = "0.1.1"
```

Or depend on the latest development version:

```toml
[dependencies]
roma_lib = { git = "https://github.com/DRLKs/roma.git", path = "roma" }
```

## Quick Example

```rust
use roma_lib::algorithms::{
    Algorithm, HillClimbing, HillClimbingParameters,
    TerminationCriteria, TerminationCriterion,
};
use roma_lib::operator::BitFlipNeighborhood;
use roma_lib::problem::KnapsackBuilder;
use roma_lib::solution_set::SolutionSet;

fn main() {
    let problem = KnapsackBuilder::new()
        .with_capacity(90.0)
        .add_item(12.0, 24.0)
        .add_item(22.0, 33.0)
        .add_item(41.0, 80.0)
        .build();

    let params = HillClimbingParameters::new(
        BitFlipNeighborhood::new(),
        TerminationCriteria::new(vec![TerminationCriterion::MaxIterations(500)]),
    );

    let mut algorithm = HillClimbing::new(params);
    let result = algorithm.run(&problem).expect("execution failed");

    if let Some(best) = result.best_solution(&problem) {
        println!("Best fitness: {:.4}", best.quality_value());
    }
}
```

See the [`examples/`](roma/examples/) directory for more complete scenarios including multi-objective optimization, experiment comparison, and PSO.

## Repository Layout

```
roma/           Main Rust crate (library + examples + tests)
benchmark_suite/   Reproducible benchmark infrastructure (Docker + Python orchestrator)
docs/           Architecture diagrams and thesis documentation
```

## Building & Testing

```bash
# Run the full test suite
cargo test --manifest-path roma/Cargo.toml

# Build optimized release
cargo build --manifest-path roma/Cargo.toml --release

# Generate API documentation
cargo doc --manifest-path roma/Cargo.toml --no-deps --open

# Run an example
cargo run --manifest-path roma/Cargo.toml --example knapsack_ga_demo
```

## Documentation

- **API reference**: <https://docs.rs/roma_lib>
- **Crate on crates.io**: <https://crates.io/crates/roma_lib>

## Contributing

Contributions are welcome and encouraged. Whether it is a bug fix, a new algorithm, an operator implementation, or documentation improvements — all contributions help make Roma better.

### How to contribute

1. **Fork** the repository and create a feature branch from `main`.
2. **Implement** your changes following the existing code conventions.
3. **Add tests** for any new functionality.
4. **Open a Pull Request** with a clear description of the change and its motivation.

### Ideas for contributions

- New metaheuristic algorithms (Tabu Search, MOEA/D, NSGA-III, CMA-ES, ...)
- Additional operators (adaptive mutation, differential crossover variants, ...)
- New benchmark problems and problem parsers
- Performance optimizations (zero-copy improvements, SIMD, ...)
- Documentation, examples, and tutorials

### Reporting issues

If you find a bug, have a question, or want to suggest an enhancement, please [open an issue](https://github.com/DRLKs/roma/issues). Include reproduction steps and your Rust version when reporting bugs.

## License

Licensed under either of the following, at your option:

- [MIT License](roma/LICENSE)
- [Apache License, Version 2.0](https://www.apache.org/licenses/LICENSE-2.0)

## Citation

If you use Roma in academic work, please consider citing:

```bibtex
@thesis{munoz2026roma,
  author  = {Muñoz del Valle, David},
  title   = {Extensible metaheuristic optimization library in Rust},
  school  = {University of Málaga},
  year    = {2026},
  type    = {Bachelor's Thesis}
}
```
drlk ~/roma ❯