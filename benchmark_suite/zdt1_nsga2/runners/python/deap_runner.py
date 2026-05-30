import json
import random
import time
from pathlib import Path

from deap import base, benchmarks, creator, tools


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
REFERENCE_POINT = [float(value) for value in INSTANCE["reference_point"]]


if BUDGET.get("type") != "evaluations":
    raise ValueError("This DEAP benchmark runner supports only evaluation budgets")

if len(SEEDS) < RUNS:
    raise ValueError("config.json must define at least one seed per run")


if not hasattr(creator, "FitnessMinZdt1Nsga2"):
    creator.create("FitnessMinZdt1Nsga2", base.Fitness, weights=(-1.0, -1.0))
if not hasattr(creator, "IndividualZdt1Nsga2"):
    creator.create("IndividualZdt1Nsga2", list, fitness=creator.FitnessMinZdt1Nsga2)


def dominates(left, right):
    strictly_better = False
    for left_value, right_value in zip(left, right):
        if left_value > right_value:
            return False
        if left_value < right_value:
            strictly_better = True
    return strictly_better


def build_pareto_front(population):
    front = []
    for index, individual in enumerate(population):
        objectives = tuple(float(value) for value in individual.fitness.values)
        dominated = any(
            other_index != index
            and dominates(other.fitness.values, objectives)
            for other_index, other in enumerate(population)
        )
        if not dominated:
            front.append(
                {
                    "variables": [float(value) for value in individual],
                    "objectives": [float(value) for value in objectives],
                }
            )
    front.sort(key=lambda point: (point["objectives"][0], point["objectives"][1]))
    return front


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


def make_toolbox(seed):
    random.seed(seed)
    toolbox = base.Toolbox()
    toolbox.register("attr_float", random.random)
    toolbox.register(
        "individual",
        tools.initRepeat,
        creator.IndividualZdt1Nsga2,
        toolbox.attr_float,
        n=DIMENSION,
    )
    toolbox.register("population", tools.initRepeat, list, toolbox.individual)
    toolbox.register("evaluate", benchmarks.zdt1)
    toolbox.register(
        "mate",
        tools.cxSimulatedBinaryBounded,
        low=0.0,
        up=1.0,
        eta=float(DEAP_CONFIG["sbx_distribution_index"]),
    )
    toolbox.register(
        "mutate",
        tools.mutPolynomialBounded,
        low=0.0,
        up=1.0,
        eta=float(DEAP_CONFIG["polynomial_distribution_index"]),
        indpb=float(DEAP_CONFIG["mutation_probability"]),
    )
    toolbox.register("select", tools.selNSGA2)
    toolbox.register("select_mating", tools.selTournamentDCD)
    return toolbox


def evaluate_population(toolbox, individuals):
    invalid = [individual for individual in individuals if not individual.fitness.valid]
    fitness_values = list(map(toolbox.evaluate, invalid))
    for individual, fitness in zip(invalid, fitness_values):
        individual.fitness.values = fitness
    return len(invalid)


def run_benchmark(seed):
    toolbox = make_toolbox(seed)
    population_size = int(DEAP_CONFIG["population_size"])
    budget_value = int(BUDGET["value"])

    population = toolbox.population(n=population_size)

    start_wall = time.perf_counter()
    evaluations = evaluate_population(toolbox, population)
    population = toolbox.select(population, len(population))

    while evaluations < budget_value:
        offspring = toolbox.select_mating(population, len(population))
        offspring = [toolbox.clone(individual) for individual in offspring]

        for left, right in zip(offspring[::2], offspring[1::2]):
            if random.random() <= float(DEAP_CONFIG["crossover_probability"]):
                toolbox.mate(left, right)
            toolbox.mutate(left)
            toolbox.mutate(right)
            del left.fitness.values
            del right.fitness.values

        evaluations += evaluate_population(toolbox, offspring)
        population = toolbox.select(population + offspring, population_size)

    end_wall = time.perf_counter()

    pareto_front = build_pareto_front(population)
    hypervolume = hypervolume_2d(pareto_front, REFERENCE_POINT)

    return {
        "benchmark_id": CONFIG["benchmark_id"],
        "library": "deap",
        "algorithm_family": CONFIG["algorithm_family"],
        "problem": INSTANCE["problem"],
        "instance_id": INSTANCE["instance_id"],
        "seed": seed,
        "budget_type": BUDGET["type"],
        "budget_value": budget_value,
        "result_metric_name": "hypervolume",
        "final_fitness": hypervolume,
        "best_fitness": hypervolume,
        "best_solution": None,
        "pareto_front": pareto_front,
        "convergence_history": [[budget_value, hypervolume]],
        "wall_time_ms": (end_wall - start_wall) * 1000.0,
        "cpu_time_ms": None,
        "evaluations": budget_value,
        "status": "ok",
        "error": None,
    }


if __name__ == "__main__":
    results = [run_benchmark(SEEDS[index]) for index in range(RUNS)]
    print(json.dumps(results, indent=2))