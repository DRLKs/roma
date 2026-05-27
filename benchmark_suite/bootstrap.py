import sys
from pathlib import Path


def configure_entrypoint(script_file):
    script_path = Path(script_file).resolve()
    benchmark_root = script_path.parent
    candidate_suite_roots = [benchmark_root.parent]

    cwd = Path.cwd().resolve()
    candidate_suite_roots.extend([cwd, cwd / "benchmark_suite"])

    seen = set()
    for benchmark_suite_root in candidate_suite_roots:
        if benchmark_suite_root in seen:
            continue
        seen.add(benchmark_suite_root)

        if not _is_benchmark_suite_root(benchmark_suite_root):
            continue

        if str(benchmark_suite_root) not in sys.path:
            sys.path.insert(0, str(benchmark_suite_root))
        return benchmark_root, benchmark_suite_root, benchmark_suite_root.parent

    raise RuntimeError(
        f"Could not locate benchmark_suite root for entry point {script_path}"
    )


def _is_benchmark_suite_root(path):
    return (path / "common.py").is_file() and (path / "reporting.py").is_file()