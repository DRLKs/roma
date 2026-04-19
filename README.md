<img src="./images/rMetal_logo.png" alt="Roma logo" width="250"/>

<p align="center">
  <a href="https://www.rust-lang.org/"><img src="https://img.shields.io/badge/Made%20with-Rust-black?style=for-the-badge&logo=rust" alt="Made with Rust"></a>
  <a href="https://crates.io/crates/roma"><img src="https://img.shields.io/badge/crates.io-roma-orange?style=for-the-badge&logo=rust" alt="Crates.io"></a>
  <a href="https://docs.rs/roma"><img src="https://img.shields.io/badge/docs.rs-roma-blue?style=for-the-badge&logo=readthedocs" alt="docs.rs"></a>
  <a href="https://github.com/DRLKs/roma/actions/workflows/rust-tests.yml"><img src="https://img.shields.io/github/actions/workflow/status/DRLKs/roma/rust-tests.yml?branch=main&style=for-the-badge" alt="CI"></a>
</p>

# Roma

Roma is a Rust metaheuristics framework for optimization and experimentation.

It provides a practical toolkit to model optimization problems, execute
single-objective and multi-objective algorithms, observe runtime behavior, and
compare experiments with repeatable settings.

## Highlights

- Unified algorithm runtime with shared lifecycle and termination handling.
- Built-in algorithms: Hill Climbing, Genetic Algorithm, Simulated Annealing,
  PSO, and NSGA-II.
- Generic `Problem<T, Q>` and `Solution<T, Q>` abstractions.
- Observer system with console output, SVG charts, and HTML reports.
- Experiment runner for repeated case execution and summary statistics.

## Installation

```toml
[dependencies]
roma = "0.1.0"
```

While development is ongoing, you can also depend on Git:

```toml
[dependencies]
roma = { git = "https://github.com/DRLKs/roma.git" }
```

## Documentation

- Crate docs: <https://docs.rs/roma>
- Crate README (docs.rs source): `roma/README.md`
- API entry point: `roma/src/lib.rs`

## Repository Layout

- `roma/` - main Rust crate (`roma`)
- `docs/` - project and architecture notes
- `images/` - visual assets

## Quick Start

Run the crate tests locally:

```bash
cargo test --manifest-path roma/Cargo.toml
```

Build API documentation locally:

```bash
cargo doc --manifest-path roma/Cargo.toml --no-deps
```

## License

Licensed under either of the following, at your option:

- MIT License
- Apache License, Version 2.0
