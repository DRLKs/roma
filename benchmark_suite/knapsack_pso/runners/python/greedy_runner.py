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
WEIGHTS = INSTANCE["weights"]
VALUES = INSTANCE["values"]
CAPACITY = float(INSTANCE["capacity"])
NUM_ITEMS = len(WEIGHTS)


def compute_fitness(solution):
    penalty = 0.5
    weight = sum(int(bit) * float(weight) for bit, weight in zip(solution, WEIGHTS))
    value = sum(int(bit) * float(value) for bit, value in zip(solution, VALUES))
    if weight > CAPACITY:
        return -(weight - CAPACITY) * penalty
    return float(value)


def run_once(seed):
    start_wall = time.perf_counter()
    start_cpu = time.process_time()

    items = list(enumerate(zip(WEIGHTS, VALUES)))
    items.sort(key=lambda iv: iv[1][1] / (iv[1][0] if iv[1][0] > 0 else 1e-9), reverse=True)

    remain = CAPACITY
    solution = [0] * NUM_ITEMS
    for index, (weight, _) in items:
        if weight <= remain:
            solution[index] = 1
            remain -= weight

    fitness = compute_fitness(solution)
    end_cpu = time.process_time()
    end_wall = time.perf_counter()

    return {
        "benchmark_id": CONFIG["benchmark_id"],
        "library": "greedy",
        "algorithm_family": "greedy",
        "problem": INSTANCE["problem"],
        "instance_id": INSTANCE["instance_id"],
        "seed": seed,
        "budget_type": BUDGET["type"],
        "budget_value": int(BUDGET["value"]),
        "best_fitness": fitness,
        "best_solution": [int(value) for value in solution],
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
                "library": "greedy",
                "algorithm_family": "greedy",
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