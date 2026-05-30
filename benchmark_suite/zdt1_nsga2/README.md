# ZDT1 NSGA-II Benchmark

This benchmark compares multi-objective NSGA-II implementations on the classic
continuous ZDT1 test problem.

## Layout

- `shared/`: common ZDT1 instance, budgets, seeds, and metric configuration.
- `runners/rust/`: benchmark-specific Roma source code.
- `runners/python/`: Python-native runners and wrappers for Java/C++ backends.
- `runners/java/`: Java source for the jMetal benchmark.
- `runners/cpp/`: C++ source for the pagmo2 benchmark.
- `results/runs.csv`: unified per-run CSV output for the benchmark.
- `results/summary.json`: aggregate summary derived from the CSV.
- `results/latex/`: benchmark-local LaTeX tables for descriptive statistics.

## Libraries

This benchmark is intended to compare:

- Roma (Rust)
- DEAP (Python)
- jMetalPy (Python)
- jMetal Java (Java)
- pagmo2 (C++)

`mealpy` is intentionally excluded here because its documented multi-objective
support is based on weighted scalarization rather than an NSGA-II Pareto-front
workflow comparable to the other libraries.

## Primary Metric

The suite stores the final Pareto front for each run and uses the 2D
hypervolume of that front as the primary scalar comparison metric. Therefore the
benchmark uses `objective_sense = max` even though each ZDT1 objective is
minimized.

## Roma Source Of Truth

The Roma benchmark implementation lives in:

- `runners/rust/zdt1_nsga2_benchmark.rs`

The file `roma/examples/zdt1_nsga2_benchmark.rs` is only a thin bridge that
includes this benchmark-local source.

## Run

From the repository root:

```bash
/home/drlk/roma/.venv/bin/python benchmark_suite/zdt1_nsga2/orchestrate.py
```

That command builds the local Docker image for this benchmark and runs Roma,
DEAP, jMetalPy, jMetal Java, and pagmo2 inside the same container.