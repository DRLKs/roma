# Roma

**Roma** is an extensible, dependency-free metaheuristic optimization library for Rust. It was developed as the practical outcome of the Bachelor's Thesis *Extensible Metaheuristic Optimization Library in Rust* (University of Málaga, 2026).

It separates the definition of an optimization problem from the search engine: model a domain once, select an algorithm and its operators, then run, observe, and compare configurations. Roma supports single- and multi-objective workflows while keeping its implementation self-contained.

It is intended for experimentation and for building custom optimizers—not as a claim that one metaheuristic is universally best. Choosing and tuning an algorithm remains problem-dependent.

## Highlights

- **Rust-native and dependency-free:** the crate is self-contained, including random-number generation, serialization, chart generation, and command-line utilities.
- **Extensible architecture:** generic `Problem`, `Solution`, `Algorithm`, and operator traits separate domain modelling from search logic.
- **Single- and multi-objective optimization:** supports scalar fitness and Pareto-based workflows, including NSGA-II and crowding-distance quality metadata.
- **Built-in algorithms:** Hill Climbing, Simulated Annealing, Genetic Algorithm, Particle Swarm Optimization, Differential Evolution, NSGA-II, Tabu Search, and Variable Neighbourhood Search.
- **Composable operators:** selection, crossover, mutation, neighbourhood, and tabu-memory operators can be exchanged independently.
- **Experimentation support:** repeated and parallel executions, comparative summaries, observer-based monitoring, and checkpoint utilities.
- **Memory-safe concurrency:** Rust's type system prevents data races without requiring a garbage collector.

## How it fits together

```text
Problem ── creates and evaluates ──> Solution ── collected by ──> SolutionSet
   │                                      ▲
   └────────── guides ───────────────> Algorithm
                                           │
                     Parameters + operators + termination criteria
                                           │
                     Observers, checkpoints, and experiment runner
```

The core extension points are:

- `Problem<T, Q>` defines valid candidates, their evaluation, objective direction, and presentation.
- `Solution<T, Q>` holds decision variables and quality metadata. Multi-objective solutions can carry Pareto rank and crowding-distance information.
- `Algorithm<T, Q>` implements the search lifecycle and returns a `SolutionSet` rather than a raw collection.
- Operator traits cover selection, crossover, mutation, neighbourhoods, and memory, so they can be composed independently of the algorithm.
- `AlgorithmObserver` receives execution events without coupling reporting to the optimizer itself.

## Evaluation

The thesis evaluates Roma on Rastrigin, TSP, Knapsack, ZDT1, and Ackley against jMetal, jMetalPy, DEAP, MEALPY, pagmo2, SciPy, and other problem-specific references. The protocol uses independent stochastic runs and Friedman/Nemenyi statistical analysis where applicable.

Selected results reported in the thesis:

| Scenario | Result |
| --- | --- |
| ZDT1 with NSGA-II (25,000 evaluations) | Median hypervolume of 10.7700 and median runtime of 175.94 ms; DEAP took 4,411.62 ms in the same benchmark. |
| Ackley with Differential Evolution (35 dimensions, 6,400 evaluations) | Similar solution quality to DEAP with a 33.9× lower median runtime (11.53 ms vs. 391.00 ms). |
| TSP-48 with a Genetic Algorithm (5-second budget) | 3.18 million median evaluations and a median route length of 1298.5; statistically tied with pagmo2 in the reported comparison. |

These figures apply only to the configurations, hardware, and implementations described in [`docs/TFG.pdf`](docs/TFG.pdf). They are not general performance guarantees.

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

## Observe a run

Attach observers before calling `run` to obtain console output, an SVG chart, or an HTML report without embedding reporting code in the problem or algorithm:

```rust
use roma_lib::HtmlReportObserver;
use roma_lib::observer::{ChartObserver, ConsoleObserver, Observable};

algorithm.add_observer(Box::new(ConsoleObserver::new(true)));
algorithm.add_observer(Box::new(ChartObserver::new_default()));
algorithm.add_observer(Box::new(HtmlReportObserver::new_default()));
```

For repeatable local runs, pass a fixed seed with the algorithm parameters. Parallel stochastic runs are reproducible under the same execution conditions, but scheduling can make results differ across machines with a different number of cores.

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
docs/TFG.tex       Thesis source
```

## Thesis

The library's architecture, implementation, evaluation methodology, limitations, and future work are documented in [`docs/TFG.pdf`](docs/TFG.pdf). The editable source is available as [`docs/TFG.tex`](docs/TFG.tex).

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
