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


RESULTS_SUBDIRS = ["raw", "aggregated", "analysis", "plots", "latex"]
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


def generate_benchmark_analysis(benchmark_root: Path, summary: dict | None = None):
    results_dir = benchmark_root / "results"
    runs_path = results_dir / "runs.csv"
    if not runs_path.exists():
        return

    _ensure_results_dirs(results_dir)
    if pd is None or np is None:
        note = (
            "El analisis estadistico local requiere pandas y numpy. "
            "Instala benchmark_suite/requirements-analysis.txt para regenerarlo."
        )
        _write_note(results_dir / "latex" / "analysis_notes.tex", note)
        (results_dir / "report.md").write_text(note + "\n", encoding="utf-8")
        return

    raw_rows = read_rows_csv(runs_path)
    if not raw_rows:
        note = "No hay ejecuciones disponibles en results/runs.csv para construir el analisis local."
        _write_note(results_dir / "latex" / "analysis_notes.tex", note)
        (results_dir / "report.md").write_text(note + "\n", encoding="utf-8")
        return

    raw_df = _normalize_raw_dataframe(pd.DataFrame(raw_rows))
    convergence_df = _extract_convergence_dataframe(raw_df)
    summary_payload = summary or _load_summary(results_dir / "summary.json")

    _write_raw_artifacts(results_dir, raw_df, convergence_df)

    algorithm_summary_df = _aggregate_algorithm(raw_df)
    instance_summary_df = _aggregate_instance_algorithm(raw_df)
    descriptive_df = algorithm_summary_df[
        ["algorithm", "mean", "median", "std", "best"]
    ].copy()

    ranks_df, friedman_df, holm_df, nemenyi_df, instance_effect_df = _instance_level_analysis(instance_summary_df)
    seed_wilcoxon_df, seed_effect_df = _seed_level_analysis(raw_df)
    effect_sizes_df = pd.concat([seed_effect_df, instance_effect_df], ignore_index=True, sort=False)

    _write_aggregated_artifacts(results_dir, algorithm_summary_df, instance_summary_df)
    _write_analysis_artifacts(
        results_dir,
        descriptive_df,
        friedman_df,
        ranks_df,
        holm_df,
        nemenyi_df,
        seed_wilcoxon_df,
        effect_sizes_df,
    )

    figure_payload = _generate_plots(
        results_dir,
        raw_df,
        convergence_df,
        algorithm_summary_df,
        ranks_df,
        friedman_df,
    )
    _write_latex_artifacts(
        results_dir,
        raw_df,
        summary_payload,
        descriptive_df,
        friedman_df,
        ranks_df,
        holm_df,
        nemenyi_df,
        seed_wilcoxon_df,
        effect_sizes_df,
        figure_payload,
    )
    _write_markdown_report(
        results_dir,
        raw_df,
        descriptive_df,
        friedman_df,
        ranks_df,
        seed_wilcoxon_df,
        figure_payload,
    )


def _ensure_results_dirs(results_dir: Path):
    for subdir in RESULTS_SUBDIRS:
        subdir_path = results_dir / subdir
        if subdir_path.exists() and not subdir_path.is_dir():
            subdir_path.unlink()
        if subdir_path.exists() and not _is_writable_directory(subdir_path):
            stale_path = _next_stale_path(subdir_path)
            subdir_path.rename(stale_path)
        subdir_path.mkdir(parents=True, exist_ok=True)


def _is_writable_directory(path: Path):
    probe = path / ".write_test"
    try:
        probe.write_text("ok", encoding="utf-8")
    except OSError:
        return False
    probe.unlink()
    return True


def _next_stale_path(path: Path):
    suffix = 1
    while True:
        candidate = path.with_name(f"{path.name}.stale_root_{suffix}")
        if not candidate.exists():
            return candidate
        suffix += 1


def _load_summary(summary_path: Path):
    if not summary_path.exists():
        return {}
    return json.loads(summary_path.read_text(encoding="utf-8"))


def _normalize_raw_dataframe(df):
    normalized = df.copy()
    for column in [
        "benchmark_id",
        "problem",
        "instance_id",
        "algorithm",
        "objective_sense",
        "status",
        "budget_type",
        "final_fitness",
        "best_fitness",
        "wall_time_ms",
        "cpu_time_ms",
        "seed",
        "budget_value",
        "success",
        "evaluations",
        "convergence_history",
    ]:
        if column not in normalized.columns:
            normalized[column] = None

    normalized["final_fitness"] = normalized["final_fitness"].fillna(normalized["best_fitness"])
    normalized["objective_sense"] = normalized["objective_sense"].fillna("min")
    normalized["success"] = normalized["success"].where(normalized["success"].notna(), normalized["status"] == "ok")
    normalized["success"] = normalized["success"].astype(bool)
    normalized["instance_key"] = normalized.apply(
        lambda row: f"{row['benchmark_id']}::{row['instance_id']}",
        axis=1,
    )

    for column in [
        "dimension",
        "seed",
        "budget_value",
        "final_fitness",
        "best_fitness",
        "wall_time_ms",
        "cpu_time_ms",
        "evaluations",
        "returncode",
    ]:
        normalized[column] = pd.to_numeric(normalized[column], errors="coerce")

    normalized["evaluations"] = normalized["evaluations"].where(
        normalized["evaluations"].notna(),
        np.where(
            (normalized["budget_type"] == "evaluations") & (normalized["status"] == "ok"),
            normalized["budget_value"],
            np.nan,
        ),
    )
    return normalized


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
                    "problem": row["problem"],
                    "instance_id": row["instance_id"],
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


def _aggregate_algorithm(raw_df):
    ok_df = raw_df[raw_df["status"] == "ok"].copy()
    if ok_df.empty:
        return pd.DataFrame(columns=["algorithm", "mean", "median", "std", "best"])

    rows = []
    for algorithm, group_df in ok_df.groupby("algorithm", sort=True):
        objective_sense = group_df["objective_sense"].iloc[0]
        fitness_values = group_df["final_fitness"].dropna().to_numpy(dtype=float)
        wall_values = group_df["wall_time_ms"].dropna().to_numpy(dtype=float)
        cpu_values = group_df["cpu_time_ms"].dropna().to_numpy(dtype=float)
        eval_values = group_df["evaluations"].dropna().to_numpy(dtype=float)
        rows.append(
            {
                "algorithm": algorithm,
                "objective_sense": objective_sense,
                "run_count": int(len(group_df)),
                "seed_count": int(group_df["seed"].dropna().nunique()),
                "success_rate": float(group_df["success"].astype(float).mean()) if len(group_df) else math.nan,
                "mean": _safe_mean(fitness_values),
                "median": _safe_median(fitness_values),
                "std": _safe_std(fitness_values),
                "best": _best_value(fitness_values, objective_sense),
                "min": _safe_min(fitness_values),
                "max": _safe_max(fitness_values),
                "p90": _safe_percentile(fitness_values, 90),
                "iqr": _safe_iqr(fitness_values),
                "mean_wall_time_ms": _safe_mean(wall_values),
                "median_wall_time_ms": _safe_median(wall_values),
                "p90_wall_time_ms": _safe_percentile(wall_values, 90),
                "mean_cpu_time_ms": _safe_mean(cpu_values),
                "median_cpu_time_ms": _safe_median(cpu_values),
                "mean_evaluations": _safe_mean(eval_values),
                "median_evaluations": _safe_median(eval_values),
            }
        )
    return pd.DataFrame(rows).sort_values("algorithm").reset_index(drop=True)


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
        "budget_type",
        "budget_value",
    ]
    rows = []
    for key, group_df in ok_df.groupby(group_columns, dropna=False):
        payload = dict(zip(group_columns, key))
        fitness_values = group_df["final_fitness"].dropna().to_numpy(dtype=float)
        wall_values = group_df["wall_time_ms"].dropna().to_numpy(dtype=float)
        cpu_values = group_df["cpu_time_ms"].dropna().to_numpy(dtype=float)
        payload.update(
            {
                "seed_count": int(group_df["seed"].dropna().nunique()),
                "median_final_fitness": _safe_median(fitness_values),
                "mean_final_fitness": _safe_mean(fitness_values),
                "std_final_fitness": _safe_std(fitness_values),
                "best_final_fitness": _best_value(fitness_values, payload["objective_sense"]),
                "median_wall_time_ms": _safe_median(wall_values),
                "median_cpu_time_ms": _safe_median(cpu_values),
            }
        )
        rows.append(payload)
    return pd.DataFrame(rows)


def _instance_level_analysis(instance_summary_df):
    if instance_summary_df.empty:
        empty = pd.DataFrame()
        friedman = pd.DataFrame([
            {
                "status": "skipped",
                "instance_count": 0,
                "algorithm_count": 0,
                "statistic": math.nan,
                "p_value": math.nan,
                "reason": "No hay agregados por instancia disponibles.",
            }
        ])
        return empty, friedman, empty, empty, empty

    algorithms = sorted(instance_summary_df["algorithm"].unique().tolist())
    wide = (
        instance_summary_df.pivot(index="instance_key", columns="algorithm", values="median_final_fitness")
        .reindex(columns=algorithms)
        .dropna(axis=0, how="any")
    )
    objective_sense = instance_summary_df["objective_sense"].iloc[0]

    if wide.empty:
        empty = pd.DataFrame()
        friedman = pd.DataFrame([
            {
                "status": "skipped",
                "instance_count": 0,
                "algorithm_count": len(algorithms),
                "statistic": math.nan,
                "p_value": math.nan,
                "reason": "No hay instancias completas compartidas entre todas las bibliotecas.",
            }
        ])
        return empty, friedman, empty, empty, empty

    rank_wide = wide.rank(axis=1, method="average", ascending=(objective_sense == "min"))
    ranks_df = (
        rank_wide.mean(axis=0)
        .rename("avg_rank")
        .reset_index()
        .rename(columns={"index": "algorithm"})
        .sort_values("avg_rank")
        .reset_index(drop=True)
    )
    ranks_df["instance_count"] = int(len(wide))

    friedman_row = {
        "status": "skipped",
        "instance_count": int(len(wide)),
        "algorithm_count": len(algorithms),
        "statistic": math.nan,
        "p_value": math.nan,
        "reason": None,
    }
    if len(algorithms) < 3:
        friedman_row["reason"] = "Friedman requiere al menos 3 bibliotecas."
    elif len(wide) < 2:
        friedman_row["reason"] = "Friedman requiere al menos 2 instancias comparables por problema."
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
    friedman_df = pd.DataFrame([friedman_row])

    holm_rows = []
    effect_rows = []
    for left, right in combinations(algorithms, 2):
        pair_row = {
            "algorithm_a": left,
            "algorithm_b": right,
            "instance_count": int(len(wide)),
            "status": "skipped",
            "statistic": math.nan,
            "p_value": math.nan,
            "p_holm": math.nan,
            "reason": None,
        }
        values_left = wide[left].to_numpy(dtype=float)
        values_right = wide[right].to_numpy(dtype=float)
        if len(wide) < 2:
            pair_row["reason"] = "Wilcoxon requiere al menos 2 instancias agregadas."
        elif scipy_stats is None:
            pair_row["reason"] = "SciPy no esta disponible."
        elif np.allclose(values_left - values_right, 0.0, rtol=1e-12, atol=1e-12):
            pair_row["reason"] = "Todas las diferencias pareadas por instancia son cero."
        else:
            statistic, p_value = scipy_stats.wilcoxon(values_left, values_right, zero_method="wilcox")
            pair_row.update(
                {
                    "status": "ok",
                    "statistic": float(statistic),
                    "p_value": float(p_value),
                    "reason": None,
                }
            )
        holm_rows.append(pair_row)
        effect_rows.append(
            {
                "scope": "instance_median",
                "metric": "final_fitness",
                "algorithm_a": left,
                "algorithm_b": right,
                "pair_count": int(len(wide)),
                "effect_size": _vargha_delaney_a12(
                    values_left,
                    values_right,
                    lower_is_better=(objective_sense == "min"),
                ),
            }
        )
    _apply_holm_correction(holm_rows)
    holm_df = pd.DataFrame(holm_rows)

    nemenyi_rows = []
    if (
        friedman_row["status"] == "ok"
        and friedman_row["p_value"] < 0.05
        and sp is not None
        and len(algorithms) >= 3
        and len(wide) >= 2
    ):
        matrix = sp.posthoc_nemenyi_friedman(wide[algorithms])
        matrix = matrix.reindex(index=algorithms, columns=algorithms)
        for left in algorithms:
            for right in algorithms:
                nemenyi_rows.append(
                    {
                        "algorithm_a": left,
                        "algorithm_b": right,
                        "adjusted_p_value": float(matrix.loc[left, right]),
                    }
                )
    nemenyi_df = pd.DataFrame(nemenyi_rows)
    return ranks_df, friedman_df, holm_df, nemenyi_df, pd.DataFrame(effect_rows)


def _seed_level_analysis(raw_df):
    ok_df = raw_df[raw_df["status"] == "ok"].copy()
    if ok_df.empty:
        return pd.DataFrame(), pd.DataFrame()

    benchmark_id = ok_df["benchmark_id"].iloc[0]
    instance_id = ok_df["instance_id"].iloc[0]
    objective_sense = ok_df["objective_sense"].iloc[0]
    algorithms = sorted(ok_df["algorithm"].unique().tolist())
    metric_specs = [
        ("final_fitness", objective_sense == "min"),
        ("wall_time_ms", True),
        ("cpu_time_ms", True),
    ]

    wilcoxon_rows = []
    effect_rows = []
    for metric, lower_is_better in metric_specs:
        metric_df = ok_df[["seed", "algorithm", metric]].dropna(subset=["seed", metric])
        if metric_df.empty:
            continue
        wide = metric_df.pivot_table(index="seed", columns="algorithm", values=metric, aggfunc="first")
        for left, right in combinations(algorithms, 2):
            if left not in wide.columns or right not in wide.columns:
                continue
            pair = wide[[left, right]].dropna()
            row = {
                "benchmark_id": benchmark_id,
                "instance_id": instance_id,
                "metric": metric,
                "algorithm_a": left,
                "algorithm_b": right,
                "paired_seeds": int(len(pair)),
                "status": "skipped",
                "statistic": math.nan,
                "p_value": math.nan,
                "reason": None,
            }
            left_values = pair[left].to_numpy(dtype=float)
            right_values = pair[right].to_numpy(dtype=float)
            transformed_left = left_values if lower_is_better else -left_values
            transformed_right = right_values if lower_is_better else -right_values
            if len(pair) < 2:
                row["reason"] = "Wilcoxon requiere al menos 2 seeds pareadas."
            elif scipy_stats is None:
                row["reason"] = "SciPy no esta disponible."
            elif np.allclose(transformed_left - transformed_right, 0.0, rtol=1e-12, atol=1e-12):
                row["reason"] = "Todas las diferencias pareadas por seed son cero."
            else:
                statistic, p_value = scipy_stats.wilcoxon(transformed_left, transformed_right, zero_method="wilcox")
                row.update(
                    {
                        "status": "ok",
                        "statistic": float(statistic),
                        "p_value": float(p_value),
                        "reason": None,
                    }
                )
            wilcoxon_rows.append(row)
            effect_rows.append(
                {
                    "scope": "paired_seed",
                    "metric": metric,
                    "algorithm_a": left,
                    "algorithm_b": right,
                    "pair_count": int(len(pair)),
                    "effect_size": _vargha_delaney_a12(left_values, right_values, lower_is_better=lower_is_better),
                }
            )
    return pd.DataFrame(wilcoxon_rows), pd.DataFrame(effect_rows)


def _write_raw_artifacts(results_dir: Path, raw_df, convergence_df):
    raw_df.to_csv(results_dir / "raw" / "runs.csv", index=False)
    convergence_df.to_csv(results_dir / "raw" / "convergence.csv", index=False)
    manifest = {
        "benchmark_id": _first_non_empty(raw_df, "benchmark_id"),
        "problem": _first_non_empty(raw_df, "problem"),
        "instance_count": int(raw_df["instance_key"].nunique()),
        "algorithm_count": int(raw_df["algorithm"].nunique()),
        "run_count": int(len(raw_df)),
        "ok_run_count": int((raw_df["status"] == "ok").sum()),
        "convergence_rows": int(len(convergence_df)),
    }
    _save_json(results_dir / "raw" / "manifest.json", manifest)


def _write_aggregated_artifacts(results_dir: Path, algorithm_summary_df, instance_summary_df):
    algorithm_summary_df.to_csv(results_dir / "aggregated" / "algorithm_summary.csv", index=False)
    instance_summary_df.to_csv(results_dir / "aggregated" / "instance_algorithm_summary.csv", index=False)


def _write_analysis_artifacts(
    results_dir: Path,
    descriptive_df,
    friedman_df,
    ranks_df,
    holm_df,
    nemenyi_df,
    seed_wilcoxon_df,
    effect_sizes_df,
):
    descriptive_df.to_csv(results_dir / "analysis" / "descriptive_summary.csv", index=False)
    friedman_df.to_csv(results_dir / "analysis" / "friedman.csv", index=False)
    ranks_df.to_csv(results_dir / "analysis" / "average_ranks.csv", index=False)
    holm_df.to_csv(results_dir / "analysis" / "posthoc_holm.csv", index=False)
    nemenyi_df.to_csv(results_dir / "analysis" / "posthoc_nemenyi.csv", index=False)
    seed_wilcoxon_df.to_csv(results_dir / "analysis" / "pairwise_wilcoxon.csv", index=False)
    effect_sizes_df.to_csv(results_dir / "analysis" / "effect_sizes.csv", index=False)
    _save_json(
        results_dir / "analysis" / "summary.json",
        {
            "descriptive_rows": int(len(descriptive_df)),
            "friedman_rows": int(len(friedman_df)),
            "average_rank_rows": int(len(ranks_df)),
            "holm_rows": int(len(holm_df)),
            "nemenyi_rows": int(len(nemenyi_df)),
            "pairwise_wilcoxon_rows": int(len(seed_wilcoxon_df)),
            "effect_size_rows": int(len(effect_sizes_df)),
        },
    )


def _generate_plots(results_dir: Path, raw_df, convergence_df, algorithm_summary_df, ranks_df, friedman_df):
    plot_payload = {
        "boxplot": None,
        "convergence": None,
        "critical_difference": None,
        "scatter": None,
    }
    if plt is None:
        return plot_payload

    ok_df = raw_df[raw_df["status"] == "ok"].copy()
    if not ok_df.empty:
        boxplot_png = results_dir / "plots" / "boxplot_fitness.png"
        boxplot_svg = results_dir / "plots" / "boxplot_fitness.svg"
        _plot_boxplot(ok_df, boxplot_png, boxplot_svg)
        plot_payload["boxplot"] = {
            "png": boxplot_png,
            "svg": boxplot_svg,
            "caption": "Boxplot de final_fitness por biblioteca sobre las seeds del benchmark.",
            "label": f"fig:{_first_non_empty(raw_df, 'benchmark_id')}:boxplot",
        }

        scatter_png = results_dir / "plots" / "fitness_vs_time.png"
        scatter_svg = results_dir / "plots" / "fitness_vs_time.svg"
        _plot_scatter(algorithm_summary_df, scatter_png, scatter_svg)
        plot_payload["scatter"] = {
            "png": scatter_png,
            "svg": scatter_svg,
            "caption": "Trade-off entre mediana de fitness y mediana de wall-clock time por biblioteca.",
            "label": f"fig:{_first_non_empty(raw_df, 'benchmark_id')}:fitness_vs_time",
        }

    if not convergence_df.empty:
        convergence_png = results_dir / "plots" / "convergence.png"
        convergence_svg = results_dir / "plots" / "convergence.svg"
        _plot_convergence(convergence_df, convergence_png, convergence_svg)
        plot_payload["convergence"] = {
            "png": convergence_png,
            "svg": convergence_svg,
            "caption": "Curvas de convergencia agregadas como mediana sobre seeds.",
            "label": f"fig:{_first_non_empty(raw_df, 'benchmark_id')}:convergence",
        }

    if _can_plot_critical_difference(ranks_df, friedman_df):
        cd_png = results_dir / "plots" / "critical_difference.png"
        cd_svg = results_dir / "plots" / "critical_difference.svg"
        _plot_critical_difference(ranks_df, friedman_df, cd_png, cd_svg)
        plot_payload["critical_difference"] = {
            "png": cd_png,
            "svg": cd_svg,
            "caption": "Critical difference diagram sobre ranks medios por instancia agregada.",
            "label": f"fig:{_first_non_empty(raw_df, 'benchmark_id')}:critical_difference",
        }

    return plot_payload


def _write_latex_artifacts(
    results_dir: Path,
    raw_df,
    summary_payload,
    descriptive_df,
    friedman_df,
    ranks_df,
    holm_df,
    nemenyi_df,
    seed_wilcoxon_df,
    effect_sizes_df,
    figure_payload,
):
    latex_dir = results_dir / "latex"
    benchmark_id = _first_non_empty(raw_df, "benchmark_id") or results_dir.parent.name

    fitness_rows = []
    timing_rows = []
    libraries = summary_payload.get("libraries", {}) if isinstance(summary_payload, dict) else {}
    if libraries:
        for algorithm, payload in sorted(libraries.items()):
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
    else:
        fitness_rows = descriptive_df.values.tolist()

    write_latex_table(
        latex_dir / "fitness_summary.tex",
        "llrrrrrrrr",
        ["Algorithm", "Status", "OK runs", "Best", "Mean", "Median", "Worst", "Std. dev.", "IQR", "MAD"],
        fitness_rows,
        f"Resumen descriptivo de fitness para {benchmark_id}.",
        f"tab:{benchmark_id}:fitness_summary",
    )
    write_latex_table(
        latex_dir / "timing_summary.tex",
        "llrrrrrr",
        ["Algorithm", "Status", "Runner ms", "Mean wall ms", "Median wall ms", "P90 wall ms", "Mean CPU ms", "Median CPU ms"],
        timing_rows,
        f"Resumen temporal para {benchmark_id}.",
        f"tab:{benchmark_id}:timing_summary",
    )
    write_latex_table(
        latex_dir / "descriptive_summary.tex",
        "lrrrr",
        ["Algorithm", "Mean", "Median", "Std", "Best"],
        descriptive_df.values.tolist(),
        "Tabla descriptiva agregada sobre seeds por biblioteca.",
        f"tab:{benchmark_id}:descriptive",
    )
    write_latex_table(
        latex_dir / "friedman.tex",
        "rrrll",
        ["Instances", "Algorithms", "Statistic", "p-value", "Status/Notes"],
        [
            [
                row["instance_count"],
                row["algorithm_count"],
                row["statistic"],
                row["p_value"],
                row["status"] if row.get("reason") is None else row["reason"],
            ]
            for row in friedman_df.to_dict(orient="records")
        ],
        "Resultado del test de Friedman sobre medianas por instancia.",
        f"tab:{benchmark_id}:friedman",
    )
    write_latex_table(
        latex_dir / "average_ranks.tex",
        "lr",
        ["Algorithm", "Average rank"],
        ranks_df[["algorithm", "avg_rank"]].values.tolist() if not ranks_df.empty else [],
        "Ranks medios por biblioteca sobre instancias agregadas.",
        f"tab:{benchmark_id}:average_ranks",
    )
    write_latex_table(
        latex_dir / "posthoc_holm.tex",
        "llrrrrp{4cm}",
        ["Alg. A", "Alg. B", "Instances", "Statistic", "p-value", "p-Holm", "Notes"],
        [
            [
                row.get("algorithm_a"),
                row.get("algorithm_b"),
                row.get("instance_count"),
                row.get("statistic"),
                row.get("p_value"),
                row.get("p_holm"),
                row.get("reason"),
            ]
            for row in holm_df.to_dict(orient="records")
        ] if not holm_df.empty else [],
        "Comparaciones post-hoc entre bibliotecas sobre medianas por instancia con correccion de Holm.",
        f"tab:{benchmark_id}:holm",
    )
    write_latex_table(
        latex_dir / "posthoc_nemenyi.tex",
        "llr",
        ["Alg. A", "Alg. B", "Adjusted p-value"],
        nemenyi_df.values.tolist() if not nemenyi_df.empty else [],
        "Matriz post-hoc de Nemenyi cuando Friedman rechaza H0.",
        f"tab:{benchmark_id}:nemenyi",
    )
    write_latex_table(
        latex_dir / "pairwise_wilcoxon.tex",
        "lllrrp{4cm}",
        ["Metric", "Alg. A", "Alg. B", "Seeds", "p-value", "Notes"],
        [
            [
                row.get("metric"),
                row.get("algorithm_a"),
                row.get("algorithm_b"),
                row.get("paired_seeds"),
                row.get("p_value"),
                row.get("reason"),
            ]
            for row in seed_wilcoxon_df.to_dict(orient="records")
        ] if not seed_wilcoxon_df.empty else [],
        "Wilcoxon signed-rank sobre seeds pareadas dentro del problema.",
        f"tab:{benchmark_id}:wilcoxon",
    )
    write_latex_table(
        latex_dir / "effect_sizes.tex",
        "llllr",
        ["Scope", "Metric", "Alg. A", "Alg. B", "A12"],
        effect_sizes_df[["scope", "metric", "algorithm_a", "algorithm_b", "effect_size"]].values.tolist() if not effect_sizes_df.empty else [],
        "Tamano de efecto Vargha-Delaney A12 para comparaciones pareadas relevantes.",
        f"tab:{benchmark_id}:effect_sizes",
    )

    _write_figure_snippet(
        latex_dir / "boxplot.tex",
        figure_payload.get("boxplot"),
        fallback_note="No hay datos suficientes para construir el boxplot de fitness.",
    )
    _write_figure_snippet(
        latex_dir / "convergence.tex",
        figure_payload.get("convergence"),
        fallback_note="No hay convergence history en los resultados raw; el benchmark no puede construir la figura de convergencia todavia.",
    )
    _write_figure_snippet(
        latex_dir / "critical_difference.tex",
        figure_payload.get("critical_difference"),
        fallback_note="No hay suficientes instancias comparables para un critical difference diagram cientificamente valido; Friedman requiere al menos 2 instancias compartidas y 3 bibliotecas.",
    )
    _write_figure_snippet(
        latex_dir / "scatter_fitness_vs_time.tex",
        figure_payload.get("scatter"),
        fallback_note="No hay datos suficientes para construir el scatter fitness vs tiempo.",
    )


def _write_markdown_report(results_dir: Path, raw_df, descriptive_df, friedman_df, ranks_df, seed_wilcoxon_df, figure_payload):
    benchmark_id = _first_non_empty(raw_df, "benchmark_id") or results_dir.parent.name
    lines = [
        f"# {benchmark_id}",
        "",
        "## Dataset",
        f"- Problema: {_first_non_empty(raw_df, 'problem')}",
        f"- Instancias: {int(raw_df['instance_key'].nunique())}",
        f"- Bibliotecas: {int(raw_df['algorithm'].nunique())}",
        f"- Runs raw: {int(len(raw_df))}",
        f"- Runs OK: {int((raw_df['status'] == 'ok').sum())}",
        "",
        "## Descriptivos",
    ]
    for row in descriptive_df.to_dict(orient="records"):
        lines.append(
            f"- {row['algorithm']}: mean={_format_number(row['mean'])}, median={_format_number(row['median'])}, std={_format_number(row['std'])}, best={_format_number(row['best'])}"
        )

    lines.extend(["", "## Friedman"])
    friedman_row = friedman_df.iloc[0].to_dict() if not friedman_df.empty else None
    if friedman_row is None:
        lines.append("- No disponible.")
    elif friedman_row["status"] == "ok":
        lines.append(
            f"- statistic={_format_number(friedman_row['statistic'])}, p-value={_format_number(friedman_row['p_value'])}, instances={int(friedman_row['instance_count'])}"
        )
    else:
        lines.append(f"- skipped: {friedman_row['reason']}")

    lines.extend(["", "## Average ranks"])
    if ranks_df.empty:
        lines.append("- No disponibles.")
    else:
        for row in ranks_df.to_dict(orient="records"):
            lines.append(f"- {row['algorithm']}: avg_rank={_format_number(row['avg_rank'])}")

    lines.extend(["", "## Wilcoxon por seeds"])
    if seed_wilcoxon_df.empty:
        lines.append("- No disponible.")
    else:
        for row in seed_wilcoxon_df.to_dict(orient="records"):
            if row["status"] == "ok":
                lines.append(
                    f"- {row['metric']}: {row['algorithm_a']} vs {row['algorithm_b']} p-value={_format_number(row['p_value'])} seeds={int(row['paired_seeds'])}"
                )
            else:
                lines.append(
                    f"- {row['metric']}: {row['algorithm_a']} vs {row['algorithm_b']} skipped ({row['reason']})"
                )

    lines.extend(["", "## Figures"])
    for name in ["boxplot", "convergence", "critical_difference", "scatter"]:
        payload = figure_payload.get(name)
        if payload is None:
            lines.append(f"- {name}: no disponible")
        else:
            lines.append(f"- {name}: {payload['png'].name}")

    (results_dir / "report.md").write_text("\n".join(lines) + "\n", encoding="utf-8")


def _write_figure_snippet(path: Path, payload, fallback_note: str):
    if payload is None:
        _write_note(path, fallback_note)
        return

    relative_png = f"../plots/{payload['png'].name}"
    lines = [
        "\\begin{figure}[htbp]",
        "\\centering",
        f"\\includegraphics[width=0.9\\linewidth]{{{_latex_escape(relative_png)}}}",
        f"\\caption{{{_latex_escape(payload['caption'])}}}",
        f"\\label{{{payload['label']}}}",
        "\\end{figure}",
        "",
    ]
    path.write_text("\n".join(lines), encoding="utf-8")


def _plot_boxplot(raw_df, png_path: Path, svg_path: Path):
    algorithms = sorted(raw_df["algorithm"].unique().tolist())
    data = [
        raw_df[raw_df["algorithm"] == algorithm]["final_fitness"].dropna().to_numpy(dtype=float)
        for algorithm in algorithms
    ]
    fig, ax = plt.subplots(figsize=(10, 5))
    ax.boxplot(data, labels=algorithms, showmeans=True)
    ax.set_xlabel("biblioteca")
    ax.set_ylabel("final_fitness")
    ax.set_title("Boxplot de final_fitness por biblioteca")
    ax.tick_params(axis="x", rotation=20)
    fig.tight_layout()
    fig.savefig(png_path, dpi=160)
    fig.savefig(svg_path)
    plt.close(fig)


def _plot_scatter(algorithm_summary_df, png_path: Path, svg_path: Path):
    fig, ax = plt.subplots(figsize=(8, 5))
    for _, row in algorithm_summary_df.iterrows():
        if pd.isna(row["median_wall_time_ms"]) or pd.isna(row["median"]):
            continue
        ax.scatter(row["median_wall_time_ms"], row["median"], s=60)
        ax.annotate(row["algorithm"], (row["median_wall_time_ms"], row["median"]), textcoords="offset points", xytext=(4, 4))
    ax.set_xlabel("median wall_time_ms")
    ax.set_ylabel("median final_fitness")
    ax.set_title("Trade-off fitness vs tiempo")
    fig.tight_layout()
    fig.savefig(png_path, dpi=160)
    fig.savefig(svg_path)
    plt.close(fig)


def _plot_convergence(convergence_df, png_path: Path, svg_path: Path):
    fig, ax = plt.subplots(figsize=(10, 5))
    for algorithm, algorithm_df in convergence_df.groupby("algorithm"):
        series = algorithm_df.groupby("evaluation")["best_fitness_so_far"].median().sort_index()
        ax.plot(series.index.to_numpy(dtype=float), series.to_numpy(dtype=float), label=algorithm)
    ax.set_xlabel("evaluation")
    ax.set_ylabel("best_fitness_so_far")
    ax.set_title("Convergencia mediana por biblioteca")
    ax.legend(loc="best")
    fig.tight_layout()
    fig.savefig(png_path, dpi=160)
    fig.savefig(svg_path)
    plt.close(fig)


def _can_plot_critical_difference(ranks_df, friedman_df):
    if ranks_df.empty or friedman_df.empty or np is None:
        return False
    row = friedman_df.iloc[0]
    return int(row["instance_count"]) >= 2 and int(row["algorithm_count"]) >= 3


def _plot_critical_difference(ranks_df, friedman_df, png_path: Path, svg_path: Path):
    algorithms = ranks_df["algorithm"].tolist()
    ranks = ranks_df["avg_rank"].to_numpy(dtype=float)
    algorithm_count = int(friedman_df.iloc[0]["algorithm_count"])
    instance_count = int(friedman_df.iloc[0]["instance_count"])
    cd_value = _critical_difference(algorithm_count, instance_count)

    fig, ax = plt.subplots(figsize=(10, 3.5))
    min_rank = 1.0
    max_rank = max(float(algorithm_count), float(np.nanmax(ranks)) if len(ranks) else 1.0)
    ax.hlines(0.5, min_rank, max_rank, color="black")
    for tick in np.arange(min_rank, max_rank + 0.01, 0.5):
        ax.vlines(tick, 0.46, 0.54, color="black", linewidth=0.6)

    for index, (algorithm, rank) in enumerate(zip(algorithms, ranks), start=1):
        y_value = 0.68 + (index % 2) * 0.18
        ax.plot([rank, rank], [0.5, y_value], color="black", linewidth=1.0)
        ax.text(rank, y_value + 0.02, f"{algorithm} ({rank:.2f})", rotation=22, ha="center", va="bottom")

    if cd_value is not None:
        start = min_rank
        end = min(max_rank, start + cd_value)
        ax.plot([start, end], [0.22, 0.22], color="black", linewidth=2.0)
        ax.vlines([start, end], 0.18, 0.26, color="black", linewidth=1.5)
        ax.text((start + end) / 2.0, 0.1, f"CD = {cd_value:.2f}", ha="center")

    ax.set_xlabel("Average rank (1 = best)")
    ax.set_yticks([])
    ax.set_ylim(0.05, 1.2)
    ax.set_xlim(min_rank - 0.1, max_rank + 0.1)
    ax.set_title("Critical difference diagram")
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


def _vargha_delaney_a12(sample_a, sample_b, lower_is_better=True):
    values_a = [float(value) for value in sample_a] if sample_a is not None else []
    values_b = [float(value) for value in sample_b] if sample_b is not None else []
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


def _save_json(path: Path, payload):
    path.parent.mkdir(parents=True, exist_ok=True)
    path.write_text(json.dumps(payload, indent=2), encoding="utf-8")


def _write_note(path: Path, note: str):
    path.parent.mkdir(parents=True, exist_ok=True)
    path.write_text(note + "\n", encoding="utf-8")


def _first_non_empty(df, column_name):
    if column_name not in df.columns:
        return None
    series = df[column_name].dropna()
    if series.empty:
        return None
    return series.iloc[0]


def _format_cell(value):
    if value is None:
        return "--"
    if isinstance(value, float) and math.isnan(value):
        return "--"
    if isinstance(value, (int, float, np.integer, np.floating)):
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


def _best_value(values, objective_sense):
    if len(values) == 0:
        return math.nan
    return float(np.max(values) if objective_sense == "max" else np.min(values))