# Ackley Differential Evolution Benchmark

This benchmark compares Differential Evolution implementations on a harder
continuous Ackley instance. It complements the existing Rastrigin
hill-climbing benchmark by keeping the search space continuous and multimodal
while switching to a population-based optimizer.

## Layout

- `shared/`: common Ackley instance, budgets, seeds, and runner configuration.
- `runners/rust/`: benchmark-specific Roma source code.
- `runners/python/`: Python-native runners and wrappers for native backends.
- `runners/cpp/`: C++ source for the pagmo2 benchmark.
- `results/runs.csv`: unified per-run CSV output for the benchmark.
- `results/summary.json`: aggregate summary derived from the CSV.
- `results/latex/`: benchmark-local LaTeX tables for descriptive statistics.
- `../reports/latex/`: suite-level LaTeX tables for cross-benchmark analysis.

## Libraries

This benchmark is intended to compare:

- Roma (Rust)
- DEAP (Python)
- SciPy Differential Evolution (Python)
- pagmo2 (C++)
- Random Search baseline (Python)

The benchmark intentionally stays lean. The pinned `jmetalpy` version available
in this repository does not expose a directly comparable single-objective
Differential Evolution runner, and the random-search baseline is included to
show how much structure DE adds on a deceptive continuous landscape.

## Roma Source Of Truth

The Roma benchmark implementation lives in:

- `runners/rust/ackley_de_benchmark.rs`

The file `roma/examples/ackley_de_benchmark.rs` is only a thin bridge that
includes this benchmark-local source.

## Run

From the repository root:

```bash
/home/drlk/roma/.venv/bin/python benchmark_suite/ackley_de/orchestrate.py
```

That command builds the local Docker image for this benchmark and runs Roma,
DEAP, SciPy, the random-search baseline, and pagmo2 inside the same container.

After a successful run the orchestrator writes `results/runs.csv`, regenerates
`results/summary.json`, emits benchmark-local LaTeX tables in `results/latex/`,
and refreshes the suite-level statistical reports in
`benchmark_suite/reports/latex/` on the host.

For Friedman and Wilcoxon tests on the host-side suite reports, install the
analysis dependency with:

```bash
/home/drlk/roma/.venv/bin/pip install -r benchmark_suite/requirements-analysis.txt
```
