import json
import logging
import random
import time
from pathlib import Path

import numpy as np
from jmetal.algorithm.singleobjective.genetic_algorithm import GeneticAlgorithm
from jmetal.core.problem import PermutationProblem
from jmetal.core.solution import PermutationSolution
from jmetal.operator.crossover import PMXCrossover
from jmetal.operator.mutation import PermutationSwapMutation
from jmetal.operator.selection import BinaryTournamentSelection
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
FLOW_MATRIX = INSTANCE["flow_matrix"]
DISTANCE_MATRIX = INSTANCE["distance_matrix"]


if BUDGET.get("type") != "evaluations":
    raise ValueError("This jMetalPy benchmark runner currently supports only evaluation budgets")

if len(SEEDS) < RUNS:
    raise ValueError("config.json must define at least one seed per run")


logging.getLogger("jmetal").setLevel(logging.CRITICAL)


class QAP(PermutationProblem):
    def __init__(self, flow_matrix, distance_matrix):
        super().__init__()
        self.flow_matrix = flow_matrix
        self.distance_matrix = distance_matrix
        self.dimension = len(flow_matrix)
        self.obj_directions = [self.MINIMIZE]
        self.obj_labels = ["cost"]

    def number_of_variables(self) -> int:
        return self.dimension

    def number_of_objectives(self) -> int:
        return 1

    def number_of_constraints(self) -> int:
        return 0

    def evaluate(self, solution: PermutationSolution) -> PermutationSolution:
        total = 0.0
        for facility_i, location_i in enumerate(solution.variables):
            for facility_j, location_j in enumerate(solution.variables):
                total += self.flow_matrix[facility_i][facility_j] * self.distance_matrix[location_i][location_j]
        solution.objectives[0] = float(total)
        return solution

    def create_solution(self) -> PermutationSolution:
        solution = PermutationSolution(
            number_of_variables=self.number_of_variables(),
            number_of_objectives=self.number_of_objectives(),
            number_of_constraints=self.number_of_constraints(),
        )
        solution.variables = random.sample(range(self.dimension), k=self.dimension)
        return solution

    def name(self):
        return "Single Objective QAP"


def run_benchmark(seed):
    random.seed(seed)
    np.random.seed(seed)

    problem = QAP(FLOW_MATRIX, DISTANCE_MATRIX)
    algorithm = GeneticAlgorithm(
        problem=problem,
        population_size=int(JMETAL_CONFIG["population_size"]),
        offspring_population_size=int(JMETAL_CONFIG["offspring_population_size"]),
        mutation=PermutationSwapMutation(probability=float(JMETAL_CONFIG["mutation_probability"])),
        crossover=PMXCrossover(probability=float(JMETAL_CONFIG["crossover_probability"])),
        selection=BinaryTournamentSelection(),
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
        "best_solution": [int(value) for value in result.variables],
        "wall_time_ms": (end_wall - start_wall) * 1000.0,
        "cpu_time_ms": None,
        "status": "ok",
        "error": None,
    }


if __name__ == "__main__":
    results = [run_benchmark(SEEDS[index]) for index in range(RUNS)]
    print(json.dumps(results, indent=2))