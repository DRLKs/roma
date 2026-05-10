import json
import math
import os
import platform
import statistics
import subprocess
import sys
import time
from datetime import datetime, timezone
from pathlib import Path


ROOT = Path(__file__).resolve().parent
REPO_ROOT = ROOT.parent.parent
RUNNERS_DIR = ROOT / "runners"
RESULTS_DIR = ROOT / "results"
RAW_DIR = RESULTS_DIR / "raw"
SUMMARY_PATH = RESULTS_DIR / "summary.json"
IN_CONTAINER_ENV = "ROMA_QAP_GA_IN_CONTAINER"
DOCKER_IMAGE = "roma-qap-ga:latest"
CONTAINER_RESULTS_DIR = "/workspace/benchmark_suite/qap_ga/results"
INSTANCE_PATH = ROOT / "shared" / "instance.json"
CONFIG_PATH = ROOT / "shared" / "config.json"


def load_json(path):
    with path.open("r", encoding="utf-8") as handle:
        return json.load(handle)


INSTANCE = load_json(INSTANCE_PATH)
CONFIG = load_json(CONFIG_PATH)


def run_command(command, cwd=ROOT):
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


def build_runners():
    if os.environ.get(IN_CONTAINER_ENV) == "1":
        roma_command = ["/usr/local/bin/roma_qap_ga_benchmark"]
        mealpy_python = "/opt/mealpy-venv/bin/python"
    else:
        roma_manifest = REPO_ROOT / "roma" / "Cargo.toml"
        roma_command = [
            "cargo",
            "run",
            "--quiet",
            "--example",
            "qap_ga_benchmark",
            "--manifest-path",
            str(roma_manifest),
        ]
        mealpy_python = sys.executable

    return {
        "roma": roma_command,
        "deap": [sys.executable, str(RUNNERS_DIR / "python" / "deap_runner.py")],
        "jmetalpy": [sys.executable, str(RUNNERS_DIR / "python" / "jmetalpy_runner.py")],
        "mealpy": [mealpy_python, str(RUNNERS_DIR / "python" / "mealpy_runner.py")],
        "jmetal_java": [sys.executable, str(RUNNERS_DIR / "python" / "jmetal_java_runner.py")],
        "pagmo2_cpp": [sys.executable, str(RUNNERS_DIR / "python" / "pagmo_cpp_runner.py")],
    }


def run_containerized_orchestration():
    RESULTS_DIR.mkdir(parents=True, exist_ok=True)

    build_command = [
        "docker",
        "build",
        "-f",
        str(ROOT / "Dockerfile"),
        "-t",
        DOCKER_IMAGE,
        str(REPO_ROOT),
    ]
    build_code, build_stdout, build_stderr = run_command(build_command, cwd=REPO_ROOT)
    if build_code != 0:
        sys.stderr.write(build_stderr or build_stdout)
        raise SystemExit(build_code)

    run_command_args = [
        "docker",
        "run",
        "--rm",
        "-v",
        f"{RESULTS_DIR}:{CONTAINER_RESULTS_DIR}",
        DOCKER_IMAGE,
    ]
    run_code, run_stdout, run_stderr = run_command(run_command_args, cwd=REPO_ROOT)
    if run_code != 0:
        sys.stderr.write(run_stderr or run_stdout)
        raise SystemExit(run_code)

    print(run_stdout, end="")


def save_json(path, payload):
    path.parent.mkdir(parents=True, exist_ok=True)
    with path.open("w", encoding="utf-8") as handle:
        json.dump(payload, handle, indent=2)


def summarize_results(results):
    ok_results = [result for result in results if result.get("status") == "ok"]
    error_results = [result for result in results if result.get("status") == "error"]
    skipped_results = [result for result in results if result.get("status") == "skipped"]
    fitness_values = [result["best_fitness"] for result in ok_results]
    wall_values = [result["wall_time_ms"] for result in ok_results]
    cpu_values = [
        result["cpu_time_ms"]
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
            "worst_fitness": None,
            "stddev_fitness": None,
            "mean_wall_time_ms": None,
            "mean_cpu_time_ms": None,
        }

    return {
        "runs": len(results),
        "ok_runs": len(ok_results),
        "failed_runs": len(results) - len(ok_results),
        "error_runs": len(error_results),
        "skipped_runs": len(skipped_results),
        "best_fitness": min(fitness_values),
        "mean_fitness": statistics.fmean(fitness_values),
        "median_fitness": statistics.median(fitness_values),
        "worst_fitness": max(fitness_values),
        "stddev_fitness": statistics.pstdev(fitness_values) if len(fitness_values) > 1 else 0.0,
        "p90_fitness": percentile(fitness_values, 0.90),
        "mean_wall_time_ms": statistics.fmean(wall_values),
        "median_wall_time_ms": statistics.median(wall_values),
        "p90_wall_time_ms": percentile(wall_values, 0.90),
        "mean_cpu_time_ms": statistics.fmean(cpu_values) if cpu_values else None,
    }


def qap_cost(assignment):
    flow_matrix = INSTANCE["flow_matrix"]
    distance_matrix = INSTANCE["distance_matrix"]
    total = 0.0
    for facility_i, location_i in enumerate(assignment):
        for facility_j, location_j in enumerate(assignment):
            total += flow_matrix[facility_i][facility_j] * distance_matrix[location_i][location_j]
    return float(total)


def coerce_assignment(raw_solution):
    if not isinstance(raw_solution, list):
        raise ValueError("best_solution must be a list")

    assignment = []
    for value in raw_solution:
        if isinstance(value, bool):
            raise ValueError("assignment values must be integer location indexes")
        if isinstance(value, int):
            assignment.append(value)
            continue
        if isinstance(value, float) and abs(value - round(value)) <= 1e-9:
            assignment.append(int(round(value)))
            continue
        raise ValueError(f"assignment value {value!r} is not an integer location index")
    return assignment


def validate_results(results):
    expected_problem = INSTANCE["problem"]
    expected_instance_id = INSTANCE["instance_id"]
    expected_budget_type = CONFIG["budget"]["type"]
    expected_budget_value = int(CONFIG["budget"]["value"])
    expected_dimension = int(INSTANCE["dimension"])

    max_fitness_abs_error = 0.0
    invalid_runs = 0
    invalid_assignments = 0
    errors = []

    for index, result in enumerate(results):
        if result.get("status") != "ok":
            continue

        if result.get("problem") != expected_problem:
            invalid_runs += 1
            errors.append(f"run {index}: unexpected problem '{result.get('problem')}'")

        if result.get("instance_id") != expected_instance_id:
            invalid_runs += 1
            errors.append(f"run {index}: unexpected instance_id '{result.get('instance_id')}'")

        if result.get("budget_type") != expected_budget_type:
            invalid_runs += 1
            errors.append(f"run {index}: unexpected budget_type '{result.get('budget_type')}'")

        if int(result.get("budget_value", -1)) != expected_budget_value:
            invalid_runs += 1
            errors.append(f"run {index}: unexpected budget_value '{result.get('budget_value')}'")

        try:
            assignment = coerce_assignment(result.get("best_solution", []))
        except ValueError as error:
            invalid_runs += 1
            invalid_assignments += 1
            errors.append(f"run {index}: {error}")
            continue

        if len(assignment) != expected_dimension:
            invalid_runs += 1
            invalid_assignments += 1
            errors.append(
                f"run {index}: expected assignment dimension {expected_dimension}, got {len(assignment)}"
            )
            continue

        if sorted(assignment) != list(range(expected_dimension)):
            invalid_runs += 1
            invalid_assignments += 1
            errors.append(f"run {index}: assignment is not a permutation of all location indexes")
            continue

        recomputed = qap_cost(assignment)
        fitness_abs_error = abs(float(result["best_fitness"]) - recomputed)
        max_fitness_abs_error = max(max_fitness_abs_error, fitness_abs_error)
        if fitness_abs_error > 1e-6:
            invalid_runs += 1
            errors.append(
                f"run {index}: reported fitness {result['best_fitness']} does not match recomputed QAP cost {recomputed}"
            )

    return {
        "valid": invalid_runs == 0,
        "invalid_runs": invalid_runs,
        "invalid_assignments": invalid_assignments,
        "max_abs_fitness_error": max_fitness_abs_error,
        "errors": errors,
    }


def main():
    if os.environ.get(IN_CONTAINER_ENV) != "1":
        run_containerized_orchestration()
        return

    runners = build_runners()
    timestamp = datetime.now(timezone.utc).strftime("%Y%m%dT%H%M%SZ")
    start_ts = time.perf_counter()
    summary = {
        "benchmark_id": CONFIG["benchmark_id"],
        "executed_at_utc": timestamp,
        "environment": {
            "orchestrator_python": platform.python_version(),
            "platform": platform.platform(),
            "in_container": os.environ.get(IN_CONTAINER_ENV) == "1",
            "docker_image": DOCKER_IMAGE,
        },
        "instance": {
            "problem": INSTANCE["problem"],
            "instance_id": INSTANCE["instance_id"],
            "dimension": INSTANCE["dimension"],
            "qap_file": INSTANCE["qap_file"],
        },
        "budget": CONFIG["budget"],
        "runs": int(CONFIG["runs"]),
        "seeds": list(CONFIG.get("seeds", []))[: int(CONFIG["runs"])],
        "libraries": {},
    }

    for library, command in runners.items():
        runner_start_ts = time.perf_counter()
        returncode, stdout, stderr = run_command(command)
        runner_elapsed_ms = (time.perf_counter() - runner_start_ts) * 1000.0
        mode = "container"

        raw_path = RAW_DIR / f"{timestamp}_{library}.json"

        if returncode != 0:
            payload = {
                "library": library,
                "status": "error",
                "execution_mode": mode,
                "runner_command": command,
                "returncode": returncode,
                "runner_wall_time_ms": runner_elapsed_ms,
                "stdout": stdout,
                "stderr": stderr,
            }
            save_json(raw_path, payload)
            summary["libraries"][library] = {
                "status": "error",
                "execution_mode": mode,
                "runner_command": command_to_string(command),
                "returncode": returncode,
                "runner_wall_time_ms": runner_elapsed_ms,
                "raw_output": str(raw_path.relative_to(ROOT)),
            }
            continue

        results = json.loads(stdout)
        if isinstance(results, dict) and results.get("status") == "skipped":
            save_json(raw_path, results)
            summary["libraries"][library] = {
                "status": "skipped",
                "execution_mode": mode,
                "runner_command": command_to_string(command),
                "runner_wall_time_ms": runner_elapsed_ms,
                "reason": results.get("reason"),
                "raw_output": str(raw_path.relative_to(ROOT)),
            }
            continue

        validation = validate_results(results)
        aggregate = summarize_results(results)
        save_json(raw_path, results)
        if validation["valid"] and aggregate["ok_runs"] > 0:
            summary["libraries"][library] = {
                "status": "ok",
                "execution_mode": mode,
                "runner_command": command_to_string(command),
                "runner_wall_time_ms": runner_elapsed_ms,
                "completed_runs": len(results),
                "raw_output": str(raw_path.relative_to(ROOT)),
                "aggregate": aggregate,
                "validation": validation,
            }
        else:
            summary["libraries"][library] = {
                "status": "error",
                "execution_mode": mode,
                "runner_command": command_to_string(command),
                "runner_wall_time_ms": runner_elapsed_ms,
                "completed_runs": len(results),
                "raw_output": str(raw_path.relative_to(ROOT)),
                "aggregate": aggregate,
                "validation": validation,
            }

    summary["total_wall_time_ms"] = (time.perf_counter() - start_ts) * 1000.0
    save_json(SUMMARY_PATH, summary)
    print(json.dumps(summary, indent=2))


if __name__ == "__main__":
    main()