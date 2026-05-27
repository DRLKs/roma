from __future__ import annotations

import math
import shutil
from itertools import combinations
from pathlib import Path

from common import prepare_results_directory, read_rows_csv

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


ANALYSIS_SUBDIRS = ("raw", "aggregated", "analysis")
STATUS_COLUMNS = [
    "benchmark_id",
    "problem",
    "instance_count",
    "algorithm_count",
    "run_count",
    "ok_run_count",
    "convergence_row_count",
    "friedman_status",
    "friedman_reason",
]


def generate_benchmark_analysis(benchmark_root: Path, _summary: dict | None = None):
    results_dir = benchmark_root / "results"
    runs_path = results_dir / "runs.csv"
    if not runs_path.exists():
        return

    if pd is None or np is None:
        _prepare_analysis_layout(results_dir)
        _write_csv(
            results_dir / "analysis" / "status.csv",
            STATUS_COLUMNS,
            [
                {
                    "benchmark_id": benchmark_root.name,
                    "friedman_status": "skipped",
                    "friedman_reason": "pandas y numpy son obligatorios para generar los CSVs de analisis.",
                }
            ],
        )
        return

    raw_rows = read_rows_csv(runs_path)
    _prepare_analysis_layout(results_dir)
    if not raw_rows:
        _write_csv(
            results_dir / "analysis" / "status.csv",
            STATUS_COLUMNS,
            [{"benchmark_id": benchmark_root.name, "friedman_status": "skipped", "friedman_reason": "runs.csv esta vacio."}],
        )
        return

    raw_df = _normalize_raw_dataframe(pd.DataFrame(raw_rows))
    convergence_df = _build_convergence_dataframe(raw_df)
    algorithm_summary_df = _build_algorithm_summary(raw_df)
    instance_summary_df = _build_instance_summary(raw_df)

    descriptive_df = algorithm_summary_df[["algorithm", "mean", "median", "std", "best"]].copy()
    ranks_df, friedman_df, holm_df, nemenyi_df, instance_effect_df = _analyze_instances(instance_summary_df)
    seed_wilcoxon_df, seed_effect_df = _analyze_paired_seeds(raw_df)
    effect_sizes_df = _concat_frames(seed_effect_df, instance_effect_df)

    _write_csv_frame(results_dir / "raw" / "runs.csv", raw_df)
    _write_csv_frame(results_dir / "raw" / "convergence.csv", convergence_df)
    _write_csv_frame(results_dir / "aggregated" / "algorithm_summary.csv", algorithm_summary_df)
    _write_csv_frame(results_dir / "aggregated" / "instance_algorithm_summary.csv", instance_summary_df)
    _write_csv_frame(results_dir / "analysis" / "descriptive_summary.csv", descriptive_df)
    _write_csv_frame(results_dir / "analysis" / "friedman.csv", friedman_df)
    _write_csv_frame(results_dir / "analysis" / "average_ranks.csv", ranks_df)
    _write_csv_frame(results_dir / "analysis" / "posthoc_holm.csv", holm_df)
    _write_csv_frame(results_dir / "analysis" / "posthoc_nemenyi.csv", nemenyi_df)
    _write_csv_frame(results_dir / "analysis" / "pairwise_wilcoxon.csv", seed_wilcoxon_df)
    _write_csv_frame(results_dir / "analysis" / "effect_sizes.csv", effect_sizes_df)
    _write_status_csv(results_dir, raw_df, convergence_df, friedman_df)


def _prepare_analysis_layout(results_dir: Path):
    prepare_results_directory(results_dir)
    _quarantine_stale_outputs(results_dir)
    _remove_legacy_output(results_dir / "latex")
    _remove_legacy_output(results_dir / "plots")
    _remove_legacy_output(results_dir / "report.md")
    _remove_legacy_output(results_dir / "summary.json")
    for subdir in ANALYSIS_SUBDIRS:
        (results_dir / subdir).mkdir(parents=True, exist_ok=True)
    _remove_non_csv_children(results_dir / "raw")
    _remove_non_csv_children(results_dir / "aggregated")
    _remove_non_csv_children(results_dir / "analysis")


def _remove_legacy_output(path: Path):
    if not path.exists():
        return
    if path.is_dir():
        shutil.rmtree(path)
    else:
        path.unlink()


def _remove_non_csv_children(directory: Path):
    if not directory.exists():
        return
    for child in list(directory.iterdir()):
        if child.is_file() and child.suffix.lower() == ".csv":
            continue
        _remove_legacy_output(child)


def _quarantine_stale_outputs(results_dir: Path):
    legacy_dir = results_dir.parent / "_legacy_results"
    for child in list(results_dir.iterdir()):
        if ".stale_root_" not in child.name:
            continue
        legacy_dir.mkdir(parents=True, exist_ok=True)
        target = legacy_dir / child.name
        suffix = 1
        while target.exists():
            target = legacy_dir / f"{child.name}.{suffix}"
            suffix += 1
        try:
            child.rename(target)
        except PermissionError:
            continue


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


def _build_convergence_dataframe(raw_df):
    rows = []
    for _, row in raw_df.iterrows():
        history = row.get("convergence_history")
        if not isinstance(history, list):
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


def _build_algorithm_summary(raw_df):
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


def _build_instance_summary(raw_df):
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


def _analyze_instances(instance_summary_df):
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


def _analyze_paired_seeds(raw_df):
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


def _write_status_csv(results_dir: Path, raw_df, convergence_df, friedman_df):
    friedman_row = friedman_df.iloc[0].to_dict() if not friedman_df.empty else {}
    _write_csv(
        results_dir / "analysis" / "status.csv",
        STATUS_COLUMNS,
        [
            {
                "benchmark_id": _first_non_empty(raw_df, "benchmark_id"),
                "problem": _first_non_empty(raw_df, "problem"),
                "instance_count": int(raw_df["instance_key"].nunique()),
                "algorithm_count": int(raw_df["algorithm"].nunique()),
                "run_count": int(len(raw_df)),
                "ok_run_count": int((raw_df["status"] == "ok").sum()),
                "convergence_row_count": int(len(convergence_df)),
                "friedman_status": friedman_row.get("status"),
                "friedman_reason": friedman_row.get("reason"),
            }
        ],
    )


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


def _first_non_empty(df, column_name):
    if column_name not in df.columns:
        return None
    series = df[column_name].dropna()
    if series.empty:
        return None
    return series.iloc[0]


def _concat_frames(*frames):
    non_empty = [frame for frame in frames if frame is not None and not frame.empty]
    if not non_empty:
        return pd.DataFrame()
    return pd.concat(non_empty, ignore_index=True, sort=False)


def _write_csv_frame(path: Path, frame):
    if frame is None or frame.empty:
        columns = list(frame.columns) if frame is not None else []
        pd.DataFrame(columns=columns).to_csv(path, index=False)
        return
    frame.to_csv(path, index=False)


def _write_csv(path: Path, fieldnames, rows):
    pd.DataFrame(rows, columns=fieldnames).to_csv(path, index=False)


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