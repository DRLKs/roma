import json
import logging
import random
import time
from pathlib import Path

import numpy as np
from jmetal.algorithm.singleobjective.local_search import LocalSearch
from jmetal.operator.mutation import PolynomialMutation
from jmetal.problem.singleobjective.unconstrained import Rastrigin
from jmetal.util.termination_criterion import StoppingByEvaluations


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
JMETAL_CONFIG = CONFIG.get("jmetalpy", CONFIG["jmetal_java"])
DIMENSION = int(INSTANCE["dimension"])


if BUDGET.get("type") != "evaluations":
    raise ValueError("This jMetalPy benchmark runner currently supports only evaluation budgets")

if len(SEEDS) < RUNS:
    raise ValueError("config.json must define at least one seed per run")


logging.getLogger("jmetal").setLevel(logging.CRITICAL)


def run_benchmark(seed):
    random.seed(seed)
    np.random.seed(seed)

    problem = Rastrigin(DIMENSION)
    # Probability 1.0 ensures ALL variables are perturbed (neighborhood semantics).
    mutation = PolynomialMutation(
        probability=1.0,
        distribution_index=float(JMETAL_CONFIG["distribution_index"]),
    )
    algorithm = LocalSearch(
        problem=problem,
        mutation=mutation,
        termination_criterion=StoppingByEvaluations(int(BUDGET["value"])),
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
        "best_solution": list(result.variables),
        "wall_time_ms": (end_wall - start_wall) * 1000.0,
        "cpu_time_ms": None,
        "status": "ok",
        "error": None,
    }


if __name__ == "__main__":
    results = [run_benchmark(SEEDS[index]) for index in range(RUNS)]
    print(json.dumps(results, indent=2))
