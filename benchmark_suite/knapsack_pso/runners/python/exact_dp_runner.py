import json
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
WEIGHTS = [int(weight) for weight in INSTANCE["weights"]]
VALUES = [int(value) for value in INSTANCE["values"]]
CAPACITY = int(INSTANCE["capacity"])
KNOWN_OPTIMUM = int(INSTANCE["known_optimum"])


def solve_exact_knapsack():
    dp = [0] * (CAPACITY + 1)
    keep = [[False] * (CAPACITY + 1) for _ in range(len(WEIGHTS))]

    for item_index, (weight, value) in enumerate(zip(WEIGHTS, VALUES)):
        for capacity in range(CAPACITY, weight - 1, -1):
            candidate = dp[capacity - weight] + value
            if candidate > dp[capacity]:
                dp[capacity] = candidate
                keep[item_index][capacity] = True

    solution = [0] * len(WEIGHTS)
    capacity = CAPACITY
    for item_index in range(len(WEIGHTS) - 1, -1, -1):
        if keep[item_index][capacity]:
            solution[item_index] = 1
            capacity -= WEIGHTS[item_index]

    return max(dp), solution


def run_once(seed):
    start_wall = time.perf_counter()
    start_cpu = time.process_time()
    best_fitness, best_solution = solve_exact_knapsack()
    end_cpu = time.process_time()
    end_wall = time.perf_counter()

    if best_fitness != KNOWN_OPTIMUM:
        raise ValueError(
            f"exact DP recomputed optimum {best_fitness} but instance declares {KNOWN_OPTIMUM}"
        )

    return {
        "benchmark_id": CONFIG["benchmark_id"],
        "library": "exact_dp",
        "algorithm_family": "exact_dynamic_programming",
        "problem": INSTANCE["problem"],
        "instance_id": INSTANCE["instance_id"],
        "seed": seed,
        "budget_type": BUDGET["type"],
        "budget_value": int(BUDGET["value"]),
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
            result = run_once(seed)
        except Exception as exc:  # noqa: BLE001
            result = {
                "benchmark_id": CONFIG["benchmark_id"],
                "library": "exact_dp",
                "algorithm_family": "exact_dynamic_programming",
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