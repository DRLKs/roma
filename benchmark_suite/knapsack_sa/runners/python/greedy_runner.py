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
    PENALTY = 0.5
    weight = sum(int(bit) * float(w) for bit, w in zip(solution, WEIGHTS))
    value = sum(int(bit) * float(v) for bit, v in zip(solution, VALUES))
    if weight > CAPACITY:
        return -(weight - CAPACITY) * PENALTY
    return float(value)


def run_once(seed):
    # deterministic greedy: sort by value/weight
    items = list(enumerate(zip(WEIGHTS, VALUES)))
    items.sort(key=lambda iv: iv[1][1] / (iv[1][0] if iv[1][0] > 0 else 1e-9), reverse=True)
    remain = CAPACITY
    solution = [0] * NUM_ITEMS
    for idx, (w, v) in items:
        if w <= remain:
            solution[idx] = 1
            remain -= w

    fitness = compute_fitness(solution)
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
        "best_solution": [int(v) for v in solution],
        "wall_time_ms": 0.0,
        "cpu_time_ms": 0.0,
        "status": "ok",
        "error": None,
    }


def main():
    results = []
    for i in range(RUNS):
        seed = SEEDS[i]
        try:
            result = run_once(seed)
        except Exception as exc:
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
