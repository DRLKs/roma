from pathlib import Path
import os
import sys


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


ROOT = resolve_benchmark_suite_root(__file__)
if str(ROOT) not in sys.path:
    sys.path.insert(0, str(ROOT))

from reporting import generate_suite_reports


def main():
    generate_suite_reports(ROOT)


if __name__ == "__main__":
    main()