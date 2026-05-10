# QAP Genetic Algorithm Benchmark

This benchmark mirrors the structure of `tsp_ga`, but compares
genetic-algorithm implementations on the Quadratic Assignment Problem (QAP).

## Layout

- `shared/`: common QAP instance, budgets, seeds, and a plain-text QAP file reused by jMetal and pagmo2.
- `runners/rust/`: benchmark-specific Roma source code.
- `runners/python/`: Python wrappers and Python-native baselines.
- `runners/java/`: Java source for the jMetal benchmark.
- `runners/cpp/`: C++ source for the pagmo2 benchmark.
- `results/raw/`: raw JSON outputs from the last executions.
- `results/summary.json`: aggregate summary from the last execution.

## Roma Source Of Truth

The Roma benchmark implementation lives in:

- `runners/rust/qap_ga_benchmark.rs`

The file `roma/examples/qap_ga_benchmark.rs` is only a thin bridge that includes
this benchmark-local source so future changes can be made directly inside this
directory.

## Run

From the repository root:

```bash
/home/drlk/roma/.venv/bin/python benchmark_suite/qap_ga/orchestrate.py
```

That command builds the local Docker image for this benchmark and runs Roma,
DEAP, jMetalPy, jMetal Java, pagmo2 C++, and mealpy inside the same container.

Note: mealpy is executed from a dedicated virtual environment inside the image
because mealpy 3.0.3 requires a different NumPy major version than jMetalPy.