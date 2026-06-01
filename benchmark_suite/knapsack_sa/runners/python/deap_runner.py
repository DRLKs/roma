import copy
import json
import math
import random
import time
from pathlib import Path

from deap import base, creator, tools


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
DEAP_CONFIG = CONFIG["deap"]
WEIGHTS = INSTANCE["weights"]
VALUES = INSTANCE["values"]
CAPACITY = float(INSTANCE["capacity"])
NUM_ITEMS = len(WEIGHTS)


if BUDGET.get("type") != "evaluations":
    raise ValueError("This DEAP benchmark runner currently supports only evaluation budgets")

if len(SEEDS) < RUNS:
    raise ValueError("config.json must define at least one seed per run")


if not hasattr(creator, "KnapsackFitnessMax"):
    creator.create("KnapsackFitnessMax", base.Fitness, weights=(1.0,))

if not hasattr(creator, "KnapsackIndividual"):
    creator.create("KnapsackIndividual", list, fitness=creator.KnapsackFitnessMax)


PENALTY = 0.5


def evaluate(individual):
    weight = sum(int(bit) * float(w) for bit, w in zip(individual, WEIGHTS))
    value = sum(int(bit) * float(v) for bit, v in zip(individual, VALUES))
    if weight > CAPACITY:
        return (-(weight - CAPACITY) * PENALTY,)
    return (float(value),)


def bit_flip_neighbor(individual):
    """Generate a neighbor by flipping exactly one random bit (Hamming-1 neighborhood)."""
    neighbor = copy.deepcopy(individual)
    index = random.randint(0, len(neighbor) - 1)
    neighbor[index] = 1 - neighbor[index]
    return neighbor


def run_benchmark(seed):
    random.seed(seed)
    budget_value = int(BUDGET["value"])
    initial_temperature = float(DEAP_CONFIG["initial_temperature"])
    cooling_rate = float(DEAP_CONFIG["cooling_rate"])

    # Create initial solution
    current = creator.KnapsackIndividual([random.randint(0, 1) for _ in range(NUM_ITEMS)])
    current.fitness.values = evaluate(current)
    best = copy.deepcopy(current)
    best.fitness.values = current.fitness.values

    temperature = initial_temperature

    start_wall = time.perf_counter()
    start_cpu = time.process_time()

    for _ in range(budget_value):
        candidate = bit_flip_neighbor(current)
        candidate = creator.KnapsackIndividual(candidate)
        candidate.fitness.values = evaluate(candidate)

        delta = candidate.fitness.values[0] - current.fitness.values[0]

        if delta >= 0:
            current = candidate
        elif temperature > 0:
            acceptance = math.exp(delta / temperature)
            if random.random() < acceptance:
                current = candidate

        if current.fitness.values[0] > best.fitness.values[0]:
            best = copy.deepcopy(current)
            best.fitness.values = current.fitness.values

        temperature *= cooling_rate

    end_cpu = time.process_time()
    end_wall = time.perf_counter()

    return {
        "benchmark_id": CONFIG["benchmark_id"],
        "library": "deap",
        "algorithm_family": CONFIG["algorithm_family"],
        "problem": INSTANCE["problem"],
        "instance_id": INSTANCE["instance_id"],
        "seed": seed,
        "budget_type": BUDGET["type"],
        "budget_value": budget_value,
        "best_fitness": float(best.fitness.values[0]),
        "best_solution": [int(value) for value in best],
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
                "library": "deap",
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
