# Roma

**Roma** is an extensible, high-performance metaheuristic optimization library written entirely in Rust. It was developed as the practical outcome of the Bachelor's Thesis *Extensible Metaheuristic Optimization Library in Rust* (University of Málaga, 2026).

The library provides reusable abstractions for modelling optimization problems, composing algorithms and operators, running reproducible experiments, and monitoring execution. It supports both single-objective and multi-objective optimization while following a zero-external-dependencies design.

## Highlights

- **Rust-native and dependency-free:** the crate is self-contained, including random-number generation, serialization, chart generation, and command-line utilities.
- **Extensible architecture:** generic `Problem`, `Solution`, `Algorithm`, and operator traits keep problem modelling separate from search logic.
- **Single- and multi-objective optimization:** supports scalar fitness and Pareto-based workflows, including NSGA-II and crowding-distance quality metadata.
- **Built-in algorithms:** Hill Climbing, Simulated Annealing, Genetic Algorithm, Particle Swarm Optimization, Differential Evolution, NSGA-II, Tabu Search, and Variable Neighbourhood Search.
- **Composable operators:** selection, crossover, mutation, neighbourhood, and tabu-memory operators can be exchanged independently.
- **Experimentation support:** repeated and parallel executions, statistical summaries, observer-based monitoring, and checkpoint utilities.
- **Memory-safe concurrency:** Rust's type system prevents data races without requiring a garbage collector.

## Evaluation

The thesis evaluates Roma using Rastrigin, TSP, Knapsack, ZDT1, and Ackley benchmarks against jMetal, jMetalPy, DEAP, MEALPY, pagmo2, and SciPy. The experimental protocol uses independent stochastic runs and Friedman/Nemenyi statistical tests.

Selected results reported in the thesis:

| Scenario | Result |
| --- | --- |
| ZDT1 with NSGA-II (25,000 evaluations) | Highest reported median hypervolume (10.7700); 175.94 ms median runtime, compared with 4,411.62 ms for DEAP. |
| Ackley with Differential Evolution (35 dimensions, 6,400 evaluations) | Similar solution quality to DEAP, with a 33.9× lower median runtime (11.53 ms vs. 391.00 ms). |
| Continuous and combinatorial benchmarks | Competitive solution quality and runtime relative to the evaluated Rust, C++, Java, and Python implementations. |

These figures apply to the benchmark configurations described in [`docs/TFG.pdf`](docs/TFG.pdf); they are not general performance guarantees.

## Installation

Add Roma to your Rust project:

```toml
[dependencies]
roma_lib = "0.1.3"
```

To use the repository version instead:

```toml
[dependencies]
roma_lib = { git = "https://github.com/DRLKs/roma.git", path = "roma" }
```

Roma requires Rust 1.90 or newer. To work on the source tree:

```bash
git clone https://github.com/DRLKs/roma.git
cd roma
cargo test --manifest-path roma/Cargo.toml
```

## Quick start

This example solves a small 0/1 knapsack instance with Hill Climbing:

```rust
use roma_lib::algorithms::{
    Algorithm, HillClimbing, HillClimbingParameters, TerminationCriteria,
    TerminationCriterion,
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

    let parameters = HillClimbingParameters::new(
        BitFlipNeighborhood::new(),
        TerminationCriteria::new(vec![TerminationCriterion::MaxIterations(500)]),
    )
    .with_seed(42);

    let mut algorithm = HillClimbing::new(parameters);
    let solutions = algorithm.run(&problem).expect("optimization failed");

    if let Some(best) = solutions.best_solution(&problem) {
        println!("Best quality: {:.4}", best.quality_value());
    }
}
```

More examples are available in [`roma/examples`](roma/examples), including TSP, QAP, Rastrigin, Ackley, ZDT1/NSGA-II, experiments, and parallel execution.

## Architecture

```text
Problem ── evaluates ──> Solution <── stores ── SolutionSet
   │                         ▲
   └── guides ──> Algorithm ─┘
                        │
                        ├── Operators (selection, crossover, mutation, neighbourhood)
                        ├── Observers (console, SVG chart, HTML report)
                        └── Experiment runner, parallel execution, and checkpoints
```

The main extension points are:

- `Problem<T, Q>` defines the domain, evaluation function, objective direction, and solution formatting.
- `Solution<T, Q>` stores decision variables and quality information.
- `Algorithm<T, Q>` implements an optimization lifecycle and returns a `SolutionSet`.
- Operator traits make variation and neighbourhood strategies interchangeable.
- `AlgorithmObserver` receives runtime events for monitoring and reporting.

## Build, test, and documentation

```bash
# Run tests
cargo test --manifest-path roma/Cargo.toml

# Build an optimized library
cargo build --manifest-path roma/Cargo.toml --release

# Run an example
cargo run --manifest-path roma/Cargo.toml --example knapsack_hc_demo

# Generate local API documentation
cargo doc --manifest-path roma/Cargo.toml --no-deps
```

API documentation is published at [docs.rs/roma_lib](https://docs.rs/roma_lib).

## Repository layout

```text
roma/              Rust crate, examples, and tests
benchmark_suite/   Reproducible benchmark runners and analysis tooling
docs/TFG.pdf       Bachelor's Thesis and experimental methodology
```

## Thesis

The library and its architecture, implementation, validation, limitations, and future work are documented in [`docs/TFG.pdf`](docs/TFG.pdf).

```bibtex
@thesis{roma_lib,
  author = {Muñoz del Valle, David},
  coauthor = {Luque Polo, Gabriel Jesús},
  title  = {Extensible Metaheuristic Optimization Library in Rust},
  school = {University of Málaga},
  year   = {2026},
  type   = {Bachelor's Thesis}
}
```

## License

Licensed under either of the following, at your option:

- [MIT License](roma/LICENSE)
- [Apache License, Version 2.0](https://www.apache.org/licenses/LICENSE-2.0)
