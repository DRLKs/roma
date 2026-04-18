# Roma

`roma` is a Rust metaheuristics framework for optimization and experimentation.

It provides reusable building blocks to define optimization problems, configure algorithms,
run reproducible executions, and observe progress through console or file-based reports.

## Features

- Single-objective algorithms: Hill Climbing, Genetic Algorithm, Simulated Annealing, PSO.
- Multi-objective optimization with NSGA-II and Pareto rank/crowding quality metadata.
- Generic `Problem` and `Solution` abstractions for custom domains.
- Structured observers (console, chart, HTML report) and checkpoint utilities.
- Experiment runner for repeated case execution and comparative summaries.

## Installation

```toml
[dependencies]
roma = "0.1.0"
```

## Quick Start

```rust
use roma::algorithms::{
    Algorithm,
    HillClimbing,
    HillClimbingParameters,
    TerminationCriteria,
    TerminationCriterion,
};
use roma::operator::BitFlipMutation;
use roma::problem::KnapsackBuilder;
use roma::solution_set::SolutionSet;

fn main() {
    let problem = KnapsackBuilder::new()
        .with_capacity(40.0)
        .add_items(vec![(4.0, 8.0), (7.0, 13.0), (5.0, 10.0), (3.0, 4.0)])
        .build();

    let params = HillClimbingParameters::new(
        BitFlipMutation::new(),
        0.15,
        TerminationCriteria::new(vec![TerminationCriterion::MaxIterations(50)]),
    )
    .with_seed(42);

    let mut algorithm = HillClimbing::new(params);
    let solutions = algorithm.run(&problem).expect("run should succeed");

    if let Some(best) = solutions.best_solution() {
        println!("best quality: {}", best.quality_value());
    }
}
```

## Core Concepts

- `Problem<T, Q>` defines solution generation, evaluation, objective direction, and formatting.
- `Solution<T, Q>` stores decision variables and an optional quality cache.
- `Algorithm<T, Q>` executes a shared runtime lifecycle and returns a `SolutionSet`.
- Observers subscribe to runtime events and can render progress/summary outputs.
- `Experiment` runs multiple algorithm cases repeatedly and computes aggregate statistics.

## Documentation

- API docs: <https://docs.rs/roma>
- Repository: <https://github.com/DRLKs/roma>

## License

Licensed under either:

- MIT license
- Apache License, Version 2.0

at your option.
