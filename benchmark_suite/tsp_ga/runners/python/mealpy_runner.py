import json
import time
from pathlib import Path

from mealpy import GA, PermutationVar, Problem


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
MEALPY_CONFIG = CONFIG["mealpy"]
DISTANCE_MATRIX = INSTANCE["distance_matrix"]
DIMENSION = int(INSTANCE["dimension"])


if BUDGET.get("type") not in {"evaluations", "time"}:
    raise ValueError("This mealpy benchmark runner supports only evaluation or time budgets")

if len(SEEDS) < RUNS:
    raise ValueError("config.json must define at least one seed per run")


def route_distance(route):
    total = 0.0
    for index in range(len(route) - 1):
        total += DISTANCE_MATRIX[route[index]][route[index + 1]]
    total += DISTANCE_MATRIX[route[-1]][route[0]]
    return float(total)


class TspProblem(Problem):
    def __init__(self, bounds=None, minmax="min", distance_matrix=None, **kwargs):
        super().__init__(bounds=bounds, minmax=minmax, **kwargs)
        self.distance_matrix = distance_matrix
        self.evaluation_count = 0

    def obj_func(self, solution):
        self.evaluation_count += 1
        decoded = self.decode_solution(solution)
        route = [int(value) for value in decoded["per_var"]]
        return route_distance(route)


def run_benchmark(seed):
    budget_value = int(BUDGET["value"])
    pop_size = int(MEALPY_CONFIG["population_size"])
    budget_type = str(BUDGET["type"])
    if budget_type == "evaluations" and budget_value < pop_size:
        raise ValueError("Evaluation budget must be at least the mealpy population size")
    if budget_type == "time" and budget_value <= 0:
        raise ValueError("Time budget must be positive")

    epoch = max(1, (budget_value - pop_size) // pop_size) if budget_type == "evaluations" else 100000
    problem = TspProblem(
        bounds=PermutationVar(valid_set=list(range(DIMENSION)), name="per_var"),
        minmax="min",
        distance_matrix=DISTANCE_MATRIX,
        log_to=None,
        name=INSTANCE["instance_id"],
    )

    model = GA.OriginalGA(
        epoch=epoch,
        pop_size=pop_size,
        pc=float(MEALPY_CONFIG["crossover_probability"]),
        pm=float(MEALPY_CONFIG["mutation_probability"]),
        selection=str(MEALPY_CONFIG["selection"]),
        k_way=float(MEALPY_CONFIG["k_way"]),
        crossover=str(MEALPY_CONFIG["crossover"]),
        mutation=str(MEALPY_CONFIG["mutation"]),
        mutation_multipoints=bool(MEALPY_CONFIG["mutation_multipoints"]),
    )

    start_wall = time.perf_counter()
    if budget_type == "time":
        model.solve(problem, seed=seed, mode="single", termination={"max_time": float(budget_value)})
    else:
        model.solve(problem, seed=seed, mode="single")
    end_wall = time.perf_counter()

    best_route = [int(value) for value in problem.decode_solution(model.g_best.solution)["per_var"]]
    best_fitness = float(model.g_best.target.fitness)

    return {
        "benchmark_id": CONFIG["benchmark_id"],
        "library": "mealpy",
        "algorithm_family": CONFIG["algorithm_family"],
        "problem": INSTANCE["problem"],
        "instance_id": INSTANCE["instance_id"],
        "seed": seed,
        "budget_type": budget_type,
        "budget_value": budget_value,
        "best_fitness": best_fitness,
        "best_solution": best_route,
        "wall_time_ms": (end_wall - start_wall) * 1000.0,
        "cpu_time_ms": None,
        "evaluations": int(problem.evaluation_count),
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
                "library": "mealpy",
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
                "evaluations": None,
                "status": "error",
                "error": str(exc),
            }
        results.append(result)

    print(json.dumps(results, indent=2))


if __name__ == "__main__":
    main()