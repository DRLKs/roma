import math
from itertools import combinations
from pathlib import Path

from common import load_json, read_rows_csv, summarize_results

scipy_stats = None
_SCIPY_IMPORT_ATTEMPTED = False


def write_benchmark_reports(benchmark_root, summary):
    results_dir = benchmark_root / "results"
    latex_dir = results_dir / "latex"
    latex_dir.mkdir(parents=True, exist_ok=True)

    fitness_rows = []
    timing_rows = []
    for algorithm, payload in sorted(summary.get("libraries", {}).items()):
        aggregate = payload.get("aggregate", {})
        fitness_rows.append(
            [
                algorithm,
                payload.get("status"),
                aggregate.get("ok_runs"),
                aggregate.get("best_fitness"),
                aggregate.get("mean_fitness"),
                aggregate.get("median_fitness"),
                aggregate.get("worst_fitness"),
                aggregate.get("stddev_fitness"),
                aggregate.get("iqr_fitness"),
                aggregate.get("mad_fitness"),
            ]
        )
        timing_rows.append(
            [
                algorithm,
                payload.get("status"),
                payload.get("runner_wall_time_ms"),
                aggregate.get("mean_wall_time_ms"),
                aggregate.get("median_wall_time_ms"),
                aggregate.get("p90_wall_time_ms"),
                aggregate.get("mean_cpu_time_ms"),
                aggregate.get("median_cpu_time_ms"),
            ]
        )

    benchmark_id = summary.get("benchmark_id", benchmark_root.name)
    write_latex_table(
        latex_dir / "fitness_summary.tex",
        column_spec="llrrrrrrrr",
        header=[
            "Algorithm",
            "Status",
            "OK runs",
            "Best",
            "Mean",
            "Median",
            "Worst",
            "Std. dev.",
            "IQR",
            "MAD",
        ],
        rows=fitness_rows,
        caption=f"Resumen descriptivo de fitness para {benchmark_id}.",
        label=f"tab:{benchmark_id}:fitness_summary",
    )
    write_latex_table(
        latex_dir / "timing_summary.tex",
        column_spec="llrrrrrr",
        header=[
            "Algorithm",
            "Status",
            "Runner ms",
            "Mean wall ms",
            "Median wall ms",
            "P90 wall ms",
            "Mean CPU ms",
            "Median CPU ms",
        ],
        rows=timing_rows,
        caption=f"Resumen temporal para {benchmark_id}.",
        label=f"tab:{benchmark_id}:timing_summary",
    )


def generate_suite_reports(benchmark_suite_root):
    datasets = _load_benchmark_datasets(benchmark_suite_root)
    reports_root = benchmark_suite_root / "reports"
    latex_dir = reports_root / "latex"
    latex_dir.mkdir(parents=True, exist_ok=True)

    if not datasets:
        _write_note(
            latex_dir / "suite_notes.tex",
            "No hay CSVs de benchmark disponibles todavía para construir el análisis transversal.",
        )
        return

    cohorts = _build_cohorts(datasets)
    if not cohorts:
        _write_note(
            latex_dir / "suite_notes.tex",
            "No hay suficientes benchmarks comparables para construir cohortes estadísticas.",
        )
        return

    overview_rows = []
    for cohort_index, cohort in enumerate(cohorts, start=1):
        cohort_slug = f"cohort_{cohort_index:02d}"
        overview_rows.append(
            [
                cohort_slug,
                len(cohort["benchmarks"]),
                len(cohort["algorithms"]),
                ", ".join(cohort["algorithms"]),
            ]
        )
        _write_cohort_reports(latex_dir, cohort_slug, cohort)

    write_latex_table(
        latex_dir / "suite_cohorts.tex",
        column_spec="lrrp{8cm}",
        header=["Cohort", "Benchmarks", "Algorithms", "Algorithms list"],
        rows=overview_rows,
        caption="Cohortes de benchmarks comparables detectadas automáticamente.",
        label="tab:suite:cohorts",
    )


def write_latex_table(path, column_spec, header, rows, caption, label):
    lines = [
        "\\begin{table}[htbp]",
        "\\centering",
        f"\\caption{{{_latex_escape(caption)}}}",
        f"\\label{{{label}}}",
        f"\\begin{{tabular}}{{{column_spec}}}",
        "\\hline",
        " {} \\\\".format(" & ".join(_latex_escape(cell) for cell in header)),
        "\\hline",
    ]
    for row in rows:
        lines.append(" {} \\\\".format(" & ".join(_format_cell(cell) for cell in row)))
    lines.extend([
        "\\hline",
        "\\end{tabular}",
        "\\end{table}",
        "",
    ])
    path.write_text("\n".join(lines), encoding="utf-8")


def _load_benchmark_datasets(benchmark_suite_root):
    datasets = []
    for child in sorted(benchmark_suite_root.iterdir()):
        if not child.is_dir():
            continue
        csv_path = child / "results" / "runs.csv"
        if not csv_path.exists():
            continue

        rows = read_rows_csv(csv_path)
        ok_rows = [row for row in rows if row.get("status") == "ok"]
        if not ok_rows:
            continue

        objective_sense = ok_rows[0].get("objective_sense") or "min"
        algorithms = sorted({row["algorithm"] for row in ok_rows})
        if len(algorithms) < 2:
            continue

        aggregates = {}
        comparison_scores = {}
        for algorithm in algorithms:
            algorithm_rows = [row for row in ok_rows if row.get("algorithm") == algorithm]
            aggregate = summarize_results(algorithm_rows, objective_sense=objective_sense)
            if aggregate.get("ok_runs", 0) == 0:
                continue
            aggregates[algorithm] = aggregate
            comparison_scores[algorithm] = _comparison_score(
                aggregate.get("mean_fitness"),
                objective_sense,
            )

        if len(aggregates) < 2:
            continue

        ranks = _rank_scores(comparison_scores)
        datasets.append(
            {
                "benchmark_id": ok_rows[0].get("benchmark_id") or child.name,
                "problem": ok_rows[0].get("problem") or child.name,
                "instance_id": ok_rows[0].get("instance_id") or child.name,
                "objective_sense": objective_sense,
                "algorithms": sorted(aggregates.keys()),
                "aggregates": aggregates,
                "comparison_scores": comparison_scores,
                "ranks": ranks,
            }
        )
    return datasets


def _build_cohorts(datasets):
    grouped = {}
    for dataset in datasets:
        key = tuple(dataset["algorithms"])
        grouped.setdefault(key, []).append(dataset)

    cohorts = []
    for algorithms, grouped_datasets in sorted(grouped.items(), key=lambda item: (-len(item[1]), -len(item[0]), item[0])):
        if len(grouped_datasets) < 2:
            continue
        cohorts.append(
            {
                "algorithms": list(algorithms),
                "benchmarks": grouped_datasets,
                "mean_ranks": _mean_ranks(grouped_datasets, algorithms),
                "friedman": _friedman_result(grouped_datasets, algorithms),
                "wilcoxon": _pairwise_wilcoxon(grouped_datasets, algorithms),
            }
        )
    return cohorts


def _mean_ranks(datasets, algorithms):
    return {
        algorithm: sum(dataset["ranks"][algorithm] for dataset in datasets) / len(datasets)
        for algorithm in algorithms
    }


def _friedman_result(datasets, algorithms):
    scipy_module = _get_scipy_stats()
    if len(algorithms) < 3:
        return {
            "status": "skipped",
            "reason": "Friedman requiere al menos 3 algoritmos en la cohorte.",
        }
    if len(datasets) < 2:
        return {
            "status": "skipped",
            "reason": "Friedman requiere al menos 2 benchmarks comparables.",
        }
    if scipy_module is None:
        return {
            "status": "skipped",
            "reason": "SciPy no está disponible en el entorno Python actual.",
        }

    samples = []
    for algorithm in algorithms:
        samples.append([dataset["comparison_scores"][algorithm] for dataset in datasets])
    statistic, p_value = scipy_module.friedmanchisquare(*samples)
    return {
        "status": "ok",
        "statistic": statistic,
        "p_value": p_value,
        "benchmarks": len(datasets),
        "algorithms": len(algorithms),
    }


def _pairwise_wilcoxon(datasets, algorithms):
    scipy_module = _get_scipy_stats()
    comparisons = []
    for left, right in combinations(algorithms, 2):
        left_ranks = [dataset["ranks"][left] for dataset in datasets]
        right_ranks = [dataset["ranks"][right] for dataset in datasets]
        result = {
            "algorithm_a": left,
            "algorithm_b": right,
            "mean_rank_a": sum(left_ranks) / len(left_ranks),
            "mean_rank_b": sum(right_ranks) / len(right_ranks),
            "effect_size": _rank_biserial_from_ranks(left_ranks, right_ranks),
        }
        if len(datasets) < 2:
            result.update({
                "status": "skipped",
                "reason": "Wilcoxon requiere al menos 2 benchmarks comparables.",
            })
        elif scipy_module is None:
            result.update({
                "status": "skipped",
                "reason": "SciPy no está disponible en el entorno Python actual.",
            })
        else:
            try:
                statistic, p_value = scipy_module.wilcoxon(left_ranks, right_ranks, zero_method="wilcox")
                result.update({
                    "status": "ok",
                    "statistic": statistic,
                    "p_value": p_value,
                })
            except ValueError as error:
                result.update({
                    "status": "skipped",
                    "reason": str(error),
                })
        comparisons.append(result)

    _apply_holm_correction(comparisons)
    return comparisons


def _apply_holm_correction(comparisons):
    valid = [comparison for comparison in comparisons if comparison.get("status") == "ok"]
    valid.sort(key=lambda comparison: comparison["p_value"])
    total = len(valid)
    running_max = 0.0
    for index, comparison in enumerate(valid):
        adjusted = (total - index) * comparison["p_value"]
        adjusted = min(1.0, max(running_max, adjusted))
        comparison["p_holm"] = adjusted
        running_max = adjusted
    for comparison in comparisons:
        if comparison.get("status") != "ok":
            comparison["p_holm"] = None


def _write_cohort_reports(latex_dir, cohort_slug, cohort):
    benchmark_rows = []
    for dataset in cohort["benchmarks"]:
        row = [dataset["benchmark_id"]]
        for algorithm in cohort["algorithms"]:
            row.append(dataset["aggregates"][algorithm]["mean_fitness"])
            row.append(dataset["ranks"][algorithm])
        benchmark_rows.append(row)

    benchmark_header = ["Benchmark"]
    benchmark_spec = "l"
    for algorithm in cohort["algorithms"]:
        benchmark_header.extend([f"{algorithm} mean", f"{algorithm} rank"])
        benchmark_spec += "rr"

    write_latex_table(
        latex_dir / f"{cohort_slug}_benchmarks.tex",
        column_spec=benchmark_spec,
        header=benchmark_header,
        rows=benchmark_rows,
        caption=f"Resultados agregados y ranks por benchmark para {cohort_slug}.",
        label=f"tab:{cohort_slug}:benchmarks",
    )

    mean_rank_rows = [
        [algorithm, cohort["mean_ranks"][algorithm]]
        for algorithm in sorted(cohort["algorithms"], key=lambda item: cohort["mean_ranks"][item])
    ]
    write_latex_table(
        latex_dir / f"{cohort_slug}_mean_ranks.tex",
        column_spec="lr",
        header=["Algorithm", "Mean rank"],
        rows=mean_rank_rows,
        caption=f"Ranks medios para {cohort_slug}.",
        label=f"tab:{cohort_slug}:mean_ranks",
    )

    friedman = cohort["friedman"]
    friedman_rows = [[
        friedman.get("status"),
        friedman.get("benchmarks"),
        friedman.get("algorithms"),
        friedman.get("statistic"),
        friedman.get("p_value"),
        friedman.get("reason"),
    ]]
    write_latex_table(
        latex_dir / f"{cohort_slug}_friedman.tex",
        column_spec="lrrrlp{5cm}",
        header=["Status", "Benchmarks", "Algorithms", "Statistic", "p-value", "Notes"],
        rows=friedman_rows,
        caption=f"Resultado del test de Friedman para {cohort_slug}.",
        label=f"tab:{cohort_slug}:friedman",
    )

    wilcoxon_rows = []
    for comparison in cohort["wilcoxon"]:
        wilcoxon_rows.append(
            [
                comparison["algorithm_a"],
                comparison["algorithm_b"],
                comparison.get("mean_rank_a"),
                comparison.get("mean_rank_b"),
                comparison.get("statistic"),
                comparison.get("p_value"),
                comparison.get("p_holm"),
                comparison.get("effect_size"),
                comparison.get("reason") if comparison.get("status") != "ok" else None,
            ]
        )
    write_latex_table(
        latex_dir / f"{cohort_slug}_wilcoxon.tex",
        column_spec="llrrrrrrp{4cm}",
        header=[
            "Alg. A",
            "Alg. B",
            "Mean rank A",
            "Mean rank B",
            "Statistic",
            "p-value",
            "p-Holm",
            "Effect size",
            "Notes",
        ],
        rows=wilcoxon_rows,
        caption=f"Comparaciones post-hoc de Wilcoxon para {cohort_slug}.",
        label=f"tab:{cohort_slug}:wilcoxon",
    )


def _comparison_score(mean_fitness, objective_sense):
    if mean_fitness is None:
        return None
    numeric_value = float(mean_fitness)
    if objective_sense == "max":
        return numeric_value
    return -numeric_value


def _rank_scores(scores_by_algorithm):
    ordered = sorted(scores_by_algorithm.items(), key=lambda item: item[1], reverse=True)
    ranks = {}
    index = 0
    while index < len(ordered):
        tie_end = index + 1
        while tie_end < len(ordered) and math.isclose(ordered[tie_end][1], ordered[index][1], rel_tol=1e-9, abs_tol=1e-9):
            tie_end += 1
        average_rank = (index + 1 + tie_end) / 2.0
        for tie_index in range(index, tie_end):
            ranks[ordered[tie_index][0]] = average_rank
        index = tie_end
    return ranks


def _rank_biserial_from_ranks(left_ranks, right_ranks):
    differences = [left - right for left, right in zip(left_ranks, right_ranks) if not math.isclose(left, right, abs_tol=1e-12)]
    if not differences:
        return 0.0

    absolute = [(abs(diff), diff) for diff in differences]
    absolute.sort(key=lambda item: item[0])
    signed_ranks = []
    index = 0
    while index < len(absolute):
        tie_end = index + 1
        while tie_end < len(absolute) and math.isclose(absolute[tie_end][0], absolute[index][0], rel_tol=1e-12, abs_tol=1e-12):
            tie_end += 1
        average_rank = (index + 1 + tie_end) / 2.0
        for tie_index in range(index, tie_end):
            signed_ranks.append((absolute[tie_index][1], average_rank))
        index = tie_end

    positive = sum(rank for diff, rank in signed_ranks if diff > 0)
    negative = sum(rank for diff, rank in signed_ranks if diff < 0)
    total = positive + negative
    if total == 0:
        return 0.0
    return (positive - negative) / total


def _format_cell(value):
    if value is None:
        return "--"
    if isinstance(value, (int, float)):
        return _format_number(value)
    return _latex_escape(value)


def _format_number(value):
    numeric_value = float(value)
    if math.isnan(numeric_value) or math.isinf(numeric_value):
        return "--"
    if math.isclose(numeric_value, round(numeric_value), abs_tol=1e-9):
        return str(int(round(numeric_value)))
    return f"{numeric_value:.4f}"


def _latex_escape(value):
    text = str(value)
    replacements = {
        "\\": "\\textbackslash{}",
        "&": "\\&",
        "%": "\\%",
        "$": "\\$",
        "#": "\\#",
        "_": "\\_",
        "{": "\\{",
        "}": "\\}",
        "~": "\\textasciitilde{}",
        "^": "\\textasciicircum{}",
    }
    for source, target in replacements.items():
        text = text.replace(source, target)
    return text


def _write_note(path, note):
    path.parent.mkdir(parents=True, exist_ok=True)
    path.write_text(note + "\n", encoding="utf-8")


def _get_scipy_stats():
    global scipy_stats, _SCIPY_IMPORT_ATTEMPTED
    if _SCIPY_IMPORT_ATTEMPTED:
        return scipy_stats

    _SCIPY_IMPORT_ATTEMPTED = True
    try:
        from scipy import stats as imported_scipy_stats
    except ImportError:
        scipy_stats = None
    else:
        scipy_stats = imported_scipy_stats
    return scipy_stats