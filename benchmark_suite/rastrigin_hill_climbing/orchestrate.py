import json
import math
import os
import platform
import sys
import time
from datetime import datetime, timezone
from pathlib import Path

try:
    from bootstrap import configure_entrypoint
except ModuleNotFoundError:
    for candidate in [
        Path(__file__).resolve().parent.parent,
        Path.cwd().resolve(),
        Path.cwd().resolve() / "benchmark_suite",
    ]:
        if (candidate / "bootstrap.py").is_file():
            if str(candidate) not in sys.path:
                sys.path.insert(0, str(candidate))
            break
    else:
        raise RuntimeError("Could not locate benchmark_suite bootstrap module")
    from bootstrap import configure_entrypoint

ROOT, BENCHMARK_SUITE_ROOT, REPO_ROOT = configure_entrypoint(__file__)

from common import (
    build_failed_rows,
    build_result_row,
    command_to_string,
    load_json,
    prepare_results_directory,
    run_command,
    save_json,
    save_rows_csv,
    summarize_results,
)
from reporting import write_benchmark_reports

RUNNERS_DIR = ROOT / "runners"
RESULTS_DIR = ROOT / "results"
RESULTS_CSV_PATH = RESULTS_DIR / "runs.csv"
SUMMARY_PATH = RESULTS_DIR / "summary.json"
IN_CONTAINER_ENV = "ROMA_RASTRIGIN_IN_CONTAINER"
DOCKER_IMAGE = "roma-rastrigin-hill-climbing:latest"
CONTAINER_RESULTS_DIR = "/workspace/benchmark_suite/rastrigin_hill_climbing/results"
INSTANCE_PATH = ROOT / "shared" / "instance.json"
CONFIG_PATH = ROOT / "shared" / "config.json"


INSTANCE = load_json(INSTANCE_PATH)
CONFIG = load_json(CONFIG_PATH)


def build_runners():
    if os.environ.get(IN_CONTAINER_ENV) == "1":
        roma_command = ["/usr/local/bin/roma_rastrigin_hill_climbing_benchmark"]
        mealpy_python = "/opt/mealpy-venv/bin/python"
    else:
        roma_manifest = REPO_ROOT / "roma" / "Cargo.toml"
        roma_command = [
            "cargo",
            "run",
            "--quiet",
            "--example",
            "rastrigin_hill_climbing_benchmark",
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
    prepare_results_directory(RESULTS_DIR)

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
        "--user",
        f"{os.getuid()}:{os.getgid()}",
        "-e",
        "HOME=/tmp/roma-benchmark-home",
        "-e",
        "XDG_CACHE_HOME=/tmp/roma-benchmark-cache",
        "-v",
        f"{RESULTS_DIR}:{CONTAINER_RESULTS_DIR}",
        DOCKER_IMAGE,
    ]
    run_code, run_stdout, run_stderr = run_command(run_command_args, cwd=REPO_ROOT)
    if run_code != 0:
        sys.stderr.write(run_stderr or run_stdout)
        raise SystemExit(run_code)

    print(run_stdout, end="")


def rastrigin_value(variables):
    return 10.0 * len(variables) + sum(
        value * value - 10.0 * math.cos(2.0 * math.pi * value) for value in variables
    )


def validate_results(results):
    expected_problem = INSTANCE["problem"]
    expected_instance_id = INSTANCE["instance_id"]
    expected_budget_type = CONFIG["budget"]["type"]
    expected_budget_value = int(CONFIG["budget"]["value"])
    expected_dimension = int(INSTANCE["dimension"])
    lower_bound = float(INSTANCE["lower_bound"])
    upper_bound = float(INSTANCE["upper_bound"])

    max_fitness_abs_error = 0.0
    invalid_runs = 0
    out_of_bounds_runs = 0
    errors = []

    for index, result in enumerate(results):
        if result.get("status") != "ok":
            continue

        if result.get("problem") != expected_problem:
            invalid_runs += 1
            errors.append(f"run {index}: unexpected problem '{result.get('problem')}'")

        if result.get("instance_id") != expected_instance_id:
            invalid_runs += 1
            errors.append(
                f"run {index}: unexpected instance_id '{result.get('instance_id')}'"
            )

        if result.get("budget_type") != expected_budget_type:
            invalid_runs += 1
            errors.append(
                f"run {index}: unexpected budget_type '{result.get('budget_type')}'"
            )

        if int(result.get("budget_value", -1)) != expected_budget_value:
            invalid_runs += 1
            errors.append(
                f"run {index}: unexpected budget_value '{result.get('budget_value')}'"
            )

        solution = result.get("best_solution", [])
        if len(solution) != expected_dimension:
            invalid_runs += 1
            errors.append(
                f"run {index}: expected solution dimension {expected_dimension}, got {len(solution)}"
            )
            continue

        if any(value < lower_bound - 1e-9 or value > upper_bound + 1e-9 for value in solution):
            out_of_bounds_runs += 1
            invalid_runs += 1
            errors.append(f"run {index}: solution contains values outside declared bounds")

        recomputed = rastrigin_value(solution)
        fitness_abs_error = abs(float(result["best_fitness"]) - recomputed)
        max_fitness_abs_error = max(max_fitness_abs_error, fitness_abs_error)
        if fitness_abs_error > 1e-6:
            invalid_runs += 1
            errors.append(
                f"run {index}: reported fitness {result['best_fitness']} does not match recomputed value {recomputed}"
            )

    return {
        "valid": invalid_runs == 0,
        "invalid_runs": invalid_runs,
        "out_of_bounds_runs": out_of_bounds_runs,
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
    benchmark_base_row = {
        "benchmark_id": "rastrigin_hill_climbing",
        "executed_at_utc": timestamp,
        "problem": INSTANCE["problem"],
        "instance_id": INSTANCE["instance_id"],
        "dimension": INSTANCE["dimension"],
        "objective_sense": "min",
        "budget_type": CONFIG["budget"]["type"],
        "budget_value": CONFIG["budget"]["value"],
    }
    csv_rows = []
    summary = {
        "benchmark_id": "rastrigin_hill_climbing",
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
            "lower_bound": INSTANCE["lower_bound"],
            "upper_bound": INSTANCE["upper_bound"],
        },
        "budget": CONFIG["budget"],
        "runs": int(CONFIG["runs"]),
        "seeds": list(CONFIG.get("seeds", []))[: int(CONFIG["runs"])],
        "libraries": {},
    }

    for library, command in runners.items():
        runner_start_ts = time.perf_counter()
        returncode, stdout, stderr = run_command(command, cwd=ROOT)
        runner_elapsed_ms = (time.perf_counter() - runner_start_ts) * 1000.0
        mode = "container"
        runner_command = command_to_string(command)

        if returncode != 0:
            csv_rows.extend(
                build_failed_rows(
                    benchmark_base_row,
                    algorithm=library,
                    seeds=summary["seeds"],
                    runner_command=runner_command,
                    execution_mode=mode,
                    status="error",
                    error=stderr or stdout,
                    returncode=returncode,
                )
            )
            summary["libraries"][library] = {
                "status": "error",
                "execution_mode": mode,
                "runner_command": runner_command,
                "returncode": returncode,
                "runner_wall_time_ms": runner_elapsed_ms,
                "csv_output": str(RESULTS_CSV_PATH.relative_to(ROOT)),
            }
            continue

        results = json.loads(stdout)
        if isinstance(results, dict) and results.get("status") == "skipped":
            csv_rows.extend(
                build_failed_rows(
                    benchmark_base_row,
                    algorithm=library,
                    seeds=summary["seeds"],
                    runner_command=runner_command,
                    execution_mode=mode,
                    status="skipped",
                    error=results.get("reason"),
                )
            )
            summary["libraries"][library] = {
                "status": "skipped",
                "execution_mode": mode,
                "runner_command": runner_command,
                "runner_wall_time_ms": runner_elapsed_ms,
                "reason": results.get("reason"),
                "csv_output": str(RESULTS_CSV_PATH.relative_to(ROOT)),
            }
            continue

        csv_rows.extend(
            build_result_row(
                benchmark_base_row,
                result,
                algorithm=library,
                runner_command=runner_command,
                execution_mode=mode,
            )
            for result in results
        )
        validation = validate_results(results)
        aggregate = summarize_results(results, objective_sense="min")
        if validation["valid"] and aggregate["ok_runs"] > 0:
            summary["libraries"][library] = {
                "status": "ok",
                "execution_mode": mode,
                "runner_command": runner_command,
                "runner_wall_time_ms": runner_elapsed_ms,
                "completed_runs": len(results),
                "csv_output": str(RESULTS_CSV_PATH.relative_to(ROOT)),
                "aggregate": aggregate,
                "validation": validation,
            }
        else:
            summary["libraries"][library] = {
                "status": "error",
                "execution_mode": mode,
                "runner_command": runner_command,
                "runner_wall_time_ms": runner_elapsed_ms,
                "completed_runs": len(results),
                "csv_output": str(RESULTS_CSV_PATH.relative_to(ROOT)),
                "aggregate": aggregate,
                "validation": validation,
            }

    summary["total_wall_time_ms"] = (time.perf_counter() - start_ts) * 1000.0
    save_rows_csv(RESULTS_CSV_PATH, csv_rows)
    save_json(SUMMARY_PATH, summary)
    write_benchmark_reports(ROOT, summary)
    print(json.dumps(summary, indent=2))


if __name__ == "__main__":
    main()