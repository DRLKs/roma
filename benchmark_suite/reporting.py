from benchmark_analysis import generate_benchmark_analysis


def write_benchmark_reports(benchmark_root, summary):
    generate_benchmark_analysis(benchmark_root, summary)


def generate_suite_reports(benchmark_suite_root):
    for child in sorted(benchmark_suite_root.iterdir()):
        if not child.is_dir():
            continue
        if (child / "results" / "runs.csv").exists():
            generate_benchmark_analysis(child)