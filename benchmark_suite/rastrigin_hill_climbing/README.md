# Rastrigin Hill Climbing Benchmark

This benchmark is self-contained and organized by role and language so each implementation lives next to the rest of the benchmark assets.

## Layout

- `shared/`: common benchmark instance, budgets, and seeds.
- `runners/rust/`: benchmark-specific Roma source code.
- `runners/python/`: Python wrappers and Python-native baselines.
- `runners/java/`: Java source for the jMetal benchmark.
- `runners/cpp/`: C++ source for the pagmo2 benchmark.
- `results/runs.csv`: unified per-run CSV output for the benchmark.
- `results/summary.json`: aggregate summary derived from the CSV.
- `results/latex/`: benchmark-local LaTeX tables for descriptive statistics.
- `../reports/latex/`: suite-level LaTeX tables for cross-benchmark statistical analysis.

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

After a successful run the orchestrator writes `results/runs.csv`, regenerates `results/summary.json`, emits benchmark-local LaTeX tables in `results/latex/`, and refreshes the suite-level statistical reports in `benchmark_suite/reports/latex/` on the host.

Note: mealpy is executed from a dedicated virtual environment inside the image because it requires a different NumPy major version than jMetalPy.

For Friedman and Wilcoxon tests on the host-side suite reports, install the analysis dependency with:

```bash
/home/drlk/roma/.venv/bin/pip install -r benchmark_suite/requirements-analysis.txt
```
