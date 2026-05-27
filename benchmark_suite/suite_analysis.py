from __future__ import annotations

import json
import math
from itertools import combinations
from pathlib import Path

from common import read_rows_csv

try:
    import matplotlib.pyplot as plt
except ImportError:
    plt = None

try:
    import numpy as np
except ImportError:
    np = None

try:
    import pandas as pd
except ImportError:
    pd = None

try:
    import scikit_posthocs as sp
except ImportError:
    sp = None

try:
    from scipy import stats as scipy_stats
except ImportError:
    scipy_stats = None


PIPELINE_DIRNAME = "benchmark"
PIPELINE_SUBDIRS = ["raw", "aggregated", "analysis", "plots", "reports", "reports/latex"]
CD_Q_ALPHA_005 = {
    2: 1.960,
    3: 2.344,
    4: 2.569,
    5: 2.728,
    6: 2.850,
    7: 2.949,
    8: 3.031,
    9: 3.102,
    10: 3.164,
}


def generate_suite_analysis(benchmark_suite_root: Path):
    pipeline_root = benchmark_suite_root / PIPELINE_DIRNAME
    _ensure_dirs(pipeline_root)
    legacy_reports_root = benchmark_suite_root / "reports" / "latex"
    legacy_reports_root.mkdir(parents=True, exist_ok=True)

    if pd is None or np is None:
        note = (
            "El pipeline estadistico requiere pandas y numpy. "
            "Instala las dependencias de benchmark_suite/requirements-analysis.txt para regenerarlo."
        )
        _write_note(pipeline_root / "reports" / "report.md", note)
        _write_note(pipeline_root / "reports" / "latex" / "suite_notes.tex", note)
        _write_note(legacy_reports_root / "suite_notes.tex", note)
        return

    raw_df = _load_raw_runs_dataframe(benchmark_suite_root)
    if raw_df.empty:
        note = "No hay CSVs de benchmark disponibles todavia para construir el analisis estadistico."
        _write_note(pipeline_root / "reports" / "report.md", note)
        _write_note(pipeline_root / "reports" / "latex" / "suite_notes.tex", note)
        _write_note(legacy_reports_root / "suite_notes.tex", note)
        return

    raw_dir = pipeline_root / "raw"
    aggregated_dir = pipeline_root / "aggregated"
    analysis_dir = pipeline_root / "analysis"
    plots_dir = pipeline_root / "plots"
    reports_dir = pipeline_root / "reports"
    latex_dir = reports_dir / "latex"

    raw_df.to_csv(raw_dir / "runs.csv", index=False)
    convergence_df = _extract_convergence_dataframe(raw_df)
    convergence_df.to_csv(raw_dir / "convergence.csv", index=False)

    raw_manifest = {
        "benchmark_count": int(raw_df["benchmark_id"].nunique()),
        "instance_count": int(raw_df["instance_key"].nunique()),
        "algorithm_count": int(raw_df["algorithm"].nunique()),
        "run_count": int(len(raw_df)),
        "ok_run_count": int((raw_df["status"] == "ok").sum()),
        "convergence_rows": int(len(convergence_df)),
        "benchmarks": sorted(raw_df["benchmark_id"].dropna().unique().tolist()),
    }
    _save_json(raw_dir / "manifest.json", raw_manifest)

    instance_summary_df = _aggregate_instance_algorithm(raw_df)
    benchmark_summary_df = _aggregate_benchmark_algorithm(raw_df)
    descriptive_df = benchmark_summary_df[
        [
            "benchmark_id",
            "instance_id",
            "algorithm",
            "objective_sense",
            "mean_final_fitness",
            "median_final_fitness",
            "std_final_fitness",
            "best_final_fitness",
        ]
    ].rename(
        columns={
            "mean_final_fitness": "mean",
            "median_final_fitness": "median",
            "std_final_fitness": "std",
            "best_final_fitness": "best",
        }
    )

    instance_summary_df.to_csv(aggregated_dir / "instance_algorithm_summary.csv", index=False)
    benchmark_summary_df.to_csv(aggregated_dir / "benchmark_algorithm_summary.csv", index=False)
    descriptive_df.to_csv(analysis_dir / "descriptive_summary.csv", index=False)

    cohorts = _build_cohorts(instance_summary_df)
    friedman_rows, avg_rank_rows, holm_rows, nemenyi_rows, effect_rows = _analyze_cohorts(cohorts)
    instance_wilcoxon_df = _pairwise_instance_wilcoxon(raw_df)

    _write_analysis_tables(
        analysis_dir,
        descriptive_df,
        friedman_rows,
        avg_rank_rows,
        holm_rows,
        nemenyi_rows,
        effect_rows,
        instance_wilcoxon_df,
    )
    _write_report_artifacts(
        reports_dir,
        latex_dir,
        raw_manifest,
        descriptive_df,
        friedman_rows,
        avg_rank_rows,
        holm_rows,
        nemenyi_rows,
        effect_rows,
        instance_wilcoxon_df,
    )
    _generate_plots(plots_dir, raw_df, instance_summary_df, convergence_df, cohorts)

    pipeline_summary = {
        "raw": raw_manifest,
        "aggregated": {
            "instance_algorithm_rows": int(len(instance_summary_df)),
            "benchmark_algorithm_rows": int(len(benchmark_summary_df)),
        },
        "analysis": {
            "cohort_count": len(cohorts),
            "friedman_rows": len(friedman_rows),
            "average_rank_rows": len(avg_rank_rows),
            "pairwise_holm_rows": len(holm_rows),
            "posthoc_rows": len(nemenyi_rows),
            "effect_size_rows": len(effect_rows),
            "instance_wilcoxon_rows": int(len(instance_wilcoxon_df)),
        },
    }
    _save_json(analysis_dir / "summary.json", pipeline_summary)

    legacy_note = (
        "El analisis suite-wide ahora se genera en benchmark_suite/benchmark/. "
        "Consulta benchmark/reports/report.md y benchmark/reports/latex/*.tex."
    )
    _write_note(legacy_reports_root / "suite_notes.tex", legacy_note)


def _ensure_dirs(pipeline_root: Path):
    for subdir in PIPELINE_SUBDIRS:
        (pipeline_root / subdir).mkdir(parents=True, exist_ok=True)


def _load_raw_runs_dataframe(benchmark_suite_root: Path):
    rows = []
    for child in sorted(benchmark_suite_root.iterdir()):
        if not child.is_dir() or child.name in {PIPELINE_DIRNAME, "reports", "__pycache__"}:
            continue
        csv_path = child / "results" / "runs.csv"
        if not csv_path.exists():
            continue
        for row in read_rows_csv(csv_path):
            row["source_benchmark_dir"] = child.name
            row["source_csv"] = str(csv_path.relative_to(benchmark_suite_root))
            rows.append(row)

    if not rows:
        return pd.DataFrame()

    df = pd.DataFrame(rows)
    return _normalize_raw_dataframe(df)


def _normalize_raw_dataframe(df):
    normalized = df.copy()
    for column in ["final_fitness", "best_fitness", "success", "evaluations", "convergence_history"]:
        if column not in normalized.columns:
            normalized[column] = None
    if "final_fitness" not in normalized.columns:
        normalized["final_fitness"] = normalized.get("best_fitness")
    normalized["final_fitness"] = normalized["final_fitness"].fillna(normalized.get("best_fitness"))
    normalized["success"] = normalized.get("success")
    normalized["success"] = normalized["success"].where(normalized["success"].notna(), normalized["status"] == "ok")
    normalized["success"] = normalized["success"].astype(bool)

    numeric_columns = [
        "dimension",
        "seed",
        "budget_value",
        "final_fitness",
        "best_fitness",
        "wall_time_ms",
        "cpu_time_ms",
        "evaluations",
        "returncode",
    ]
    for column in numeric_columns:
        if column in normalized.columns:
            normalized[column] = pd.to_numeric(normalized[column], errors="coerce")

    normalized["objective_sense"] = normalized["objective_sense"].fillna("min")
    normalized["instance_key"] = normalized.apply(
        lambda row: f"{row['benchmark_id']}::{row['instance_id']}",
        axis=1,
    )
    normalized["evaluations"] = normalized["evaluations"].where(
        normalized["evaluations"].notna(),
        np.where(
            (normalized["budget_type"] == "evaluations") & (normalized["status"] == "ok"),
            normalized["budget_value"],
            np.nan,
        ),
    )
    normalized["ranking_value"] = normalized.apply(_ranking_value_for_row, axis=1)
    return normalized


def _ranking_value_for_row(row):
    value = row.get("final_fitness")
    if value is None or (isinstance(value, float) and math.isnan(value)):
        return math.nan
    if row.get("objective_sense") == "max":
        return -float(value)
    return float(value)


def _extract_convergence_dataframe(raw_df):
    rows = []
    for _, row in raw_df.iterrows():
        history = row.get("convergence_history")
        if not history:
            continue
        for point in history:
            parsed = _parse_history_point(point)
            if parsed is None:
                continue
            evaluation, best_fitness_so_far = parsed
            rows.append(
                {
                    "benchmark_id": row["benchmark_id"],
                    "instance_id": row["instance_id"],
                    "instance_key": row["instance_key"],
                    "algorithm": row["algorithm"],
                    "seed": row["seed"],
                    "evaluation": evaluation,
                    "best_fitness_so_far": best_fitness_so_far,
                    "objective_sense": row["objective_sense"],
                }
            )
    return pd.DataFrame(rows)


def _parse_history_point(point):
    if isinstance(point, dict):
        evaluation = point.get("evaluation")
        fitness = point.get("best_fitness_so_far", point.get("fitness"))
    elif isinstance(point, (list, tuple)) and len(point) >= 2:
        evaluation, fitness = point[0], point[1]
    else:
        return None

    try:
        return float(evaluation), float(fitness)
    except (TypeError, ValueError):
        return None


def _aggregate_instance_algorithm(raw_df):
    ok_df = raw_df[raw_df["status"] == "ok"].copy()
    if ok_df.empty:
        return pd.DataFrame()

    group_columns = [
        "benchmark_id",
        "problem",
        "dimension",
        "instance_id",
        "instance_key",
        "objective_sense",
        "algorithm",
        "algorithm_family",
        "budget_type",
        "budget_value",
    ]
    summary_rows = []
    for group_key, group_df in ok_df.groupby(group_columns, dropna=False):
        payload = dict(zip(group_columns, group_key))
        fitness_values = group_df["final_fitness"].dropna().to_numpy(dtype=float)
        wall_values = group_df["wall_time_ms"].dropna().to_numpy(dtype=float)
        cpu_values = group_df["cpu_time_ms"].dropna().to_numpy(dtype=float)
        eval_values = group_df["evaluations"].dropna().to_numpy(dtype=float)

        payload.update(
            {
                "run_count": int(len(group_df)),
                "seed_count": int(group_df["seed"].dropna().nunique()),
                "success_rate": float(group_df["success"].astype(float).mean()) if len(group_df) else math.nan,
                "mean_final_fitness": _safe_mean(fitness_values),
                "median_final_fitness": _safe_median(fitness_values),
                "std_final_fitness": _safe_std(fitness_values),
                "min_final_fitness": _safe_min(fitness_values),
                "max_final_fitness": _safe_max(fitness_values),
                "best_final_fitness": _best_value(fitness_values, payload["objective_sense"]),
                "p90_final_fitness": _safe_percentile(fitness_values, 90),
                "iqr_final_fitness": _safe_iqr(fitness_values),
                "mean_wall_time_ms": _safe_mean(wall_values),
                "median_wall_time_ms": _safe_median(wall_values),
                "std_wall_time_ms": _safe_std(wall_values),
                "p90_wall_time_ms": _safe_percentile(wall_values, 90),
                "mean_cpu_time_ms": _safe_mean(cpu_values),
                "median_cpu_time_ms": _safe_median(cpu_values),
                "p90_cpu_time_ms": _safe_percentile(cpu_values, 90),
                "mean_evaluations": _safe_mean(eval_values),
                "median_evaluations": _safe_median(eval_values),
            }
        )
        payload["ranking_value"] = _ranking_value(payload["median_final_fitness"], payload["objective_sense"])
        summary_rows.append(payload)

    return pd.DataFrame(summary_rows)


def _aggregate_benchmark_algorithm(raw_df):
    if raw_df.empty:
        return pd.DataFrame()
    return _aggregate_instance_algorithm(raw_df).drop(columns=["instance_key"])


def _build_cohorts(instance_summary_df):
    if instance_summary_df.empty:
        return []

    working = instance_summary_df.copy()
    working["algorithm_set"] = working.groupby("instance_key")["algorithm"].transform(
        lambda series: "|".join(sorted(series.tolist()))
    )

    cohorts = []
    for cohort_index, (algorithm_set, cohort_df) in enumerate(
        sorted(working.groupby("algorithm_set"), key=lambda item: (-item[1]["instance_key"].nunique(), item[0])),
        start=1,
    ):
        algorithms = algorithm_set.split("|") if algorithm_set else []
        if len(algorithms) < 2:
            continue
        wide = (
            cohort_df.pivot(index="instance_key", columns="algorithm", values="ranking_value")
            .reindex(columns=algorithms)
            .dropna(axis=0, how="any")
        )
        if wide.empty:
            continue
        raw_wide = (
            cohort_df.pivot(index="instance_key", columns="algorithm", values="median_final_fitness")
            .reindex(columns=algorithms)
            .loc[wide.index]
        )
        rank_wide = wide.rank(axis=1, method="average", ascending=True)
        metadata = (
            cohort_df.drop_duplicates("instance_key")
            .set_index("instance_key")
            .loc[wide.index, ["benchmark_id", "instance_id", "problem", "dimension", "objective_sense"]]
            .reset_index()
        )
        cohorts.append(
            {
                "cohort_id": f"cohort_{cohort_index:02d}",
                "algorithms": algorithms,
                "wide": wide,
                "raw_wide": raw_wide,
                "rank_wide": rank_wide,
                "metadata": metadata,
            }
        )
    return cohorts


def _analyze_cohorts(cohorts):
    friedman_rows = []
    avg_rank_rows = []
    holm_rows = []
    nemenyi_rows = []
    effect_rows = []

    for cohort in cohorts:
        algorithms = cohort["algorithms"]
        wide = cohort["wide"]
        rank_wide = cohort["rank_wide"]
        raw_wide = cohort["raw_wide"]
        instance_count = int(len(wide))
        avg_ranks = rank_wide.mean(axis=0)
        for algorithm, avg_rank in avg_ranks.sort_values().items():
            avg_rank_rows.append(
                {
                    "cohort_id": cohort["cohort_id"],
                    "algorithm": algorithm,
                    "avg_rank": float(avg_rank),
                    "instance_count": instance_count,
                }
            )

        friedman_row = {
            "cohort_id": cohort["cohort_id"],
            "instance_count": instance_count,
            "algorithm_count": len(algorithms),
            "algorithms": ", ".join(algorithms),
            "status": "skipped",
            "statistic": math.nan,
            "p_value": math.nan,
            "reason": None,
        }
        if len(algorithms) < 3:
            friedman_row["reason"] = "Friedman requiere al menos 3 algoritmos."
        elif instance_count < 2:
            friedman_row["reason"] = "Friedman requiere al menos 2 instancias comparables."
        elif scipy_stats is None:
            friedman_row["reason"] = "SciPy no esta disponible."
        else:
            samples = [wide[algorithm].to_numpy(dtype=float) for algorithm in algorithms]
            statistic, p_value = scipy_stats.friedmanchisquare(*samples)
            friedman_row.update(
                {
                    "status": "ok",
                    "statistic": float(statistic),
                    "p_value": float(p_value),
                    "reason": None,
                }
            )
        friedman_rows.append(friedman_row)

        pairwise_rows = []
        for left, right in combinations(algorithms, 2):
            left_values = wide[left].to_numpy(dtype=float)
            right_values = wide[right].to_numpy(dtype=float)
            left_ranks = rank_wide[left].to_numpy(dtype=float)
            right_ranks = rank_wide[right].to_numpy(dtype=float)
            row = {
                "cohort_id": cohort["cohort_id"],
                "algorithm_a": left,
                "algorithm_b": right,
                "instance_count": instance_count,
                "mean_rank_a": float(avg_ranks[left]),
                "mean_rank_b": float(avg_ranks[right]),
                "a12_rank": _vargha_delaney_a12_from_preferences(left_ranks, right_ranks, lower_is_better=True),
                "status": "skipped",
                "statistic": math.nan,
                "p_value": math.nan,
                "p_holm": math.nan,
                "reason": None,
            }
            if instance_count < 2:
                row["reason"] = "Wilcoxon requiere al menos 2 instancias."
            elif scipy_stats is None:
                row["reason"] = "SciPy no esta disponible."
            elif np.allclose(left_values - right_values, 0.0, rtol=1e-12, atol=1e-12):
                row["reason"] = "Todas las diferencias pareadas son cero."
            else:
                try:
                    statistic, p_value = scipy_stats.wilcoxon(left_values, right_values, zero_method="wilcox")
                except ValueError as error:
                    row["reason"] = str(error)
                else:
                    row.update(
                        {
                            "status": "ok",
                            "statistic": float(statistic),
                            "p_value": float(p_value),
                            "reason": None,
                        }
                    )
            pairwise_rows.append(row)

            effect_rows.append(
                {
                    "cohort_id": cohort["cohort_id"],
                    "algorithm_a": left,
                    "algorithm_b": right,
                    "effect_metric": "A12_rank",
                    "effect_value": row["a12_rank"],
                    "instance_count": instance_count,
                }
            )

        _apply_holm_correction(pairwise_rows)
        holm_rows.extend(pairwise_rows)

        if friedman_row["status"] == "ok" and friedman_row["p_value"] < 0.05 and sp is not None and len(algorithms) >= 3:
            matrix = sp.posthoc_nemenyi_friedman(wide[algorithms])
            matrix = matrix.reindex(index=algorithms, columns=algorithms)
            for left in algorithms:
                for right in algorithms:
                    nemenyi_rows.append(
                        {
                            "cohort_id": cohort["cohort_id"],
                            "algorithm_a": left,
                            "algorithm_b": right,
                            "adjusted_p_value": float(matrix.loc[left, right]),
                        }
                    )
            cohort["nemenyi_matrix"] = matrix
        else:
            cohort["nemenyi_matrix"] = None

        cohort["average_ranks"] = avg_ranks
        cohort["friedman"] = friedman_row
        cohort["holm_rows"] = pairwise_rows
        cohort["effect_rows"] = [row for row in effect_rows if row["cohort_id"] == cohort["cohort_id"]]
        cohort["critical_difference"] = _critical_difference(len(algorithms), instance_count)
        cohort["raw_values"] = raw_wide

    return friedman_rows, avg_rank_rows, holm_rows, nemenyi_rows, effect_rows


def _pairwise_instance_wilcoxon(raw_df):
    columns = [
        "benchmark_id",
        "instance_id",
        "instance_key",
        "problem",
        "dimension",
        "metric",
        "algorithm_a",
        "algorithm_b",
        "common_seeds",
        "statistic",
        "p_value",
        "a12",
        "status",
        "reason",
    ]
    ok_df = raw_df[raw_df["status"] == "ok"].copy()
    if ok_df.empty:
        return pd.DataFrame(columns=columns)

    rows = []
    metric_specs = [
        ("final_fitness", True),
        ("wall_time_ms", True),
        ("cpu_time_ms", True),
    ]
    for instance_key, instance_df in ok_df.groupby("instance_key"):
        algorithms = sorted(instance_df["algorithm"].dropna().unique().tolist())
        metadata = instance_df.iloc[0]
        for metric, default_lower_is_better in metric_specs:
            metric_df = instance_df[["seed", "algorithm", metric, "objective_sense"]].dropna(subset=["seed", metric])
            if metric_df.empty:
                continue
            wide = metric_df.pivot_table(index="seed", columns="algorithm", values=metric, aggfunc="first")
            for left, right in combinations(algorithms, 2):
                if left not in wide.columns or right not in wide.columns:
                    continue
                pair = wide[[left, right]].dropna()
                objective_sense = str(metadata.get("objective_sense", "min"))
                lower_is_better = default_lower_is_better
                if metric == "final_fitness" and objective_sense == "max":
                    lower_is_better = False

                row = {
                    "benchmark_id": metadata.get("benchmark_id"),
                    "instance_id": metadata.get("instance_id"),
                    "instance_key": instance_key,
                    "problem": metadata.get("problem"),
                    "dimension": metadata.get("dimension"),
                    "metric": metric,
                    "algorithm_a": left,
                    "algorithm_b": right,
                    "common_seeds": int(len(pair)),
                    "statistic": math.nan,
                    "p_value": math.nan,
                    "a12": math.nan,
                    "status": "skipped",
                    "reason": None,
                }
                if len(pair) < 2:
                    row["reason"] = "Wilcoxon requiere al menos 2 seeds pareadas."
                elif scipy_stats is None:
                    row["reason"] = "SciPy no esta disponible."
                else:
                    left_values = pair[left].to_numpy(dtype=float)
                    right_values = pair[right].to_numpy(dtype=float)
                    transformed_left = left_values if lower_is_better else -left_values
                    transformed_right = right_values if lower_is_better else -right_values
                    if np.allclose(transformed_left - transformed_right, 0.0, rtol=1e-12, atol=1e-12):
                        row["reason"] = "Todas las diferencias pareadas son cero."
                    else:
                        try:
                            statistic, p_value = scipy_stats.wilcoxon(transformed_left, transformed_right, zero_method="wilcox")
                        except ValueError as error:
                            row["reason"] = str(error)
                        else:
                            row.update(
                                {
                                    "statistic": float(statistic),
                                    "p_value": float(p_value),
                                    "a12": _vargha_delaney_a12(left_values, right_values, lower_is_better=lower_is_better),
                                    "status": "ok",
                                    "reason": None,
                                }
                            )
                rows.append(row)

    return pd.DataFrame(rows, columns=columns)


def _write_analysis_tables(
    analysis_dir: Path,
    descriptive_df,
    friedman_rows,
    avg_rank_rows,
    holm_rows,
    nemenyi_rows,
    effect_rows,
    instance_wilcoxon_df,
):
    pd.DataFrame(friedman_rows).to_csv(analysis_dir / "friedman.csv", index=False)
    pd.DataFrame(avg_rank_rows).to_csv(analysis_dir / "average_ranks.csv", index=False)
    pd.DataFrame(holm_rows).to_csv(analysis_dir / "pairwise_holm.csv", index=False)
    pd.DataFrame(nemenyi_rows).to_csv(analysis_dir / "posthoc_nemenyi.csv", index=False)
    pd.DataFrame(effect_rows).to_csv(analysis_dir / "effect_sizes.csv", index=False)
    instance_wilcoxon_df.to_csv(analysis_dir / "instance_pairwise_wilcoxon.csv", index=False)


def _write_report_artifacts(
    reports_dir: Path,
    latex_dir: Path,
    raw_manifest,
    descriptive_df,
    friedman_rows,
    avg_rank_rows,
    holm_rows,
    nemenyi_rows,
    effect_rows,
    instance_wilcoxon_df,
):
    summary = {
        "runs": raw_manifest["run_count"],
        "ok_runs": raw_manifest["ok_run_count"],
        "benchmarks": raw_manifest["benchmark_count"],
        "instances": raw_manifest["instance_count"],
        "algorithms": raw_manifest["algorithm_count"],
        "friedman_ok": sum(1 for row in friedman_rows if row["status"] == "ok"),
        "friedman_significant": sum(
            1 for row in friedman_rows if row["status"] == "ok" and row["p_value"] < 0.05
        ),
    }
    _save_json(reports_dir / "summary.json", summary)

    markdown_lines = [
        "# Benchmark Statistical Pipeline",
        "",
        "## Dataset",
        f"- Raw runs: {raw_manifest['run_count']}",
        f"- Successful runs: {raw_manifest['ok_run_count']}",
        f"- Benchmarks: {raw_manifest['benchmark_count']}",
        f"- Instances: {raw_manifest['instance_count']}",
        f"- Algorithms: {raw_manifest['algorithm_count']}",
        f"- Convergence points: {raw_manifest['convergence_rows']}",
        "",
        "## Statistical tests",
    ]
    if friedman_rows:
        for row in friedman_rows:
            if row["status"] == "ok":
                markdown_lines.append(
                    f"- {row['cohort_id']}: Friedman statistic={row['statistic']:.4f}, p-value={row['p_value']:.4f}, instances={row['instance_count']}, algorithms={row['algorithm_count']}"
                )
            else:
                markdown_lines.append(f"- {row['cohort_id']}: skipped ({row['reason']})")
    else:
        markdown_lines.append("- No comparable cohorts were found.")
    markdown_lines.extend([
        "",
        "## Generated artifacts",
        "- `benchmark/raw/`: raw runs and exploded convergence histories",
        "- `benchmark/aggregated/`: per-instance and per-benchmark aggregates over seeds",
        "- `benchmark/analysis/`: descriptive tables, Friedman, Holm/Wilcoxon, Nemenyi and effect sizes",
        "- `benchmark/plots/`: boxplots, convergence plots, scatter plots and critical difference diagrams",
        "- `benchmark/reports/latex/`: LaTeX tables and figure snippets",
        "",
    ])
    (reports_dir / "report.md").write_text("\n".join(markdown_lines), encoding="utf-8")

    write_latex_table(
        latex_dir / "descriptive_summary.tex",
        "lllp{1.2cm}rrrr",
        ["Benchmark", "Instance", "Algorithm", "Sense", "Mean", "Median", "Std", "Best"],
        descriptive_df.values.tolist(),
        "Resumen descriptivo por benchmark y algoritmo.",
        "tab:benchmark:descriptive_summary",
    )
    write_latex_table(
        latex_dir / "friedman.tex",
        "lrrrll",
        ["Cohort", "Instances", "Algorithms", "Statistic", "p-value", "Status/Notes"],
        [
            [
                row["cohort_id"],
                row["instance_count"],
                row["algorithm_count"],
                row["statistic"],
                row["p_value"],
                row["status"] if row["reason"] is None else row["reason"],
            ]
            for row in friedman_rows
        ],
        "Resultados del test de Friedman sobre medianas por instancia.",
        "tab:benchmark:friedman",
    )
    write_latex_table(
        latex_dir / "average_ranks.tex",
        "llr",
        ["Cohort", "Algorithm", "Average rank"],
        [[row["cohort_id"], row["algorithm"], row["avg_rank"]] for row in avg_rank_rows],
        "Ranks medios por cohorte.",
        "tab:benchmark:average_ranks",
    )
    write_latex_table(
        latex_dir / "pairwise_holm.tex",
        "lllrrrr",
        ["Cohort", "Alg. A", "Alg. B", "Statistic", "p-value", "p-Holm", "A12 rank"],
        [
            [
                row["cohort_id"],
                row["algorithm_a"],
                row["algorithm_b"],
                row["statistic"],
                row["p_value"],
                row["p_holm"],
                row["a12_rank"],
            ]
            for row in holm_rows
        ],
        "Comparaciones pareadas entre algoritmos con correccion de Holm.",
        "tab:benchmark:pairwise_holm",
    )
    if nemenyi_rows:
        write_latex_table(
            latex_dir / "posthoc_nemenyi.tex",
            "llr",
            ["Cohort", "Comparison", "Adjusted p-value"],
            [
                [
                    row["cohort_id"],
                    f"{row['algorithm_a']} vs {row['algorithm_b']}",
                    row["adjusted_p_value"],
                ]
                for row in nemenyi_rows
                if row["algorithm_a"] != row["algorithm_b"]
            ],
            "Matriz post-hoc de Nemenyi para cohortes con Friedman significativo.",
            "tab:benchmark:posthoc_nemenyi",
        )
    if not instance_wilcoxon_df.empty:
        write_latex_table(
            latex_dir / "instance_pairwise_wilcoxon.tex",
            "lllllrrr",
            ["Benchmark", "Instance", "Metric", "Alg. A", "Alg. B", "Seeds", "p-value", "A12"],
            instance_wilcoxon_df[
                [
                    "benchmark_id",
                    "instance_id",
                    "metric",
                    "algorithm_a",
                    "algorithm_b",
                    "common_seeds",
                    "p_value",
                    "a12",
                ]
            ].values.tolist(),
            "Wilcoxon pareado por instancia usando seeds compartidas.",
            "tab:benchmark:instance_pairwise_wilcoxon",
        )


def _generate_plots(plots_dir: Path, raw_df, instance_summary_df, convergence_df, cohorts):
    if plt is None:
        _write_note(plots_dir / "plots.txt", "matplotlib no esta disponible; no se generaron graficos.")
        return

    plot_paths = []
    ok_df = raw_df[raw_df["status"] == "ok"].copy()
    for (benchmark_id, instance_id), group_df in ok_df.groupby(["benchmark_id", "instance_id"]):
        boxplot_name = f"{benchmark_id}_{instance_id}_boxplot"
        _plot_boxplot(group_df, plots_dir / f"{boxplot_name}.png", plots_dir / f"{boxplot_name}.svg")
        plot_paths.append(f"plots/{boxplot_name}.png")

        scatter_name = f"{benchmark_id}_{instance_id}_fitness_vs_time"
        scatter_df = instance_summary_df[
            (instance_summary_df["benchmark_id"] == benchmark_id)
            & (instance_summary_df["instance_id"] == instance_id)
        ]
        _plot_tradeoff_scatter(scatter_df, plots_dir / f"{scatter_name}.png", plots_dir / f"{scatter_name}.svg")
        plot_paths.append(f"plots/{scatter_name}.png")

    if not convergence_df.empty:
        for (benchmark_id, instance_id), group_df in convergence_df.groupby(["benchmark_id", "instance_id"]):
            convergence_name = f"{benchmark_id}_{instance_id}_convergence"
            _plot_convergence(group_df, plots_dir / f"{convergence_name}.png", plots_dir / f"{convergence_name}.svg")
            plot_paths.append(f"plots/{convergence_name}.png")

    for cohort in cohorts:
        avg_rank_path_png = plots_dir / f"{cohort['cohort_id']}_critical_difference.png"
        avg_rank_path_svg = plots_dir / f"{cohort['cohort_id']}_critical_difference.svg"
        _plot_critical_difference(cohort, avg_rank_path_png, avg_rank_path_svg)
        plot_paths.append(f"plots/{cohort['cohort_id']}_critical_difference.png")

    figure_lines = [
        "\\section*{Generated plots}",
    ]
    for relative_plot in plot_paths:
        figure_lines.extend(
            [
                "\\begin{figure}[htbp]",
                "\\centering",
                f"\\includegraphics[width=0.9\\linewidth]{{../{_latex_escape(relative_plot)}}}",
                f"\\caption{{{_latex_escape(relative_plot)}}}",
                "\\end{figure}",
                "",
            ]
        )
    (plots_dir.parent / "reports" / "latex" / "plots.tex").write_text("\n".join(figure_lines) + "\n", encoding="utf-8")


def _plot_boxplot(group_df, png_path: Path, svg_path: Path):
    algorithms = sorted(group_df["algorithm"].unique().tolist())
    data = [group_df[group_df["algorithm"] == algorithm]["final_fitness"].dropna().to_numpy(dtype=float) for algorithm in algorithms]
    if not any(len(series) for series in data):
        return

    fig, ax = plt.subplots(figsize=(10, 5))
    ax.boxplot(data, labels=algorithms, showmeans=True)
    ax.set_title(f"Final fitness by algorithm: {group_df.iloc[0]['benchmark_id']} / {group_df.iloc[0]['instance_id']}")
    ax.set_ylabel("final_fitness")
    ax.tick_params(axis="x", rotation=20)
    fig.tight_layout()
    fig.savefig(png_path, dpi=160)
    fig.savefig(svg_path)
    plt.close(fig)


def _plot_tradeoff_scatter(group_df, png_path: Path, svg_path: Path):
    if group_df.empty:
        return

    fig, ax = plt.subplots(figsize=(8, 5))
    for _, row in group_df.iterrows():
        x_value = row.get("median_wall_time_ms")
        y_value = row.get("median_final_fitness")
        if pd.isna(x_value) or pd.isna(y_value):
            continue
        ax.scatter([x_value], [y_value], label=row["algorithm"], s=60)
        ax.annotate(row["algorithm"], (x_value, y_value), textcoords="offset points", xytext=(4, 4))
    ax.set_xlabel("median wall_time_ms")
    ax.set_ylabel("median final_fitness")
    ax.set_title(f"Fitness vs time: {group_df.iloc[0]['benchmark_id']} / {group_df.iloc[0]['instance_id']}")
    fig.tight_layout()
    fig.savefig(png_path, dpi=160)
    fig.savefig(svg_path)
    plt.close(fig)


def _plot_convergence(group_df, png_path: Path, svg_path: Path):
    fig, ax = plt.subplots(figsize=(10, 5))
    for algorithm, algorithm_df in group_df.groupby("algorithm"):
        series = algorithm_df.groupby("evaluation")["best_fitness_so_far"].median().sort_index()
        ax.plot(series.index.to_numpy(dtype=float), series.to_numpy(dtype=float), label=algorithm)
    ax.set_xlabel("evaluation")
    ax.set_ylabel("best_fitness_so_far (median over seeds)")
    ax.set_title(f"Convergence: {group_df.iloc[0]['benchmark_id']} / {group_df.iloc[0]['instance_id']}")
    ax.legend(loc="best")
    fig.tight_layout()
    fig.savefig(png_path, dpi=160)
    fig.savefig(svg_path)
    plt.close(fig)


def _plot_critical_difference(cohort, png_path: Path, svg_path: Path):
    average_ranks = cohort["average_ranks"].sort_values()
    matrix = cohort.get("nemenyi_matrix")
    if sp is not None and matrix is not None and hasattr(sp, "critical_difference_diagram"):
        fig, ax = plt.subplots(figsize=(10, 3.5))
        sp.critical_difference_diagram(average_ranks, matrix, ax=ax)
        fig.tight_layout()
        fig.savefig(png_path, dpi=160)
        fig.savefig(svg_path)
        plt.close(fig)
        return

    fig, ax = plt.subplots(figsize=(10, 3.5))
    algorithms = list(average_ranks.index)
    ranks = average_ranks.to_numpy(dtype=float)
    min_rank = 1.0
    max_rank = max(float(len(algorithms)), math.ceil(float(np.nanmax(ranks))) if len(ranks) else 1.0)
    ax.hlines(0.5, min_rank, max_rank, color="black")
    for tick in np.arange(min_rank, max_rank + 0.01, 0.5):
        ax.vlines(tick, 0.46, 0.54, color="black", linewidth=0.6)
    for index, (algorithm, rank) in enumerate(zip(algorithms, ranks), start=1):
        y_value = 0.7 + (index % 2) * 0.15
        ax.plot([rank, rank], [0.5, y_value], color="black", linewidth=1.0)
        ax.text(rank, y_value + 0.02, f"{algorithm} ({rank:.2f})", rotation=25, ha="center", va="bottom")

    cd_value = cohort.get("critical_difference")
    if cd_value is not None:
        cd_start = min_rank
        cd_end = min(max_rank, cd_start + cd_value)
        ax.plot([cd_start, cd_end], [0.25, 0.25], color="black", linewidth=2.0)
        ax.vlines([cd_start, cd_end], 0.21, 0.29, color="black", linewidth=1.5)
        ax.text((cd_start + cd_end) / 2.0, 0.14, f"CD = {cd_value:.2f}", ha="center")

    ax.set_title(f"Critical difference diagram: {cohort['cohort_id']}")
    ax.set_ylim(0.05, 1.2)
    ax.set_xlim(min_rank - 0.1, max_rank + 0.1)
    ax.set_yticks([])
    ax.set_xlabel("Average rank (1 = best)")
    for spine in ["left", "right", "top"]:
        ax.spines[spine].set_visible(False)
    fig.tight_layout()
    fig.savefig(png_path, dpi=160)
    fig.savefig(svg_path)
    plt.close(fig)


def _critical_difference(algorithm_count: int, instance_count: int):
    if algorithm_count < 2 or instance_count < 2:
        return None
    q_alpha = CD_Q_ALPHA_005.get(algorithm_count)
    if q_alpha is None:
        return None
    return q_alpha * math.sqrt(algorithm_count * (algorithm_count + 1) / (6.0 * instance_count))


def _vargha_delaney_a12(sample_a, sample_b, lower_is_better=True):
    values_a = [float(value) for value in sample_a]
    values_b = [float(value) for value in sample_b]
    if not values_a or not values_b:
        return math.nan

    wins = 0.0
    total = 0
    for left in values_a:
        for right in values_b:
            total += 1
            if math.isclose(left, right, rel_tol=1e-12, abs_tol=1e-12):
                wins += 0.5
            elif (left < right and lower_is_better) or (left > right and not lower_is_better):
                wins += 1.0
    return wins / total if total else math.nan


def _vargha_delaney_a12_from_preferences(sample_a, sample_b, lower_is_better=True):
    values_a = [float(value) for value in sample_a]
    values_b = [float(value) for value in sample_b]
    if len(values_a) != len(values_b) or not values_a:
        return math.nan

    wins = 0.0
    for left, right in zip(values_a, values_b):
        if math.isclose(left, right, rel_tol=1e-12, abs_tol=1e-12):
            wins += 0.5
        elif (left < right and lower_is_better) or (left > right and not lower_is_better):
            wins += 1.0
    return wins / len(values_a)


def _apply_holm_correction(rows):
    valid_rows = [row for row in rows if row.get("status") == "ok" and not math.isnan(row.get("p_value", math.nan))]
    valid_rows.sort(key=lambda row: row["p_value"])
    running_max = 0.0
    total = len(valid_rows)
    for index, row in enumerate(valid_rows):
        adjusted = min(1.0, max(running_max, (total - index) * row["p_value"]))
        row["p_holm"] = adjusted
        running_max = adjusted
    for row in rows:
        row.setdefault("p_holm", math.nan)


def _ranking_value(value, objective_sense):
    if value is None or (isinstance(value, float) and math.isnan(value)):
        return math.nan
    return -float(value) if objective_sense == "max" else float(value)


def _best_value(values, objective_sense):
    if len(values) == 0:
        return math.nan
    return float(np.max(values) if objective_sense == "max" else np.min(values))


def _safe_mean(values):
    return float(np.mean(values)) if len(values) else math.nan


def _safe_median(values):
    return float(np.median(values)) if len(values) else math.nan


def _safe_std(values):
    return float(np.std(values)) if len(values) else math.nan


def _safe_min(values):
    return float(np.min(values)) if len(values) else math.nan


def _safe_max(values):
    return float(np.max(values)) if len(values) else math.nan


def _safe_percentile(values, percentile):
    return float(np.percentile(values, percentile)) if len(values) else math.nan


def _safe_iqr(values):
    if len(values) == 0:
        return math.nan
    q75, q25 = np.percentile(values, [75, 25])
    return float(q75 - q25)


def _save_json(path: Path, payload):
    path.parent.mkdir(parents=True, exist_ok=True)
    path.write_text(json.dumps(payload, indent=2), encoding="utf-8")


def _write_note(path: Path, note: str):
    path.parent.mkdir(parents=True, exist_ok=True)
    path.write_text(note + "\n", encoding="utf-8")


def write_latex_table(path: Path, column_spec, header, rows, caption, label):
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
    lines.extend(["\\hline", "\\end{tabular}", "\\end{table}", ""])
    path.write_text("\n".join(lines), encoding="utf-8")


def _format_cell(value):
    if value is None:
        return "--"
    if isinstance(value, float) and math.isnan(value):
        return "--"
    if isinstance(value, (int, float, np.integer, np.floating)):
        numeric_value = float(value)
        if math.isclose(numeric_value, round(numeric_value), abs_tol=1e-9):
            return str(int(round(numeric_value)))
        return f"{numeric_value:.4f}"
    return _latex_escape(value)


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