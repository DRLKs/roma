<p align="center">
  <a href="https://www.rust-lang.org/"><img src="https://img.shields.io/badge/Made%20with-Rust-black?style=for-the-badge&logo=rust" alt="Made with Rust"></a>
  <a href="https://crates.io/crates/roma_lib"><img src="https://img.shields.io/badge/crates.io-roma-orange?style=for-the-badge&logo=rust" alt="Crates.io"></a>
  <a href="https://docs.rs/roma_lib"><img src="https://img.shields.io/badge/docs.rs-roma-blue?style=for-the-badge&logo=readthedocs" alt="docs.rs"></a>
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
roma = "0.1.1"
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




drlk ~/roma ❯ .venv/bin/python benchmark_suite/tsp_ga/orchestrate.py
[tsp_ga] library=roma status=ok elapsed_s=140.02 completed_runs=28
[tsp_ga] library=deap status=ok elapsed_s=140.27 completed_runs=28
[tsp_ga] library=jmetalpy status=ok elapsed_s=140.62 completed_runs=28
[tsp_ga] library=mealpy status=ok elapsed_s=141.59 completed_runs=28
[tsp_ga] library=jmetal_java status=ok elapsed_s=144.34 completed_runs=28
[tsp_ga] library=pagmo2_cpp status=ok elapsed_s=140.57 completed_runs=28
{
  "benchmark_id": "tsp_ga",
  "executed_at_utc": "20260525T203103Z",
  "environment": {
    "orchestrator_python": "3.12.3",
    "platform": "Linux-6.19.11-arch1-1-x86_64-with-glibc2.39",
    "in_container": true,
    "docker_image": "roma-tsp-ga:latest"
  },
  "instance": {
    "problem": "tsp",
    "instance_id": "tsp_48_clustered_euc2d",
    "dimension": 48,
    "close_tour": true,
    "tsplib_file": "instance.tsp"
  },
  "budget": {
    "type": "time",
    "value": 5
  },
  "runs": 28,
  "seeds": [
    100,
    101,
    102,
    103,
    104,
    105,
    106,
    107,
    108,
    109,
    110,
    111,
    112,
    113,
    114,
    115,
    116,
    117,
    118,
    119,
    120,
    121,
    122,
    123,
    124,
    125,
    126,
    127
  ],
  "libraries": {
    "roma": {
      "status": "ok",
      "execution_mode": "container",
      "runner_command": "/usr/local/bin/roma_tsp_ga_benchmark",
      "runner_wall_time_ms": 140016.35641600023,
      "completed_runs": 28,
      "raw_output": "results/raw/20260525T203103Z_roma.json",
      "aggregate": {
        "runs": 28,
        "ok_runs": 28,
        "failed_runs": 0,
        "error_runs": 0,
        "skipped_runs": 0,
        "best_fitness": 1089,
        "mean_fitness": 1308.4642857142858,
        "median_fitness": 1298.5,
        "worst_fitness": 1541,
        "stddev_fitness": 117.55408546794145,
        "p90_fitness": 1453.2,
        "mean_wall_time_ms": 5000.274698428571,
        "median_wall_time_ms": 5000.2344885,
        "p90_wall_time_ms": 5000.4870913,
        "mean_cpu_time_ms": 4930.772510071429
      },
      "validation": {
        "valid": true,
        "invalid_runs": 0,
        "invalid_routes": 0,
        "max_abs_fitness_error": 0.0,
        "errors": []
      }
    },
    "deap": {
      "status": "ok",
      "execution_mode": "container",
      "runner_command": "/usr/bin/python3 /workspace/benchmark_suite/tsp_ga/runners/python/deap_runner.py",
      "runner_wall_time_ms": 140268.7318219996,
      "completed_runs": 28,
      "raw_output": "results/raw/20260525T203103Z_deap.json",
      "aggregate": {
        "runs": 28,
        "ok_runs": 28,
        "failed_runs": 0,
        "error_runs": 0,
        "skipped_runs": 0,
        "best_fitness": 1247.0,
        "mean_fitness": 1452.5,
        "median_fitness": 1440.0,
        "worst_fitness": 1738.0,
        "stddev_fitness": 134.2573488278601,
        "p90_fitness": 1647.1000000000001,
        "mean_wall_time_ms": 5001.217131857207,
        "median_wall_time_ms": 5001.137998000559,
        "p90_wall_time_ms": 5002.165598000374,
        "mean_cpu_time_ms": 4984.100793678573
      },
      "validation": {
        "valid": true,
        "invalid_runs": 0,
        "invalid_routes": 0,
        "max_abs_fitness_error": 0.0,
        "errors": []
      }
    },
    "jmetalpy": {
      "status": "ok",
      "execution_mode": "container",
      "runner_command": "/usr/bin/python3 /workspace/benchmark_suite/tsp_ga/runners/python/jmetalpy_runner.py",
      "runner_wall_time_ms": 140621.38639400018,
      "completed_runs": 28,
      "raw_output": "results/raw/20260525T203103Z_jmetalpy.json",
      "aggregate": {
        "runs": 28,
        "ok_runs": 28,
        "failed_runs": 0,
        "error_runs": 0,
        "skipped_runs": 0,
        "best_fitness": 2652.0,
        "mean_fitness": 3310.214285714286,
        "median_fitness": 3276.5,
        "worst_fitness": 3965.0,
        "stddev_fitness": 314.0901095844559,
        "p90_fitness": 3731.3,
        "mean_wall_time_ms": 5002.154403428546,
        "median_wall_time_ms": 5001.8863159994,
        "p90_wall_time_ms": 5004.390569900079,
        "mean_cpu_time_ms": null
      },
      "validation": {
        "valid": true,
        "invalid_runs": 0,
        "invalid_routes": 0,
        "max_abs_fitness_error": 0.0,
        "errors": []
      }
    },
    "mealpy": {
      "status": "ok",
      "execution_mode": "container",
      "runner_command": "/opt/mealpy-venv/bin/python /workspace/benchmark_suite/tsp_ga/runners/python/mealpy_runner.py",
      "runner_wall_time_ms": 141592.32400400014,
      "completed_runs": 28,
      "raw_output": "results/raw/20260525T203103Z_mealpy.json",
      "aggregate": {
        "runs": 28,
        "ok_runs": 28,
        "failed_runs": 0,
        "error_runs": 0,
        "skipped_runs": 0,
        "best_fitness": 4801.0,
        "mean_fitness": 5181.75,
        "median_fitness": 5198.0,
        "worst_fitness": 5493.0,
        "stddev_fitness": 187.0610337754575,
        "p90_fitness": 5435.6,
        "mean_wall_time_ms": 5002.112158250059,
        "median_wall_time_ms": 5001.918902500165,
        "p90_wall_time_ms": 5003.625583500161,
        "mean_cpu_time_ms": null
      },
      "validation": {
        "valid": true,
        "invalid_runs": 0,
        "invalid_routes": 0,
        "max_abs_fitness_error": 0.0,
        "errors": []
      }
    },
    "jmetal_java": {
      "status": "ok",
      "execution_mode": "container",
      "runner_command": "/usr/bin/python3 /workspace/benchmark_suite/tsp_ga/runners/python/jmetal_java_runner.py",
      "runner_wall_time_ms": 144342.86368300035,
      "completed_runs": 28,
      "raw_output": "results/raw/20260525T203103Z_jmetal_java.json",
      "aggregate": {
        "runs": 28,
        "ok_runs": 28,
        "failed_runs": 0,
        "error_runs": 0,
        "skipped_runs": 0,
        "best_fitness": 1309.0,
        "mean_fitness": 1738.5357142857142,
        "median_fitness": 1722.5,
        "worst_fitness": 2251.0,
        "stddev_fitness": 211.63419959632117,
        "p90_fitness": 2017.7,
        "mean_wall_time_ms": 4992.556234178571,
        "median_wall_time_ms": 4993.1164069999995,
        "p90_wall_time_ms": 4993.2908946,
        "mean_cpu_time_ms": 4901.365433
      },
      "validation": {
        "valid": true,
        "invalid_runs": 0,
        "invalid_routes": 0,
        "max_abs_fitness_error": 0.0,
        "errors": []
      }
    },
    "pagmo2_cpp": {
      "status": "ok",
      "execution_mode": "container",
      "runner_command": "/usr/bin/python3 /workspace/benchmark_suite/tsp_ga/runners/python/pagmo_cpp_runner.py",
      "runner_wall_time_ms": 140567.95834500008,
      "completed_runs": 28,
      "raw_output": "results/raw/20260525T203103Z_pagmo2_cpp.json",
      "aggregate": {
        "runs": 28,
        "ok_runs": 28,
        "failed_runs": 0,
        "error_runs": 0,
        "skipped_runs": 0,
        "best_fitness": 1022,
        "mean_fitness": 1267.142857142857,
        "median_fitness": 1251.0,
        "worst_fitness": 1644,
        "stddev_fitness": 153.4658441025965,
        "p90_fitness": 1437.1000000000001,
        "mean_wall_time_ms": 5000.053186428571,
        "median_wall_time_ms": 5000.045556499999,
        "p90_wall_time_ms": 5000.096836199999,
        "mean_cpu_time_ms": 4986.762964285715
      },
      "validation": {
        "valid": true,
        "invalid_runs": 0,
        "invalid_routes": 0,
        "max_abs_fitness_error": 0.0,
        "errors": []
      }
    }
  },
  "total_wall_time_ms": 847425.675556
}
drlk ~/roma ❯