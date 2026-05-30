import json
import math
import time
from pathlib import Path

import numpy as np
from scipy.optimize import differential_evolution


ROOT = Path(__file__).resolve().parents[2]
SHARED_DIR = ROOT / "shared"
INSTANCE_PATH = SHARED_DIR / "instance.json"
CONFIG_PATH = SHARED_DIR / "config.json"


def load_json(path):
    with path.open("r", encoding="utf-8") as handle:
        return json.load(handle)


INSTANCE = load_json(INSTANCE_PATH)
CONFIG = load_json(CONFIG_PATH)
BUDGET = CONFIG["budget"]
RUNS = int(CONFIG["runs"])
SEEDS = list(CONFIG.get("seeds", []))
SCIPY_CONFIG = CONFIG["scipy"]
DIMENSION = int(INSTANCE["dimension"])
LOWER_BOUND = float(INSTANCE["lower_bound"])
UPPER_BOUND = float(INSTANCE["upper_bound"])


if BUDGET.get("type") != "evaluations":
    raise ValueError("This SciPy benchmark runner currently supports only evaluation budgets")

if len(SEEDS) < RUNS:
    raise ValueError("config.json must define at least one seed per run")


def ackley_value(variables):
    dimension = float(len(variables))
    squared_mean = sum(value * value for value in variables) / dimension
    cosine_mean = sum(math.cos(2.0 * math.pi * value) for value in variables) / dimension
    return -20.0 * math.exp(-0.2 * math.sqrt(squared_mean)) - math.exp(cosine_mean) + 20.0 + math.e


def run_benchmark(seed):
    budget_value = int(BUDGET["value"])
    population_multiplier = int(SCIPY_CONFIG["population_multiplier"])
    effective_population = population_multiplier * DIMENSION

    if population_multiplier <= 0:
        raise ValueError("scipy.population_multiplier must be positive")

    if budget_value < effective_population:
        raise ValueError("Evaluation budget must be at least the effective SciPy population size")

    maxiter = max(0, budget_value // effective_population - 1)
    bounds = [(LOWER_BOUND, UPPER_BOUND)] * DIMENSION

    start_wall = time.perf_counter()
    start_cpu = time.process_time()
    result = differential_evolution(
        ackley_value,
        bounds,
        strategy=str(SCIPY_CONFIG["strategy"]),
        maxiter=maxiter,
        popsize=population_multiplier,
        mutation=(float(SCIPY_CONFIG["mutation_min"]), float(SCIPY_CONFIG["mutation_max"])),
        recombination=float(SCIPY_CONFIG["recombination"]),
        polish=False,
        rng=np.random.default_rng(seed),
        updating="deferred",
        workers=1,
    )
    end_cpu = time.process_time()
    end_wall = time.perf_counter()

    return {
        "benchmark_id": CONFIG["benchmark_id"],
        "library": "scipy",
        "algorithm_family": CONFIG["algorithm_family"],
        "problem": INSTANCE["problem"],
        "instance_id": INSTANCE["instance_id"],
        "seed": seed,
        "budget_type": BUDGET["type"],
        "budget_value": budget_value,
        "best_fitness": float(result.fun),
        "best_solution": [float(value) for value in result.x.tolist()],
        "wall_time_ms": (end_wall - start_wall) * 1000.0,
        "cpu_time_ms": (end_cpu - start_cpu) * 1000.0,
        "evaluations": int(result.nfev),
        "status": "ok",
        "error": None,
    }


if __name__ == "__main__":
    results = [run_benchmark(SEEDS[index]) for index in range(RUNS)]
    print(json.dumps(results, indent=2))
