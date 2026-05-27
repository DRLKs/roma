import json
import os
import platform
import sys
import time
from datetime import datetime, timezone
from pathlib import Path


def resolve_benchmark_suite_root(script_file):
    script_path = Path(script_file)
    candidate_roots = []

    executable_path = Path(sys.executable)
    try:
        executable_path = executable_path.resolve()
    except OSError:
        pass
    candidate_roots.extend([*executable_path.parents])

    shell_pwd = os.environ.get("PWD")
    if shell_pwd:
        shell_path = Path(shell_pwd)
        try:
            shell_path = shell_path.resolve()
        except OSError:
            pass
        candidate_roots.extend([shell_path, *shell_path.parents])

    cwd = Path.cwd().resolve()
    candidate_roots.extend([cwd, *cwd.parents])

    argv_path = Path(sys.argv[0])
    if not argv_path.is_absolute():
        argv_path = (cwd / argv_path).resolve()
    else:
        argv_path = argv_path.resolve()

    for parent in [argv_path.parent, *argv_path.parents]:
        candidate_roots.append(parent)

    for parent in [script_path.parent, *script_path.parents]:
        try:
            resolved_parent = parent.resolve()
        except OSError:
            resolved_parent = parent
        candidate_roots.append(resolved_parent)

    seen = set()
    for candidate in candidate_roots:
        if candidate in seen:
            continue
        seen.add(candidate)

        if (candidate / "common.py").is_file() and (candidate / "reporting.py").is_file():
            return candidate

        benchmark_suite_dir = candidate / "benchmark_suite"
        if (benchmark_suite_dir / "common.py").is_file() and (
            benchmark_suite_dir / "reporting.py"
        ).is_file():
            return benchmark_suite_dir

    home = Path.home()
    fallback_bases = [home]
    try:
        fallback_bases.extend(child for child in home.iterdir() if child.is_dir())
    except OSError:
        pass

    for base in fallback_bases:
        for candidate in (base, base / "benchmark_suite"):
            if (candidate / "common.py").is_file() and (
                candidate / "reporting.py"
            ).is_file():
                return candidate

    try:
        for common_path in home.rglob("benchmark_suite/common.py"):
            candidate = common_path.parent
            if (candidate / "reporting.py").is_file():
                return candidate
    except OSError:
        pass

    raise RuntimeError("Could not locate benchmark_suite root from current execution context")


BENCHMARK_SUITE_ROOT = resolve_benchmark_suite_root(__file__)
ROOT = BENCHMARK_SUITE_ROOT / Path(__file__).parent.name
REPO_ROOT = BENCHMARK_SUITE_ROOT.parent
if str(BENCHMARK_SUITE_ROOT) not in sys.path:
    sys.path.insert(0, str(BENCHMARK_SUITE_ROOT))

from common import (
    build_failed_rows,
    build_result_row,
    command_to_string,
    load_json,
    run_command,
    save_json,
    save_rows_csv,
    summarize_results,
)
from reporting import generate_suite_reports, write_benchmark_reports

RUNNERS_DIR = ROOT / "runners"
RESULTS_DIR = ROOT / "results"
RESULTS_CSV_PATH = RESULTS_DIR / "runs.csv"
SUMMARY_PATH = RESULTS_DIR / "summary.json"
IN_CONTAINER_ENV = "ROMA_KNAPSACK_SA_IN_CONTAINER"
DOCKER_IMAGE = "roma-knapsack-sa:latest"
CONTAINER_RESULTS_DIR = "/workspace/benchmark_suite/knapsack_sa/results"
INSTANCE_PATH = ROOT / "shared" / "instance.json"
CONFIG_PATH = ROOT / "shared" / "config.json"


INSTANCE = load_json(INSTANCE_PATH)
CONFIG = load_json(CONFIG_PATH)


def build_runners():
    if os.environ.get(IN_CONTAINER_ENV) == "1":
        roma_command = ["/usr/local/bin/roma_knapsack_sa_benchmark"]
    else:
        roma_manifest = REPO_ROOT / "roma" / "Cargo.toml"
        roma_command = [
            "cargo",
            "run",
            "--quiet",
            "--example",
            "knapsack_sa_benchmark",
            "--manifest-path",
            str(roma_manifest),
        ]

    return {
        "roma": roma_command,
        "deap": [sys.executable, str(RUNNERS_DIR / "python" / "deap_runner.py")],
        "greedy": [sys.executable, str(RUNNERS_DIR / "python" / "greedy_runner.py")],
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
    generate_suite_reports(ROOT.parent)


def knapsack_cost(solution):
    # follow Roma's knapsack evaluation: value if feasible, otherwise negative penalty
    weights = INSTANCE["weights"]
    values = INSTANCE["values"]
    capacity = float(INSTANCE["capacity"])
    penalty = 0.5
    total_w = 0.0
    total_v = 0.0
    for bit, w, v in zip(solution, weights, values):
        val = 1 if isinstance(bit, bool) and bit else int(bit)
        if val:
            total_w += float(w)
            total_v += float(v)

    if total_w > capacity:
        return -(total_w - capacity) * penalty
    return float(total_v)


def coerce_solution(raw_solution):
    if not isinstance(raw_solution, list):
        raise ValueError("best_solution must be a list")

    solution = []
    for value in raw_solution:
        if isinstance(value, bool):
            solution.append(value)
            continue
        if isinstance(value, int):
            if value not in (0, 1):
                raise ValueError(f"solution integer value {value!r} must be 0 or 1")
            solution.append(bool(value))
            continue
        if isinstance(value, float) and abs(value - round(value)) <= 1e-9:
            solution.append(bool(int(round(value))))
            continue
        raise ValueError(f"solution value {value!r} is not a boolean or 0/1 integer")
    return solution


def validate_results(results):
    expected_problem = INSTANCE["problem"]
    expected_instance_id = INSTANCE["instance_id"]
    expected_budget_type = CONFIG["budget"]["type"]
    expected_budget_value = int(CONFIG["budget"]["value"])
    expected_num_items = len(INSTANCE["weights"])

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
            solution = coerce_solution(result.get("best_solution", []))
        except ValueError as error:
            invalid_runs += 1
            invalid_assignments += 1
            errors.append(f"run {index}: {error}")
            continue

        if len(solution) != expected_num_items:
            invalid_runs += 1
            invalid_assignments += 1
            errors.append(
                f"run {index}: expected solution length {expected_num_items}, got {len(solution)}"
            )
            continue

        recomputed = knapsack_cost(solution)
        fitness_abs_error = abs(float(result["best_fitness"]) - recomputed)
        max_fitness_abs_error = max(max_fitness_abs_error, fitness_abs_error)
        if fitness_abs_error > 1e-6:
            invalid_runs += 1
            errors.append(
                f"run {index}: reported fitness {result['best_fitness']} does not match recomputed knapsack value {recomputed}"
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
    benchmark_base_row = {
        "benchmark_id": CONFIG["benchmark_id"],
        "executed_at_utc": timestamp,
        "problem": INSTANCE["problem"],
        "instance_id": INSTANCE["instance_id"],
        "dimension": len(INSTANCE["weights"]),
        "objective_sense": "max",
        "budget_type": CONFIG["budget"]["type"],
        "budget_value": CONFIG["budget"]["value"],
    }
    csv_rows = []
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
            "capacity": INSTANCE["capacity"],
            "num_items": len(INSTANCE["weights"]),
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
        aggregate = summarize_results(results, objective_sense="max")
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
