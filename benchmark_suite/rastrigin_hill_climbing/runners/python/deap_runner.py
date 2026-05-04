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
DIMENSION = int(INSTANCE["dimension"])
LOWER_BOUND = float(INSTANCE["lower_bound"])
UPPER_BOUND = float(INSTANCE["upper_bound"])


if BUDGET.get("type") != "evaluations":
    raise ValueError("This DEAP benchmark runner currently supports only evaluation budgets")

if len(SEEDS) < RUNS:
    raise ValueError("config.json must define at least one seed per run")


if not hasattr(creator, "RastriginFitnessMin"):
    creator.create("RastriginFitnessMin", base.Fitness, weights=(-1.0,))

if not hasattr(creator, "RastriginIndividual"):
    creator.create("RastriginIndividual", list, fitness=creator.RastriginFitnessMin)


def rastrigin_value(variables):
    return 10.0 * len(variables) + sum(
        value * value - 10.0 * math.cos(2.0 * math.pi * value) for value in variables
    )


def evaluate(individual):
    return (rastrigin_value(individual),)


def bounded_gaussian_mutation(individual, sigma, indpb):
    tools.mutGaussian(individual, mu=0.0, sigma=sigma, indpb=indpb)
    for index, value in enumerate(individual):
        if value < LOWER_BOUND:
            individual[index] = LOWER_BOUND
        elif value > UPPER_BOUND:
            individual[index] = UPPER_BOUND
    return (individual,)


def build_toolbox():
    toolbox = base.Toolbox()
    toolbox.register("value", random.uniform, LOWER_BOUND, UPPER_BOUND)
    toolbox.register(
        "individual",
        tools.initRepeat,
        creator.RastriginIndividual,
        toolbox.value,
        DIMENSION,
    )
    toolbox.register("clone", copy.deepcopy)
    toolbox.register("evaluate", evaluate)
    toolbox.register(
        "mutate",
        bounded_gaussian_mutation,
        sigma=float(DEAP_CONFIG["sigma"]),
        indpb=float(DEAP_CONFIG["mutation_rate"]),
    )
    return toolbox


def run_benchmark(seed):
    random.seed(seed)
    toolbox = build_toolbox()
    current = toolbox.individual()
    current.fitness.values = toolbox.evaluate(current)
    best = toolbox.clone(current)
    best.fitness.values = current.fitness.values

    start_wall = time.perf_counter()

    for _ in range(int(BUDGET["value"])):
        candidate = toolbox.clone(current)
        candidate, = toolbox.mutate(candidate)
        candidate.fitness.values = toolbox.evaluate(candidate)

        if candidate.fitness.values[0] <= current.fitness.values[0]:
            current = candidate

        if current.fitness.values[0] <= best.fitness.values[0]:
            best = toolbox.clone(current)
            best.fitness.values = current.fitness.values

    end_wall = time.perf_counter()

    return {
        "benchmark_id": CONFIG["benchmark_id"],
        "library": "deap",
        "algorithm_family": CONFIG["algorithm_family"],
        "problem": INSTANCE["problem"],
        "instance_id": INSTANCE["instance_id"],
        "seed": seed,
        "budget_type": BUDGET["type"],
        "budget_value": int(BUDGET["value"]),
        "best_fitness": float(best.fitness.values[0]),
        "best_solution": list(best),
        "wall_time_ms": (end_wall - start_wall) * 1000.0,
        "cpu_time_ms": None,
        "status": "ok",
        "error": None,
    }


if __name__ == "__main__":
    results = [run_benchmark(SEEDS[index]) for index in range(RUNS)]
    print(json.dumps(results, indent=2))
