import json
import os
import shutil
import shlex
import subprocess
import sys
from pathlib import Path


ROOT = Path(__file__).resolve().parents[2]
SHARED_DIR = ROOT / "shared"
INSTANCE_PATH = SHARED_DIR / "instance.json"
CONFIG_PATH = SHARED_DIR / "config.json"
CPP_SOURCE = ROOT / "runners" / "cpp" / "zdt1_pagmo_benchmark.cpp"
CACHE_DIR = ROOT / ".cache" / "pagmo_cpp"
BINARY_PATH = CACHE_DIR / "zdt1_pagmo_benchmark"


def load_json(path):
    with path.open("r", encoding="utf-8") as handle:
        return json.load(handle)


INSTANCE = load_json(INSTANCE_PATH)
CONFIG = load_json(CONFIG_PATH)
BUDGET = CONFIG["budget"]
RUNS = int(CONFIG["runs"])
SEEDS = list(CONFIG.get("seeds", []))
PAGMO_CONFIG = CONFIG["pagmo2_cpp"]
IN_CONTAINER_ENV = "ROMA_ZDT1_NSGA2_IN_CONTAINER"


def run_command(command, cwd=ROOT):
    return subprocess.run(
        command,
        cwd=cwd,
        check=False,
        capture_output=True,
        text=True,
    )


def skipped(reason, details=None):
    payload = {
        "library": "pagmo2_cpp",
        "status": "skipped",
        "reason": reason,
    }
    if details:
        payload["details"] = details
    print(json.dumps(payload, indent=2))
    raise SystemExit(0)


def local_pagmo_flags():
    if shutil.which("pkg-config") is None or shutil.which("g++") is None:
        return None
    completed = run_command(["pkg-config", "--cflags", "--libs", "pagmo"])
    if completed.returncode != 0:
        return None
    return shlex.split(completed.stdout.strip())


def compile_local_binary():
    CACHE_DIR.mkdir(parents=True, exist_ok=True)
    if BINARY_PATH.exists() and BINARY_PATH.stat().st_mtime >= CPP_SOURCE.stat().st_mtime:
        return

    if os.environ.get(IN_CONTAINER_ENV) != "1":
        flags = local_pagmo_flags()
        if flags is None:
            skipped("pagmo2 C++ is unavailable in the current environment")
    else:
        flags = ["-lpagmo"]

    completed = run_command(
        [
            "g++",
            "-O2",
            "-std=c++17",
            str(CPP_SOURCE),
            "-o",
            str(BINARY_PATH),
            *flags,
        ]
    )
    if completed.returncode != 0:
        sys.stderr.write(completed.stderr)
        raise SystemExit(completed.returncode)


def run_once(command):
    completed = run_command(command)
    if completed.returncode != 0:
        sys.stderr.write(completed.stderr)
        raise SystemExit(completed.returncode)
    return json.loads(completed.stdout)


def benchmark_args(seed):
    return [
        CONFIG["benchmark_id"],
        CONFIG["algorithm_family"],
        INSTANCE["problem"],
        INSTANCE["instance_id"],
        str(INSTANCE["dimension"]),
        BUDGET["type"],
        str(BUDGET["value"]),
        str(PAGMO_CONFIG["population_size"]),
        str(PAGMO_CONFIG["crossover_probability"]),
        str(PAGMO_CONFIG["mutation_probability"]),
        str(PAGMO_CONFIG["eta_c"]),
        str(PAGMO_CONFIG["eta_m"]),
        json.dumps(INSTANCE["reference_point"]),
        str(seed),
    ]


if __name__ == "__main__":
    if BUDGET.get("type") != "evaluations":
        raise ValueError("This pagmo2 C++ benchmark runner supports only evaluation budgets")

    if len(SEEDS) < RUNS:
        raise ValueError("config.json must define at least one seed per run")

    if shutil.which("g++") is None:
        skipped("g++ is unavailable in the current environment")

    compile_local_binary()
    results = [run_once([str(BINARY_PATH), *benchmark_args(SEEDS[index])]) for index in range(RUNS)]
    print(json.dumps(results, indent=2))