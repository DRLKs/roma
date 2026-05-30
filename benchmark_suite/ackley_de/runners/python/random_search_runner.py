import json
import math
import random
import time
from pathlib import Path


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
DIMENSION = int(INSTANCE["dimension"])
LOWER_BOUND = float(INSTANCE["lower_bound"])
UPPER_BOUND = float(INSTANCE["upper_bound"])


if BUDGET.get("type") != "evaluations":
    raise ValueError("This random-search benchmark runner currently supports only evaluation budgets")

if len(SEEDS) < RUNS:
    raise ValueError("config.json must define at least one seed per run")


def ackley_value(variables):
    dimension = float(len(variables))
    squared_mean = sum(value * value for value in variables) / dimension
    cosine_mean = sum(math.cos(2.0 * math.pi * value) for value in variables) / dimension
    return -20.0 * math.exp(-0.2 * math.sqrt(squared_mean)) - math.exp(cosine_mean) + 20.0 + math.e


def sample_solution(rng):
    return [rng.uniform(LOWER_BOUND, UPPER_BOUND) for _ in range(DIMENSION)]


def run_benchmark(seed):
    rng = random.Random(seed)
    budget_value = int(BUDGET["value"])
    if budget_value <= 0:
        raise ValueError("Evaluation budget must be positive")

    start_wall = time.perf_counter()
    start_cpu = time.process_time()

    best_solution = None
    best_fitness = None
    for _ in range(budget_value):
        candidate = sample_solution(rng)
        fitness = ackley_value(candidate)
        if best_fitness is None or fitness < best_fitness:
            best_solution = candidate
            best_fitness = fitness

    end_cpu = time.process_time()
    end_wall = time.perf_counter()

    return {
        "benchmark_id": CONFIG["benchmark_id"],
        "library": "random_search",
        "algorithm_family": "baseline_random_search",
        "problem": INSTANCE["problem"],
        "instance_id": INSTANCE["instance_id"],
        "seed": seed,
        "budget_type": BUDGET["type"],
        "budget_value": budget_value,
        "best_fitness": float(best_fitness),
        "best_solution": [float(value) for value in best_solution],
        "wall_time_ms": (end_wall - start_wall) * 1000.0,
        "cpu_time_ms": (end_cpu - start_cpu) * 1000.0,
        "evaluations": budget_value,
        "status": "ok",
        "error": None,
    }


def main():
    results = [run_benchmark(SEEDS[index]) for index in range(RUNS)]
    print(json.dumps(results, indent=2))


if __name__ == "__main__":
    main()
