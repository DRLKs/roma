# Knapsack Binary PSO Benchmark

This benchmark compares Binary PSO implementations on a larger 0/1 knapsack
instance from the strongly-correlated family. It fills a real gap in the suite:
Roma already benchmarks hill climbing, simulated annealing, genetic
algorithms, differential evolution, and NSGA-II, but not particle swarm
optimization.

The instance is intentionally paired with two reference baselines:

- `pyswarms` as an established Python Binary PSO implementation.
- An exact dynamic-programming solver to expose the remaining optimality gap.

## Layout

- `shared/`: common knapsack instance, optimum, budgets, seeds, and runner configuration.
- `runners/rust/`: benchmark-specific Roma source code.
- `runners/python/`: Python-native runners for PySwarms and deterministic baselines.
- `results/runs.csv`: unified per-run CSV output for the benchmark.
- `results/summary.json`: aggregate summary derived from the CSV.
- `results/latex/`: benchmark-local LaTeX tables for descriptive statistics.
- `../reports/latex/`: suite-level LaTeX tables for cross-benchmark analysis.

## Libraries

This benchmark is intended to compare:

- Roma (Rust)
- PySwarms (Python)
- Exact Dynamic Programming baseline (Python)
- Greedy ratio baseline (Python)

The exact solver is not a metaheuristic, but it is included on purpose because
Binary PSO is much more informative when the optimality gap is observable on a
discrete instance with a known optimum.

## Roma Source Of Truth

The Roma benchmark implementation lives in:

- `runners/rust/knapsack_pso_benchmark.rs`

The file `roma/examples/knapsack_pso_benchmark.rs` is only a thin bridge that
includes this benchmark-local source.

## Run

From the repository root:

```bash
/home/drlk/roma/.venv/bin/python benchmark_suite/knapsack_pso/orchestrate.py
```

That command builds the local Docker image for this benchmark and runs Roma,
PySwarms, the exact solver, and the greedy baseline inside the same container.

After a successful run the orchestrator writes `results/runs.csv`, regenerates
`results/summary.json`, emits benchmark-local LaTeX tables in `results/latex/`,
and refreshes the suite-level statistical reports in
`benchmark_suite/reports/latex/` on the host.

For Friedman and Wilcoxon tests on the host-side suite reports, install the
analysis dependency with:

```bash
/home/drlk/roma/.venv/bin/pip install -r benchmark_suite/requirements-analysis.txt
```