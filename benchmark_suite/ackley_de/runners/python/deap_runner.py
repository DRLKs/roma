import copy
import json
import math
import random
import time
from pathlib import Path

from deap import base, creator


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


if not hasattr(creator, "AckleyFitnessMin"):
    creator.create("AckleyFitnessMin", base.Fitness, weights=(-1.0,))

if not hasattr(creator, "AckleyIndividual"):
    creator.create("AckleyIndividual", list, fitness=creator.AckleyFitnessMin)


def ackley_value(variables):
    dimension = float(len(variables))
    squared_mean = sum(value * value for value in variables) / dimension
    cosine_mean = sum(math.cos(2.0 * math.pi * value) for value in variables) / dimension
    return -20.0 * math.exp(-0.2 * math.sqrt(squared_mean)) - math.exp(cosine_mean) + 20.0 + math.e


def evaluate(individual):
    return (ackley_value(individual),)


def clamp(value):
    return min(UPPER_BOUND, max(LOWER_BOUND, float(value)))


def build_toolbox():
    toolbox = base.Toolbox()
    toolbox.register("value", random.uniform, LOWER_BOUND, UPPER_BOUND)
    toolbox.register("clone", copy.deepcopy)
    toolbox.register(
        "individual",
        lambda: creator.AckleyIndividual(toolbox.value() for _ in range(DIMENSION)),
    )
    toolbox.register("evaluate", evaluate)
    return toolbox


def initial_best(population):
    return min(population, key=lambda individual: float(individual.fitness.values[0]))


def run_benchmark(seed):
    random.seed(seed)
    toolbox = build_toolbox()
    population_size = int(DEAP_CONFIG["population_size"])
    budget_value = int(BUDGET["value"])
    crossover_rate = float(DEAP_CONFIG["crossover_rate"])
    differential_weight = float(DEAP_CONFIG["differential_weight"])

    if population_size < 4:
        raise ValueError("DEAP population_size must be at least 4")

    if budget_value < population_size:
        raise ValueError("Evaluation budget must be at least the DEAP population size")

    population = [toolbox.individual() for _ in range(population_size)]

    start_wall = time.perf_counter()
    start_cpu = time.process_time()

    for individual in population:
        individual.fitness.values = toolbox.evaluate(individual)
    evaluations = population_size
    best = toolbox.clone(initial_best(population))
    best.fitness.values = tuple(best.fitness.values)

    while evaluations < budget_value:
        next_population = []
        for target_index, target in enumerate(population):
            if evaluations >= budget_value:
                next_population.extend(toolbox.clone(individual) for individual in population[target_index:])
                break

            candidate_indices = [index for index in range(population_size) if index != target_index]
            donor_a_index, donor_b_index, donor_c_index = random.sample(candidate_indices, 3)
            donor_a = population[donor_a_index]
            donor_b = population[donor_b_index]
            donor_c = population[donor_c_index]

            trial = toolbox.clone(target)
            forced_index = random.randrange(DIMENSION)
            for variable_index in range(DIMENSION):
                if variable_index == forced_index or random.random() < crossover_rate:
                    mutant_value = donor_a[variable_index] + differential_weight * (
                        donor_b[variable_index] - donor_c[variable_index]
                    )
                    trial[variable_index] = clamp(mutant_value)

            trial.fitness.values = toolbox.evaluate(trial)
            evaluations += 1

            if float(trial.fitness.values[0]) <= float(target.fitness.values[0]):
                survivor = trial
            else:
                survivor = toolbox.clone(target)
                survivor.fitness.values = tuple(target.fitness.values)

            next_population.append(survivor)
            if float(survivor.fitness.values[0]) <= float(best.fitness.values[0]):
                best = toolbox.clone(survivor)
                best.fitness.values = tuple(survivor.fitness.values)

        population = next_population

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
        "best_solution": [float(value) for value in best],
        "wall_time_ms": (end_wall - start_wall) * 1000.0,
        "cpu_time_ms": (end_cpu - start_cpu) * 1000.0,
        "evaluations": evaluations,
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
                "evaluations": None,
                "status": "error",
                "error": str(exc),
            }
        results.append(result)

    print(json.dumps(results, indent=2))


if __name__ == "__main__":
    main()
