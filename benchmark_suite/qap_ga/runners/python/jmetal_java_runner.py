import json
import os
import shutil
import subprocess
import sys
import urllib.request
from pathlib import Path


ROOT = Path(__file__).resolve().parents[2]
SHARED_DIR = ROOT / "shared"
INSTANCE_PATH = SHARED_DIR / "instance.json"
CONFIG_PATH = SHARED_DIR / "config.json"
JAVA_SOURCE = ROOT / "runners" / "java" / "QapGeneticAlgorithmBenchmark.java"
CACHE_DIR = ROOT / ".cache" / "jmetal_java"
LIB_DIR = CACHE_DIR / "lib"
CLASS_DIR = CACHE_DIR / "classes"
MAIN_CLASS = "QapGeneticAlgorithmBenchmark"


def load_json(path):
    with path.open("r", encoding="utf-8") as handle:
        return json.load(handle)


INSTANCE = load_json(INSTANCE_PATH)
CONFIG = load_json(CONFIG_PATH)
BUDGET = CONFIG["budget"]
RUNS = int(CONFIG["runs"])
SEEDS = list(CONFIG.get("seeds", []))
JMETAL_CONFIG = CONFIG["jmetal_java"]
JMETAL_VERSION = str(JMETAL_CONFIG.get("version", "5.11"))
QAP_PATH = SHARED_DIR / INSTANCE["qap_file"]
JAR_URLS = {
    "jmetal-core": f"https://repo1.maven.org/maven2/org/uma/jmetal/jmetal-core/{JMETAL_VERSION}/jmetal-core-{JMETAL_VERSION}-jar-with-dependencies.jar",
    "jmetal-algorithm": f"https://repo1.maven.org/maven2/org/uma/jmetal/jmetal-algorithm/{JMETAL_VERSION}/jmetal-algorithm-{JMETAL_VERSION}-jar-with-dependencies.jar",
    "jmetal-problem": f"https://repo1.maven.org/maven2/org/uma/jmetal/jmetal-problem/{JMETAL_VERSION}/jmetal-problem-{JMETAL_VERSION}-jar-with-dependencies.jar",
}


def skipped(reason, details=None):
    payload = {
        "library": "jmetal_java",
        "status": "skipped",
        "reason": reason,
    }
    if details:
        payload["details"] = details
    print(json.dumps(payload, indent=2))
    raise SystemExit(0)


def run_command(command, cwd=ROOT):
    return subprocess.run(
        command,
        cwd=cwd,
        check=False,
        capture_output=True,
        text=True,
    )


def ensure_download(url, destination):
    destination.parent.mkdir(parents=True, exist_ok=True)
    if destination.exists():
        return
    urllib.request.urlretrieve(url, destination)


def ensure_jars():
    jars = []
    for artifact, url in JAR_URLS.items():
        destination = LIB_DIR / f"{artifact}-{JMETAL_VERSION}-jar-with-dependencies.jar"
        ensure_download(url, destination)
        jars.append(destination)
    return jars


def compile_runner(jars):
    CLASS_DIR.mkdir(parents=True, exist_ok=True)
    class_file = CLASS_DIR / f"{MAIN_CLASS}.class"
    source_mtime = JAVA_SOURCE.stat().st_mtime
    if class_file.exists() and class_file.stat().st_mtime >= source_mtime:
        return

    classpath = os.pathsep.join(str(path) for path in jars)
    completed = run_command(
        [
            "javac",
            "-cp",
            classpath,
            "-d",
            str(CLASS_DIR),
            str(JAVA_SOURCE),
        ]
    )
    if completed.returncode != 0:
        sys.stderr.write(completed.stderr)
        raise SystemExit(completed.returncode)


def run_once(seed, jars):
    classpath = os.pathsep.join([str(CLASS_DIR), *(str(path) for path in jars)])
    completed = run_command(
        [
            "java",
            "-cp",
            classpath,
            MAIN_CLASS,
            CONFIG["benchmark_id"],
            CONFIG["algorithm_family"],
            INSTANCE["problem"],
            INSTANCE["instance_id"],
            str(QAP_PATH),
            BUDGET["type"],
            str(BUDGET["value"]),
            str(JMETAL_CONFIG["population_size"]),
            str(JMETAL_CONFIG["crossover_probability"]),
            str(JMETAL_CONFIG["mutation_probability"]),
            str(seed),
        ]
    )
    if completed.returncode != 0:
        sys.stderr.write(completed.stderr)
        raise SystemExit(completed.returncode)
    return json.loads(completed.stdout)


if __name__ == "__main__":
    if BUDGET.get("type") != "evaluations":
        raise ValueError("This jMetal Java benchmark runner currently supports only evaluation budgets")

    if len(SEEDS) < RUNS:
        raise ValueError("config.json must define at least one seed per run")

    if shutil.which("java") is None or shutil.which("javac") is None:
        skipped("java or javac is unavailable in the local environment")

    try:
        jar_paths = ensure_jars()
    except Exception as error:  # noqa: BLE001
        skipped("failed to download jMetal Java artifacts", str(error))

    compile_runner(jar_paths)
    results = [run_once(SEEDS[index], jar_paths) for index in range(RUNS)]
    print(json.dumps(results, indent=2))