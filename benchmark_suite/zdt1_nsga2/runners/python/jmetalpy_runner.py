import json
import logging
import random
import time
from pathlib import Path

import numpy as np
from jmetal.algorithm.multiobjective.nsgaii import NSGAII
from jmetal.operator.crossover import SBXCrossover
from jmetal.operator.mutation import PolynomialMutation
from jmetal.problem.multiobjective.zdt import ZDT1
from jmetal.util.solution import get_non_dominated_solutions
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
JMETAL_CONFIG = CONFIG["jmetalpy"]
DIMENSION = int(INSTANCE["dimension"])
REFERENCE_POINT = [float(value) for value in INSTANCE["reference_point"]]


if BUDGET.get("type") != "evaluations":
    raise ValueError("This jMetalPy benchmark runner supports only evaluation budgets")

if len(SEEDS) < RUNS:
    raise ValueError("config.json must define at least one seed per run")


logging.getLogger("jmetal").setLevel(logging.CRITICAL)


def hypervolume_2d(front, reference_point):
    filtered = []
    for point in front:
        if len(point["objectives"]) != 2:
            continue
        f1, f2 = point["objectives"]
        if f1 <= reference_point[0] and f2 <= reference_point[1]:
            filtered.append((float(f1), float(f2)))

    filtered.sort(key=lambda item: item[0])

    total = 0.0
    previous_f2 = float(reference_point[1])
    for f1, f2 in filtered:
        if f2 < previous_f2:
            total += max(0.0, float(reference_point[0]) - f1) * (previous_f2 - f2)
            previous_f2 = f2
    return total


def run_benchmark(seed):
    random.seed(seed)
    np.random.seed(seed)

    problem = ZDT1(number_of_variables=DIMENSION)
    algorithm = NSGAII(
        problem=problem,
        population_size=int(JMETAL_CONFIG["population_size"]),
        offspring_population_size=int(JMETAL_CONFIG["offspring_population_size"]),
        mutation=PolynomialMutation(
            probability=float(JMETAL_CONFIG["mutation_probability"]),
            distribution_index=float(JMETAL_CONFIG["polynomial_distribution_index"]),
        ),
        crossover=SBXCrossover(
            probability=float(JMETAL_CONFIG["crossover_probability"]),
            distribution_index=float(JMETAL_CONFIG["sbx_distribution_index"]),
        ),
        termination_criterion=StoppingByEvaluations(int(BUDGET["value"])),
    )

    start_wall = time.perf_counter()
    algorithm.run()
    end_wall = time.perf_counter()

    population = algorithm.result()
    non_dominated = get_non_dominated_solutions(population)
    pareto_front = sorted(
        [
            {
                "variables": [float(value) for value in solution.variables],
                "objectives": [float(value) for value in solution.objectives],
            }
            for solution in non_dominated
        ],
        key=lambda point: (point["objectives"][0], point["objectives"][1]),
    )
    hypervolume = hypervolume_2d(pareto_front, REFERENCE_POINT)

    return {
        "benchmark_id": CONFIG["benchmark_id"],
        "library": "jmetalpy",
        "algorithm_family": CONFIG["algorithm_family"],
        "problem": INSTANCE["problem"],
        "instance_id": INSTANCE["instance_id"],
        "seed": seed,
        "budget_type": BUDGET["type"],
        "budget_value": int(BUDGET["value"]),
        "result_metric_name": "hypervolume",
        "final_fitness": hypervolume,
        "best_fitness": hypervolume,
        "best_solution": None,
        "pareto_front": pareto_front,
        "convergence_history": [[int(BUDGET["value"]), hypervolume]],
        "wall_time_ms": (end_wall - start_wall) * 1000.0,
        "cpu_time_ms": None,
        "evaluations": int(BUDGET["value"]),
        "status": "ok",
        "error": None,
    }


if __name__ == "__main__":
    results = [run_benchmark(SEEDS[index]) for index in range(RUNS)]
    print(json.dumps(results, indent=2))