import json
import math
import random
import time
from pathlib import Path

import numpy as np
from mealpy import FloatVar, SA


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
MEALPY_CONFIG = CONFIG["mealpy"]
DIMENSION = int(INSTANCE["dimension"])
LOWER_BOUND = float(INSTANCE["lower_bound"])
UPPER_BOUND = float(INSTANCE["upper_bound"])

if BUDGET.get("type") != "evaluations":
    raise ValueError("This mealpy benchmark runner currently supports only evaluation budgets")

if len(SEEDS) < RUNS:
    raise ValueError("config.json must define at least one seed per run")


def rastrigin(solution):
    n = len(solution)
    return 10.0 * n + sum(
        x * x - 10.0 * math.cos(2.0 * math.pi * x) for x in solution
    )


def run_benchmark(seed):
    random.seed(seed)
    np.random.seed(seed)

    budget_value = int(BUDGET["value"])
    pop_size = int(MEALPY_CONFIG.get("pop_size", 2))
    # Each epoch evaluates pop_size solutions; approximate the evaluation budget.
    epoch = max(1, budget_value // pop_size)

    problem = {
        "obj_func": rastrigin,
        "bounds": FloatVar(
            lb=[LOWER_BOUND] * DIMENSION,
            ub=[UPPER_BOUND] * DIMENSION,
        ),
        "minmax": "min",
        "log_to": None,
    }

    model = SA.OriginalSA(
        epoch=epoch,
        pop_size=pop_size,
        temp_init=float(MEALPY_CONFIG["temp_init"]),
        step_size=float(MEALPY_CONFIG["step_size"]),
    )

    start_wall = time.perf_counter()
    model.solve(problem, seed=seed, mode="single")
    end_wall = time.perf_counter()

    best = model.g_best
    best_solution = best.solution.tolist()
    best_fitness = float(best.target.fitness)

    return {
        "benchmark_id": CONFIG["benchmark_id"],
        "library": "mealpy",
        "algorithm_family": CONFIG["algorithm_family"],
        "problem": INSTANCE["problem"],
        "instance_id": INSTANCE["instance_id"],
        "seed": seed,
        "budget_type": BUDGET["type"],
        "budget_value": budget_value,
        "best_fitness": best_fitness,
        "best_solution": best_solution,
        "wall_time_ms": (end_wall - start_wall) * 1000.0,
        "cpu_time_ms": None,
        "status": "ok",
        "error": None,
    }


def main():
    results = []
    for index in range(RUNS):
        seed = SEEDS[index]
        try:
            result = run_benchmark(seed)
        except Exception as exc:  # noqa: BLE001
            result = {
                "benchmark_id": CONFIG["benchmark_id"],
                "library": "mealpy",
                "algorithm_family": CONFIG["algorithm_family"],
                "problem": INSTANCE["problem"],
                "instance_id": INSTANCE["instance_id"],
                "seed": seed,
                "budget_type": BUDGET["type"],
                "budget_value": int(BUDGET["value"]),
                "best_fitness": None,
                "best_solution": None,
                "wall_time_ms": None,
                "cpu_time_ms": None,
                "status": "error",
                "error": str(exc),
            }
        results.append(result)

    print(json.dumps(results))


if __name__ == "__main__":
    main()
