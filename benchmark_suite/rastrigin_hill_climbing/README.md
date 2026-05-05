# Rastrigin Hill Climbing Benchmark

This benchmark is self-contained and organized by role and language so each implementation lives next to the rest of the benchmark assets.

## Layout

- `shared/`: common benchmark instance, budgets, and seeds.
- `runners/rust/`: benchmark-specific Roma source code.
- `runners/python/`: Python wrappers and Python-native baselines.
- `runners/java/`: Java source for the jMetal benchmark.
- `runners/cpp/`: C++ source for the pagmo2 benchmark.
- `results/raw/`: raw JSON outputs from the last executions.
- `results/summary.json`: aggregate summary from the last execution.

## Roma Source Of Truth

The Roma benchmark implementation lives in:

- `runners/rust/rastrigin_hill_climbing_benchmark.rs`

The file `roma/examples/rastrigin_hill_climbing_benchmark.rs` is only a thin bridge that includes this benchmark-local source so future changes can be made directly inside this directory.

## Run

From the repository root:

```bash
/home/drlk/roma/.venv/bin/python benchmark_suite/rastrigin_hill_climbing/orchestrate.py
```

That command builds the local Docker image for this benchmark and runs Roma, DEAP, jMetalPy, jMetal Java, pagmo2 C++, and mealpy inside the same container.

Note: mealpy is executed from a dedicated virtual environment inside the image because it requires a different NumPy major version than jMetalPy.
