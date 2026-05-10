# Knapsack Simulated Annealing Benchmark

Benchmark that compares Roma's Simulated Annealing implementation on a
knapsack instance with other Python baselines (DEAP GA and a simple greedy
baseline). The goal is to add an algorithm not yet used in the suite
(Simulated Annealing) and compare Roma against other common libraries.

Run:

```bash
/home/drlk/roma/.venv/bin/python benchmark_suite/knapsack_sa/orchestrate.py
```

The orchestrator builds a Docker image and runs the different runners
inside a controlled environment. Results are saved in `results/`.
