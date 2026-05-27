import csv
import json
import math
import os
import statistics
import subprocess


RESULTS_CSV_FIELDNAMES = [
    "benchmark_id",
    "executed_at_utc",
    "problem",
    "instance_id",
    "dimension",
    "objective_sense",
    "algorithm",
    "algorithm_family",
    "seed",
    "budget_type",
    "budget_value",
    "status",
    "final_fitness",
    "best_fitness",
    "wall_time_ms",
    "cpu_time_ms",
    "success",
    "evaluations",
    "best_solution",
    "convergence_history",
    "error",
    "execution_mode",
    "runner_command",
    "returncode",
]


def load_json(path):
    with path.open("r", encoding="utf-8") as handle:
        return json.load(handle)


def save_rows_csv(path, rows, fieldnames=None):
    effective_fieldnames = fieldnames or RESULTS_CSV_FIELDNAMES
    path.parent.mkdir(parents=True, exist_ok=True)
    with path.open("w", encoding="utf-8", newline="") as handle:
        writer = csv.DictWriter(handle, fieldnames=effective_fieldnames)
        writer.writeheader()
        for row in rows:
            writer.writerow(_normalize_csv_row(row, effective_fieldnames))


def read_rows_csv(path):
    with path.open("r", encoding="utf-8", newline="") as handle:
        reader = csv.DictReader(handle)
        return [_denormalize_csv_row(dict(row)) for row in reader]


def prepare_results_directory(path):
    path.mkdir(parents=True, exist_ok=True)
    for child in path.iterdir():
        if _is_writable_path(child):
            continue
        child.rename(_next_stale_path(child))


def run_command(command, cwd):
    completed = subprocess.run(
        command,
        cwd=cwd,
        check=False,
        capture_output=True,
        text=True,
    )
    return completed.returncode, completed.stdout, completed.stderr


def command_to_string(command):
    return " ".join(command)


def percentile(values, p):
    if not values:
        return None
    ordered = sorted(values)
    if len(ordered) == 1:
        return ordered[0]
    rank = (len(ordered) - 1) * p
    lower_idx = int(math.floor(rank))
    upper_idx = int(math.ceil(rank))
    if lower_idx == upper_idx:
        return ordered[lower_idx]
    lower = ordered[lower_idx]
    upper = ordered[upper_idx]
    return lower + (upper - lower) * (rank - lower_idx)


def summarize_results(results, objective_sense):
    ok_results = [result for result in results if result.get("status") == "ok"]
    error_results = [result for result in results if result.get("status") == "error"]
    skipped_results = [result for result in results if result.get("status") == "skipped"]
    fitness_values = [
        float(result.get("final_fitness", result.get("best_fitness")))
        for result in ok_results
        if result.get("final_fitness", result.get("best_fitness")) is not None
    ]
    wall_values = [float(result["wall_time_ms"]) for result in ok_results]
    cpu_values = [
        float(result["cpu_time_ms"])
        for result in ok_results
        if result.get("cpu_time_ms") is not None
    ]

    if not ok_results:
        return {
            "runs": len(results),
            "ok_runs": 0,
            "failed_runs": len(results),
            "error_runs": len(error_results),
            "skipped_runs": len(skipped_results),
            "best_fitness": None,
            "mean_fitness": None,
            "median_fitness": None,
            "worst_fitness": None,
            "stddev_fitness": None,
            "iqr_fitness": None,
            "mad_fitness": None,
            "p90_fitness": None,
            "mean_wall_time_ms": None,
            "median_wall_time_ms": None,
            "p90_wall_time_ms": None,
            "mean_cpu_time_ms": None,
            "median_cpu_time_ms": None,
        }

    if objective_sense == "max":
        best_fitness = max(fitness_values)
        worst_fitness = min(fitness_values)
    else:
        best_fitness = min(fitness_values)
        worst_fitness = max(fitness_values)

    return {
        "runs": len(results),
        "ok_runs": len(ok_results),
        "failed_runs": len(results) - len(ok_results),
        "error_runs": len(error_results),
        "skipped_runs": len(skipped_results),
        "best_fitness": best_fitness,
        "mean_fitness": statistics.fmean(fitness_values),
        "median_fitness": statistics.median(fitness_values),
        "worst_fitness": worst_fitness,
        "stddev_fitness": statistics.pstdev(fitness_values) if len(fitness_values) > 1 else 0.0,
        "iqr_fitness": _interquartile_range(fitness_values),
        "mad_fitness": _median_absolute_deviation(fitness_values),
        "p90_fitness": percentile(fitness_values, 0.90),
        "mean_wall_time_ms": statistics.fmean(wall_values),
        "median_wall_time_ms": statistics.median(wall_values),
        "p90_wall_time_ms": percentile(wall_values, 0.90),
        "mean_cpu_time_ms": statistics.fmean(cpu_values) if cpu_values else None,
        "median_cpu_time_ms": statistics.median(cpu_values) if cpu_values else None,
    }


def build_result_row(base_row, result, algorithm, runner_command, execution_mode, returncode=None):
    final_fitness = result.get("final_fitness", result.get("best_fitness"))
    status = result.get("status", "ok")
    success = result.get("success")
    if success is None:
        success = status == "ok"
    evaluations = result.get("evaluations")
    if evaluations is None and status == "ok" and result.get("budget_type", base_row.get("budget_type")) == "evaluations":
        evaluations = result.get("budget_value", base_row.get("budget_value"))

    return {
        **base_row,
        "algorithm": algorithm,
        "algorithm_family": result.get("algorithm_family"),
        "seed": result.get("seed"),
        "budget_type": result.get("budget_type", base_row.get("budget_type")),
        "budget_value": result.get("budget_value", base_row.get("budget_value")),
        "status": status,
        "final_fitness": final_fitness,
        "best_fitness": result.get("best_fitness", final_fitness),
        "wall_time_ms": result.get("wall_time_ms"),
        "cpu_time_ms": result.get("cpu_time_ms"),
        "success": success,
        "evaluations": evaluations,
        "best_solution": result.get("best_solution"),
        "convergence_history": result.get("convergence_history"),
        "error": result.get("error"),
        "execution_mode": execution_mode,
        "runner_command": runner_command,
        "returncode": returncode,
    }


def build_failed_rows(base_row, algorithm, seeds, runner_command, execution_mode, status, error, returncode=None):
    rows = []
    for seed in seeds:
        rows.append(
            {
                **base_row,
                "algorithm": algorithm,
                "algorithm_family": None,
                "seed": seed,
                "budget_type": base_row.get("budget_type"),
                "budget_value": base_row.get("budget_value"),
                "status": status,
                "final_fitness": None,
                "best_fitness": None,
                "wall_time_ms": None,
                "cpu_time_ms": None,
                "success": False,
                "evaluations": None,
                "best_solution": None,
                "convergence_history": None,
                "error": error,
                "execution_mode": execution_mode,
                "runner_command": runner_command,
                "returncode": returncode,
            }
        )
    return rows


def _normalize_csv_row(row, fieldnames):
    normalized = {}
    for fieldname in fieldnames:
        value = row.get(fieldname)
        if isinstance(value, (dict, list)):
            normalized[fieldname] = json.dumps(value, separators=(",", ":"))
        elif value is None:
            normalized[fieldname] = ""
        else:
            normalized[fieldname] = value
    return normalized


def _denormalize_csv_row(row):
    denormalized = {}
    for fieldname, value in row.items():
        if value == "":
            denormalized[fieldname] = None
            continue
        if fieldname in {"best_solution", "convergence_history"}:
            try:
                denormalized[fieldname] = json.loads(value)
                continue
            except json.JSONDecodeError:
                pass
        if fieldname == "success":
            lowered = value.lower()
            if lowered in {"true", "false"}:
                denormalized[fieldname] = lowered == "true"
                continue
        denormalized[fieldname] = value
    return denormalized


def _interquartile_range(values):
    if len(values) < 2:
        return 0.0
    return percentile(values, 0.75) - percentile(values, 0.25)


def _median_absolute_deviation(values):
    if not values:
        return None
    median_value = statistics.median(values)
    deviations = [abs(value - median_value) for value in values]
    return statistics.median(deviations)


def _is_writable_path(path):
    if path.is_dir():
        return os.access(path, os.W_OK | os.X_OK)
    return os.access(path, os.W_OK)


def _next_stale_path(path):
    suffix = 1
    while True:
        candidate = path.with_name(f"{path.name}.stale_root_{suffix}")
        if not candidate.exists():
            return candidate
        suffix += 1