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


def build_toolbox():
    toolbox = base.Toolbox()
    toolbox.register("attr_bool", random.randint, 0, 1)
    toolbox.register("individual", tools.initRepeat, creator.KnapsackIndividual, toolbox.attr_bool, n=NUM_ITEMS)
    toolbox.register("clone", copy.deepcopy)
    toolbox.register("evaluate", evaluate)
    toolbox.register("mate", tools.cxTwoPoint)
    toolbox.register(
        "mutate",
        tools.mutFlipBit,
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
            # mutFlipBit returns a tuple (individual,)
            if random.random() < 1.0:
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
