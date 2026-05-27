from pathlib import Path
import sys

try:
    from bootstrap import configure_entrypoint
except ModuleNotFoundError:
    for candidate in [
        Path(__file__).resolve().parent,
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

_, ROOT, _ = configure_entrypoint(__file__)

from reporting import generate_suite_reports


def main():
    generate_suite_reports(ROOT)


if __name__ == "__main__":
    main()