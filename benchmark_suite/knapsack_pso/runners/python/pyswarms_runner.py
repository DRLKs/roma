import json
import os
import random
import time
from pathlib import Path

import numpy as np


ROOT = Path(__file__).resolve().parents[2]
SHARED_DIR = ROOT / "shared"
RESULTS_DIR = ROOT / "results"
INSTANCE_PATH = SHARED_DIR / "instance.json"
CONFIG_PATH = SHARED_DIR / "config.json"

RESULTS_DIR.mkdir(parents=True, exist_ok=True)
os.chdir(RESULTS_DIR)

from pyswarms.discrete.binary import BinaryPSO


def load_json(path):
    with path.open("r", encoding="utf-8") as handle:
        return json.load(handle)


INSTANCE = load_json(INSTANCE_PATH)
CONFIG = load_json(CONFIG_PATH)
BUDGET = CONFIG["budget"]
RUNS = int(CONFIG["runs"])
SEEDS = list(CONFIG.get("seeds", []))
PY_SWARMS_CONFIG = CONFIG["pyswarms"]
WEIGHTS = np.array(INSTANCE["weights"], dtype=float)
VALUES = np.array(INSTANCE["values"], dtype=float)
CAPACITY = float(INSTANCE["capacity"])
NUM_ITEMS = len(WEIGHTS)


if BUDGET.get("type") != "evaluations":
    raise ValueError("This PySwarms benchmark runner currently supports only evaluation budgets")

if len(SEEDS) < RUNS:
    raise ValueError("config.json must define at least one seed per run")


def compute_fitness(solution):
    penalty = 0.5
    weight = float(np.dot(solution, WEIGHTS))
    value = float(np.dot(solution, VALUES))
    if weight > CAPACITY:
        return -(weight - CAPACITY) * penalty
    return value


def objective(particles):
    weights = particles @ WEIGHTS
    values = particles @ VALUES
    penalties = np.maximum(weights - CAPACITY, 0.0) * 0.5
    fitness = np.where(weights > CAPACITY, -penalties, values)
    return -fitness


def run_benchmark(seed):
    random.seed(seed)
    np.random.seed(seed)

    swarm_size = int(PY_SWARMS_CONFIG["swarm_size"])
    budget_value = int(BUDGET["value"])
    if budget_value < swarm_size:
        raise ValueError("Evaluation budget must be at least the swarm size")
    if budget_value % swarm_size != 0:
        raise ValueError("Evaluation budget must be divisible by the PySwarms swarm size")

    iterations = budget_value // swarm_size - 1
    options = {
        "c1": float(PY_SWARMS_CONFIG["cognitive_coefficient"]),
        "c2": float(PY_SWARMS_CONFIG["social_coefficient"]),
        "w": float(PY_SWARMS_CONFIG["inertia_weight"]),
        "k": int(PY_SWARMS_CONFIG["neighborhood_size"]),
        "p": int(PY_SWARMS_CONFIG["minkowski_p"]),
    }
    velocity_clamp = float(PY_SWARMS_CONFIG["velocity_clamp"])
    init_pos = np.random.randint(0, 2, size=(swarm_size, NUM_ITEMS))

    start_wall = time.perf_counter()
    start_cpu = time.process_time()
    optimizer = BinaryPSO(
        n_particles=swarm_size,
        dimensions=NUM_ITEMS,
        options=options,
        init_pos=init_pos,
        velocity_clamp=(-velocity_clamp, velocity_clamp),
        ftol=float("-inf"),
    )
    cost, position = optimizer.optimize(objective, iters=iterations, verbose=False)
    end_cpu = time.process_time()
    end_wall = time.perf_counter()

    best_solution = [int(value) for value in np.rint(position).astype(int).tolist()]
    best_fitness = compute_fitness(np.array(best_solution, dtype=float))

    return {
        "benchmark_id": CONFIG["benchmark_id"],
        "library": "pyswarms",
        "algorithm_family": CONFIG["algorithm_family"],
        "problem": INSTANCE["problem"],
        "instance_id": INSTANCE["instance_id"],
        "seed": seed,
        "budget_type": BUDGET["type"],
        "budget_value": budget_value,
        "best_fitness": float(best_fitness),
        "best_solution": best_solution,
        "wall_time_ms": (end_wall - start_wall) * 1000.0,
        "cpu_time_ms": (end_cpu - start_cpu) * 1000.0,
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
                "library": "pyswarms",
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

    print(json.dumps(results, indent=2))


if __name__ == "__main__":
    main()