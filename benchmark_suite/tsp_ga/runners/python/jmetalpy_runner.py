import json
import logging
import random
import time
from pathlib import Path

import numpy as np
from jmetal.algorithm.singleobjective.genetic_algorithm import GeneticAlgorithm
from jmetal.operator.crossover import PMXCrossover
from jmetal.operator.mutation import PermutationSwapMutation
from jmetal.operator.selection import BinaryTournamentSelection
from jmetal.problem.singleobjective.tsp import TSP
from jmetal.util.termination_criterion import StoppingByEvaluations, StoppingByTime


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
JMETAL_CONFIG = CONFIG["jmetalpy"]
TSPLIB_PATH = SHARED_DIR / INSTANCE["tsplib_file"]


if BUDGET.get("type") not in {"evaluations", "time"}:
    raise ValueError("This jMetalPy benchmark runner supports only evaluation or time budgets")

if len(SEEDS) < RUNS:
    raise ValueError("config.json must define at least one seed per run")


logging.getLogger("jmetal").setLevel(logging.CRITICAL)


def run_benchmark(seed):
    random.seed(seed)
    np.random.seed(seed)

    problem = TSP(instance=str(TSPLIB_PATH))
    if BUDGET["type"] == "evaluations":
        termination = StoppingByEvaluations(int(BUDGET["value"]))
    else:
        termination = StoppingByTime(float(BUDGET["value"]))

    algorithm = GeneticAlgorithm(
        problem=problem,
        population_size=int(JMETAL_CONFIG["population_size"]),
        offspring_population_size=int(JMETAL_CONFIG["offspring_population_size"]),
        mutation=PermutationSwapMutation(probability=float(JMETAL_CONFIG["mutation_probability"])),
        crossover=PMXCrossover(probability=float(JMETAL_CONFIG["crossover_probability"])),
        selection=BinaryTournamentSelection(),
        termination_criterion=termination,
    )

    start_wall = time.perf_counter()
    algorithm.run()
    end_wall = time.perf_counter()

    result = algorithm.result()

    return {
        "benchmark_id": CONFIG["benchmark_id"],
        "library": "jmetalpy",
        "algorithm_family": CONFIG["algorithm_family"],
        "problem": INSTANCE["problem"],
        "instance_id": INSTANCE["instance_id"],
        "seed": seed,
        "budget_type": BUDGET["type"],
        "budget_value": int(BUDGET["value"]),
        "best_fitness": float(result.objectives[0]),
        "best_solution": [int(value) for value in result.variables],
        "wall_time_ms": (end_wall - start_wall) * 1000.0,
        "cpu_time_ms": None,
        "status": "ok",
        "error": None,
    }


if __name__ == "__main__":
    results = [run_benchmark(SEEDS[index]) for index in range(RUNS)]
    print(json.dumps(results, indent=2))