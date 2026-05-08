import copy
import json
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
DISTANCE_MATRIX = INSTANCE["distance_matrix"]
DIMENSION = int(INSTANCE["dimension"])


if BUDGET.get("type") != "evaluations":
    raise ValueError("This DEAP benchmark runner currently supports only evaluation budgets")

if len(SEEDS) < RUNS:
    raise ValueError("config.json must define at least one seed per run")


if not hasattr(creator, "TspFitnessMin"):
    creator.create("TspFitnessMin", base.Fitness, weights=(-1.0,))

if not hasattr(creator, "TspIndividual"):
    creator.create("TspIndividual", list, fitness=creator.TspFitnessMin)


def route_distance(route):
    total = 0.0
    for index in range(len(route) - 1):
        total += DISTANCE_MATRIX[route[index]][route[index + 1]]
    total += DISTANCE_MATRIX[route[-1]][route[0]]
    return float(total)


def evaluate(individual):
    return (route_distance(individual),)


def build_toolbox():
    toolbox = base.Toolbox()
    toolbox.register("permutation", random.sample, range(DIMENSION), DIMENSION)
    toolbox.register("individual", tools.initIterate, creator.TspIndividual, toolbox.permutation)
    toolbox.register("clone", copy.deepcopy)
    toolbox.register("evaluate", evaluate)
    toolbox.register("mate", tools.cxOrdered)
    toolbox.register(
        "mutate",
        tools.mutShuffleIndexes,
        indpb=float(DEAP_CONFIG["mutation_probability"]),
    )
    toolbox.register(
        "select",
        tools.selTournament,
        tournsize=int(DEAP_CONFIG["tournament_size"]),
    )
    return toolbox


def run_benchmark(seed):
    random.seed(seed)
    toolbox = build_toolbox()
    population_size = int(DEAP_CONFIG["population_size"])
    budget_value = int(BUDGET["value"])
    crossover_probability = float(DEAP_CONFIG["crossover_probability"])
    mutation_probability = float(DEAP_CONFIG["mutation_probability"])

    if budget_value < population_size:
        raise ValueError("Evaluation budget must be at least the DEAP population size")

    population = [toolbox.individual() for _ in range(population_size)]
    start_wall = time.perf_counter()
    start_cpu = time.process_time()

    for individual in population:
        individual.fitness.values = toolbox.evaluate(individual)
    evaluations = population_size

    hall_of_fame = tools.HallOfFame(1)
    hall_of_fame.update(population)
    population = tools.selBest(population, population_size)

    while evaluations < budget_value:
        offspring = list(map(toolbox.clone, toolbox.select(population, population_size)))

        for child1, child2 in zip(offspring[::2], offspring[1::2]):
            if random.random() < crossover_probability:
                toolbox.mate(child1, child2)
                if hasattr(child1.fitness, "values"):
                    del child1.fitness.values
                if hasattr(child2.fitness, "values"):
                    del child2.fitness.values

        for mutant in offspring:
            if random.random() < mutation_probability:
                toolbox.mutate(mutant)
                if hasattr(mutant.fitness, "values"):
                    del mutant.fitness.values

        invalid = [individual for individual in offspring if not individual.fitness.valid]
        if not invalid:
            break

        remaining_evaluations = budget_value - evaluations
        invalid = invalid[:remaining_evaluations]

        for individual in invalid:
            individual.fitness.values = toolbox.evaluate(individual)
        evaluations += len(invalid)

        offspring = [individual for individual in offspring if individual.fitness.valid]
        if not offspring:
            break

        population = tools.selBest(population + offspring, population_size)
        hall_of_fame.update(population)

    end_cpu = time.process_time()
    end_wall = time.perf_counter()
    best = hall_of_fame[0] if len(hall_of_fame) > 0 else tools.selBest(population, 1)[0]

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