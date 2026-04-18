//! HTML report observer.
//!
//! This observer generates a self-contained `report.html` file at the end of
//! an algorithm run. The report includes:
//! - execution metadata,
//! - best solution snapshot,
//! - generation statistics,
//! - an embedded SVG convergence chart.

use crate::observer::traits::AlgorithmObserver;
use crate::observer::AlgorithmEvent;
use crate::utils::chart::{ChartBuilder, Series};
use std::fmt::Debug;
use std::fs::File;
use std::io::Write;
use std::path::PathBuf;
use std::time::{SystemTime, UNIX_EPOCH};

#[derive(Clone, Debug)]
struct GenerationMetrics {
    generation: usize,
    evaluations: usize,
    best: f64,
    average: f64,
    worst: f64,
}

#[derive(Clone, Debug)]
struct BestSnapshot {
    generation: usize,
    quality: f64,
    variables_preview: String,
}

#[derive(Clone, Debug)]
struct ReportModel {
    algorithm_name: String,
    status_text: String,
    status_class: &'static str,
    finished: bool,
    total_generations: usize,
    total_evaluations: usize,
    best_overall: f64,
    report_path_short: String,
    report_path_full: String,
    error_message: Option<String>,
    best_solution_row: String,
    recent_generations_rows: String,
    best_updates_rows: String,
}

/// Observer that generates an HTML execution report with metrics and charts.
///
/// # Output layout
///
/// By default, each run is written to:
/// `target/observers_outputs/reports/<algorithm_slug>/run_<timestamp_ms>_<pid>/report.html`.
///
/// Use [`HtmlReportObserver::with_flat_output`] if you prefer a flat output
/// folder without per-run subdirectories.
///
/// # Typical usage
///
/// Create and register the observer in your algorithm before calling `run()`.
/// At the end of execution, the observer prints a clickable terminal link to
/// the generated HTML file.
pub struct HtmlReportObserver {
    name: String,
    base_output_path: PathBuf,
    run_output_path: Option<PathBuf>,
    use_run_subdirectory: bool,
    algorithm_name: Option<String>,
    error_message: Option<String>,
    finished_summary: Option<(usize, usize)>,
    generations: Vec<GenerationMetrics>,
    best_snapshots: Vec<BestSnapshot>,
    last_snapshot_seq: Option<u64>,
}

impl HtmlReportObserver {
    /// Creates a report observer with a custom base output directory.
    ///
    /// The generated report path is:
    /// `<base>/<algorithm_slug>/run_<timestamp_ms>_<pid>/report.html`.
    pub fn new(base_output_path: PathBuf) -> Self {
        Self {
            name: "HtmlReportObserver".to_string(),
            base_output_path,
            run_output_path: None,
            use_run_subdirectory: true,
            algorithm_name: None,
            error_message: None,
            finished_summary: None,
            generations: Vec::new(),
            best_snapshots: Vec::new(),
            last_snapshot_seq: None,
        }
    }

    /// Creates a report observer with default base directory:
    /// `target/observers_outputs/reports`.
    pub fn new_default() -> Self {
        Self::new(crate::observer::default_observers_output_path().join("reports"))
    }

    /// Disables automatic per-run subdirectories.
    ///
    /// When enabled (default), each run gets its own timestamped folder.
    /// When disabled, the report is written directly to
    /// `<base_output_path>/report.html`.
    pub fn with_flat_output(mut self) -> Self {
        self.use_run_subdirectory = false;
        self
    }

    fn sanitize_folder_component(raw: &str) -> String {
        let mut out = String::with_capacity(raw.len());
        let mut prev_is_sep = false;

        for ch in raw.chars() {
            let normalized = if ch.is_ascii_alphanumeric() {
                ch.to_ascii_lowercase()
            } else {
                '_'
            };

            if normalized == '_' {
                if prev_is_sep {
                    continue;
                }
                prev_is_sep = true;
                out.push('_');
            } else {
                prev_is_sep = false;
                out.push(normalized);
            }
        }

        let trimmed = out.trim_matches('_');
        if trimmed.is_empty() {
            "algorithm".to_string()
        } else {
            trimmed.to_string()
        }
    }

    fn build_run_output_path(&self, algorithm_name: &str) -> PathBuf {
        if !self.use_run_subdirectory {
            return self.base_output_path.clone();
        }

        let algorithm_folder = Self::sanitize_folder_component(algorithm_name);
        let timestamp_ms = SystemTime::now()
            .duration_since(UNIX_EPOCH)
            .map(|d| d.as_millis())
            .unwrap_or(0);
        let pid = std::process::id();

        self.base_output_path
            .join(algorithm_folder)
            .join(format!("run_{}_{}", timestamp_ms, pid))
    }

    fn resolve_output_path(&self) -> PathBuf {
        self.run_output_path
            .clone()
            .unwrap_or_else(|| self.base_output_path.clone())
    }

    fn prepare_output_directory(&mut self, algorithm_name: &str) {
        let output_path = self.build_run_output_path(algorithm_name);
        std::fs::create_dir_all(&output_path).ok();
        self.run_output_path = Some(output_path);
    }

    fn escape_html(input: &str) -> String {
        input
            .replace('&', "&amp;")
            .replace('<', "&lt;")
            .replace('>', "&gt;")
            .replace('"', "&quot;")
            .replace('\'', "&#39;")
    }

    fn truncate_preview(input: String, max_chars: usize) -> String {
        let mut chars = input.chars();
        let preview: String = chars.by_ref().take(max_chars).collect();
        if chars.next().is_some() {
            format!("{}…", preview)
        } else {
            preview
        }
    }

    fn build_convergence_svg(&self) -> String {
        if self.generations.is_empty() {
            return "<p>No generation metrics captured.</p>".to_string();
        }

        let best_data: Vec<(f64, f64)> = self
            .generations
            .iter()
            .map(|g| (g.generation as f64, g.best))
            .collect();
        let avg_data: Vec<(f64, f64)> = self
            .generations
            .iter()
            .map(|g| (g.generation as f64, g.average))
            .collect();
        let worst_data: Vec<(f64, f64)> = self
            .generations
            .iter()
            .map(|g| (g.generation as f64, g.worst))
            .collect();

        let y_min = self
            .generations
            .iter()
            .flat_map(|g| [g.best, g.average, g.worst])
            .fold(f64::INFINITY, f64::min);

        ChartBuilder::new()
            .title("Fitness evolution")
            .x_label("Generation")
            .y_label("Fitness")
            .size(980, 520)
            .x_min(0.0)
            .y_min(y_min)
            .x_clamp_non_negative()
            .add_series(Series::new("Best", best_data).with_color("#2563eb"))
            .add_series(Series::new("Average", avg_data).with_color("#10b981"))
            .add_series(Series::new("Worst", worst_data).with_color("#dc2626"))
            .build()
            .generate_svg()
    }

    fn render_css() -> &'static str {
        r#"
    :root {
      --surface: #ffffff;
      --surface-subtle: #f8fafc;
      --background: #f3f4f6;
      --text: #111827;
      --muted: #6b7280;
      --border: #d1d5db;
      --border-soft: #e5e7eb;
      --accent: #1d4ed8;
      --accent-soft: #dbeafe;
      --success: #15803d;
      --error: #b91c1c;
      --radius: 10px;
      --space-1: 4px;
      --space-2: 8px;
      --space-3: 12px;
      --space-4: 16px;
      --space-6: 24px;
      --space-8: 32px;
    }

    * { box-sizing: border-box; }

    body {
      margin: 0;
      color: var(--text);
      background: var(--background);
      font-family: "Segoe UI", Tahoma, Geneva, Verdana, sans-serif;
      line-height: 1.45;
    }

    .page {
      width: min(1200px, 100% - 32px);
      margin: var(--space-6) auto;
    }

    .header {
      margin-bottom: var(--space-6);
      padding-bottom: var(--space-3);
      border-bottom: 2px solid var(--border);
    }

    .header-title {
      margin: 0;
      font-size: 28px;
      font-weight: 700;
      letter-spacing: 0.2px;
    }

    .header-meta {
      margin-top: var(--space-2);
      color: var(--muted);
      font-size: 14px;
    }

    .status-badge {
      display: inline-flex;
      align-items: center;
      margin-top: var(--space-3);
      padding: 2px 10px;
      border-radius: 999px;
      border: 1px solid var(--border);
      font-size: 13px;
      font-weight: 600;
      background: var(--surface);
    }

    .status-completed {
      color: var(--success);
      border-color: #86efac;
      background: #f0fdf4;
    }

    .status-error {
      color: var(--error);
      border-color: #fecaca;
      background: #fef2f2;
    }

    .kpi-grid {
      display: grid;
      grid-template-columns: repeat(auto-fit, minmax(180px, 1fr));
      gap: var(--space-3);
      margin-bottom: var(--space-6);
    }

    .card {
      background: var(--surface);
      border: 1px solid var(--border-soft);
      border-radius: var(--radius);
      padding: var(--space-3);
    }

    .card-label {
      display: block;
      margin-bottom: var(--space-1);
      color: var(--muted);
      font-size: 12px;
      letter-spacing: 0.4px;
      text-transform: uppercase;
      font-weight: 600;
    }

    .card-value {
      margin: 0;
      font-size: 20px;
      font-weight: 650;
      font-variant-numeric: tabular-nums;
      overflow-wrap: anywhere;
    }

    .section {
      margin-top: var(--space-6);
      background: var(--surface);
      border: 1px solid var(--border-soft);
      border-radius: var(--radius);
      padding: var(--space-4);
    }

    .section-title {
      margin: 0 0 var(--space-3) 0;
      font-size: 18px;
      font-weight: 650;
      color: #0f172a;
    }

    .chart-wrap {
      border: 1px solid var(--border-soft);
      border-radius: 8px;
      padding: var(--space-2);
      background: #fcfcfd;
      overflow-x: auto;
    }

    .chart-wrap svg {
      display: block;
      width: 100%;
      height: auto;
    }

    .table-wrap {
      width: 100%;
      overflow-x: auto;
      border: 1px solid var(--border-soft);
      border-radius: 8px;
    }

    table {
      width: 100%;
      min-width: 760px;
      border-collapse: collapse;
      background: var(--surface);
    }

    th, td {
      border-bottom: 1px solid var(--border-soft);
      padding: var(--space-2) var(--space-3);
      text-align: left;
      font-size: 13px;
      vertical-align: top;
    }

    th {
      position: sticky;
      top: 0;
      background: var(--surface-subtle);
      color: #111827;
      font-weight: 650;
    }

    tr:nth-child(even) td {
      background: #fbfcfe;
    }

    .numeric {
      text-align: right;
      font-variant-numeric: tabular-nums;
    }

    code {
      font-family: "SFMono-Regular", Consolas, "Liberation Mono", Menlo, monospace;
      white-space: pre-wrap;
      word-break: break-word;
      font-size: 12px;
      color: #1f2937;
    }

    .error-box {
      margin-top: var(--space-3);
      border: 1px solid #fecaca;
      border-radius: 8px;
      background: #fef2f2;
      color: #991b1b;
      padding: var(--space-2) var(--space-3);
      font-size: 13px;
    }

    .path-note {
      margin-top: var(--space-6);
      padding: var(--space-2) var(--space-3);
      border-top: 1px dashed var(--border);
      color: var(--muted);
      font-size: 12px;
      line-height: 1.4;
    }

    .path-note code {
      font-size: 11px;
      color: #475569;
    }

    @media (max-width: 760px) {
      .page {
        width: calc(100% - 16px);
        margin: var(--space-4) auto;
      }

      .section {
        padding: var(--space-3);
      }

      .header-title {
        font-size: 24px;
      }

      .card-value {
        font-size: 18px;
      }
    }
"#
    }

    fn summary_totals(&self) -> (usize, usize) {
        self.finished_summary.unwrap_or_else(|| {
            let last = self.generations.last();
            (
                last.map(|g| g.generation).unwrap_or(0),
                last.map(|g| g.evaluations).unwrap_or(0),
            )
        })
    }

    fn compact_report_path(path: &str) -> String {
        if path.len() <= 64 {
            return path.to_string();
        }

        let keep = 26;
        let start = &path[..keep.min(path.len())];
        let end_index = path.len().saturating_sub(keep);
        let end = &path[end_index..];
        format!("{}...{}", start, end)
    }

    fn build_report_model(&self, report_path: &std::path::Path) -> ReportModel {
        let algorithm_name = self
            .algorithm_name
            .as_deref()
            .map(Self::escape_html)
            .unwrap_or_else(|| "Unknown".to_string());

        let finished = self.finished_summary.is_some();
        let (status_text, status_class, error_message) = match (&self.error_message, finished) {
            (Some(message), _) => (
                "Failed".to_string(),
                "status-error",
                Some(Self::escape_html(message)),
            ),
            (None, false) => (
                "Interrupted".to_string(),
                "status-error",
                Some(
                    "Run did not reach End event; report generated from partial snapshots."
                        .to_string(),
                ),
            ),
            (None, true) => ("Completed".to_string(), "status-completed", None),
        };

        let (total_generations, total_evaluations) = self.summary_totals();

        let best_overall = self
            .generations
            .iter()
            .map(|g| g.best)
            .reduce(f64::max)
            .unwrap_or(0.0);

        let best_solution_row = self
            .best_snapshots
            .last()
            .map(|snapshot| {
                format!(
                    "<tr><td class=\"numeric\">{}</td><td class=\"numeric\">{:.6}</td><td><code>{}</code></td></tr>",
                    snapshot.generation,
                    snapshot.quality,
                    Self::escape_html(&snapshot.variables_preview)
                )
            })
            .unwrap_or_else(|| {
                "<tr><td colspan=\"3\">No solution snapshot captured.</td></tr>".to_string()
            });

        let recent_generations_rows = if self.generations.is_empty() {
            "<tr><td colspan=\"5\">No generation metrics captured.</td></tr>".to_string()
        } else {
            self.generations
                .iter()
                .rev()
                .take(15)
                .map(|g| {
                    format!(
                        "<tr><td class=\"numeric\">{}</td><td class=\"numeric\">{}</td><td class=\"numeric\">{:.6}</td><td class=\"numeric\">{:.6}</td><td class=\"numeric\">{:.6}</td></tr>",
                        g.generation, g.evaluations, g.best, g.average, g.worst
                    )
                })
                .collect::<String>()
        };

        let best_updates_rows = if self.best_snapshots.is_empty() {
            "<tr><td colspan=\"3\">No best-solution snapshots captured.</td></tr>".to_string()
        } else {
            self.best_snapshots
                .iter()
                .rev()
                .take(20)
                .map(|snapshot| {
                    format!(
                        "<tr><td class=\"numeric\">{}</td><td class=\"numeric\">{:.6}</td><td><code>{}</code></td></tr>",
                        snapshot.generation,
                        snapshot.quality,
                        Self::escape_html(&snapshot.variables_preview)
                    )
                })
                .collect::<String>()
        };

        ReportModel {
            algorithm_name,
            status_text,
            status_class,
            finished,
            total_generations,
            total_evaluations,
            best_overall,
            report_path_short: Self::compact_report_path(&report_path.display().to_string()),
            report_path_full: Self::escape_html(&report_path.display().to_string()),
            error_message,
            best_solution_row,
            recent_generations_rows,
            best_updates_rows,
        }
    }

    fn render_summary_cards(model: &ReportModel) -> String {
        format!(
            "<div class=\"kpi-grid\">
    <div class=\"card\"><span class=\"card-label\">Algorithm</span><p class=\"card-value\">{algorithm_name}</p></div>
    <div class=\"card\"><span class=\"card-label\">Status</span><p class=\"card-value\">{status_text}</p></div>
    <div class=\"card\"><span class=\"card-label\">Run completeness</span><p class=\"card-value\">{finished_label}</p></div>
    <div class=\"card\"><span class=\"card-label\">Total generations</span><p class=\"card-value numeric\">{total_generations}</p></div>
    <div class=\"card\"><span class=\"card-label\">Total evaluations</span><p class=\"card-value numeric\">{total_evaluations}</p></div>
    <div class=\"card\"><span class=\"card-label\">Best fitness observed</span><p class=\"card-value numeric\">{best_overall:.6}</p></div>
  </div>",
            algorithm_name = model.algorithm_name,
            status_text = model.status_text,
            finished_label = if model.finished { "End event received" } else { "Partial (no End event)" },
            total_generations = model.total_generations,
            total_evaluations = model.total_evaluations,
            best_overall = model.best_overall,
        )
    }

    fn render_html_document(&self, model: &ReportModel, chart_svg: &str) -> String {
        let error_box = model
            .error_message
            .as_ref()
            .map(|message| {
                format!(
                    "<div class=\"error-box\"><strong>Execution error:</strong> {}</div>",
                    message
                )
            })
            .unwrap_or_default();

        let summary_cards = Self::render_summary_cards(model);
        let css = Self::render_css();

        format!(
            r#"<!doctype html>
<html lang="en">
<head>
  <meta charset="utf-8" />
  <meta name="viewport" content="width=device-width, initial-scale=1" />
  <title>Algorithm report</title>
  <style>{css}</style>
</head>
<body>
  <div class="page">
    <header class="header">
      <h1 class="header-title">Algorithm execution report</h1>
      <div class="header-meta">Generated by Roma HtmlReportObserver</div>
      <span class="status-badge {status_class}">{status_text}</span>
      {error_box}
    </header>

    {summary_cards}

    <main>
      <section class="section">
        <h2 class="section-title">Best solution found</h2>
        <div class="table-wrap">
          <table>
            <thead><tr><th>Generation</th><th>Quality</th><th>Variables preview</th></tr></thead>
            <tbody>{best_solution_row}</tbody>
          </table>
        </div>
      </section>

      <section class="section">
        <h2 class="section-title">Fitness chart</h2>
        <div class="chart-wrap">{chart_svg}</div>
      </section>

      <section class="section">
        <h2 class="section-title">Recent generation metrics (latest 15)</h2>
        <div class="table-wrap">
          <table>
            <thead><tr><th>Generation</th><th>Evaluations</th><th>Best</th><th>Average</th><th>Worst</th></tr></thead>
            <tbody>{recent_generations_rows}</tbody>
          </table>
        </div>
      </section>

      <section class="section">
        <h2 class="section-title">Best-solution snapshots (latest 20)</h2>
        <div class="table-wrap">
          <table>
            <thead><tr><th>Generation</th><th>Quality</th><th>Variables preview</th></tr></thead>
            <tbody>{best_updates_rows}</tbody>
          </table>
        </div>
      </section>

      <div class="path-note">
        <strong>Report file</strong>: <code>{report_path_short}</code><br />
        <code>{report_path_full}</code>
      </div>
    </main>
  </div>
</body>
</html>
"#,
            css = css,
            status_class = model.status_class,
            status_text = model.status_text,
            error_box = error_box,
            summary_cards = summary_cards,
            best_solution_row = model.best_solution_row,
            chart_svg = chart_svg,
            recent_generations_rows = model.recent_generations_rows,
            best_updates_rows = model.best_updates_rows,
            report_path_short = model.report_path_short,
            report_path_full = model.report_path_full,
        )
    }

    fn generate_report(&self) -> Result<(), Box<dyn std::error::Error>> {
        let output_path = self.resolve_output_path();
        std::fs::create_dir_all(&output_path)?;
        let report_path = output_path.join("report.html");
        let model = self.build_report_model(&report_path);
        let chart_svg = self.build_convergence_svg();
        let html = self.render_html_document(&model, &chart_svg);

        let mut file = File::create(report_path)?;
        file.write_all(html.as_bytes())?;

        Ok(())
    }

    fn report_file_path(&self) -> PathBuf {
        self.resolve_output_path().join("report.html")
    }

    fn report_file_url(&self) -> String {
        let report_path = self.report_file_path();
        let absolute = std::fs::canonicalize(&report_path).unwrap_or(report_path);
        format!("file://{}", absolute.display())
    }

    fn terminal_hyperlink(url: &str, label: &str) -> String {
        format!("\x1b]8;;{}\x1b\\{}\x1b]8;;\x1b\\", url, label)
    }
}

impl<T, Q> AlgorithmObserver<T, Q> for HtmlReportObserver
where
    T: Clone + Send + Debug + 'static,
    Q: Clone + Send + 'static,
{
    fn update(&mut self, event: &AlgorithmEvent<T, Q>) {
        match event {
            AlgorithmEvent::Start { algorithm_name } => {
                self.algorithm_name = Some(algorithm_name.clone());
                self.prepare_output_directory(algorithm_name);
                self.error_message = None;
                self.finished_summary = None;
                self.generations.clear();
                self.best_snapshots.clear();
                self.last_snapshot_seq = None;
            }
            AlgorithmEvent::ExecutionStateUpdated { state } => {
                if let Some(last_seq) = self.last_snapshot_seq {
                    if state.seq_id <= last_seq {
                        return;
                    }
                }

                self.last_snapshot_seq = Some(state.seq_id);
                self.generations.push(GenerationMetrics {
                    generation: state.iteration,
                    evaluations: state.evaluations,
                    best: state.best_fitness,
                    average: state.average_fitness,
                    worst: state.worst_fitness,
                });
                let preview = Self::truncate_preview(state.best_solution_presentation.clone(), 220);
                self.best_snapshots.push(BestSnapshot {
                    generation: state.iteration,
                    quality: state.best_fitness,
                    variables_preview: preview,
                });
            }
            AlgorithmEvent::End {
                total_generations,
                total_evaluations,
                ..
            } => {
                self.finished_summary = Some((*total_generations, *total_evaluations));
                if let Err(error) = self.generate_report() {
                    eprintln!("HtmlReportObserver: failed to generate report: {}", error);
                } else {
                    let report_url = self.report_file_url();
                    println!(
                        "  Open report: {}",
                        Self::terminal_hyperlink(&report_url, "Open HTML report")
                    );
                }
            }
            AlgorithmEvent::Failed {
                total_generations,
                total_evaluations,
                error_message,
                ..
            } => {
                self.error_message = Some(error_message.clone());
                self.finished_summary = Some((*total_generations, *total_evaluations));
                if let Err(error) = self.generate_report() {
                    eprintln!(
                        "HtmlReportObserver: failed to generate report after failure: {}",
                        error
                    );
                } else {
                    let report_url = self.report_file_url();
                    println!(
                        "  Open report: {}",
                        Self::terminal_hyperlink(&report_url, "Open HTML report")
                    );
                }
            }
            _ => {}
        }
    }

    fn finalize(&mut self) {
        if let Err(error) = self.generate_report() {
            eprintln!(
                "HtmlReportObserver: failed to generate report in finalize: {}",
                error
            );
        }
    }

    fn name(&self) -> &str {
        &self.name
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::observer::traits::AlgorithmObserver;
    use crate::observer::ObserverState;

    #[test]
    fn creates_html_report_with_summary_and_chart() {
        let base = std::env::temp_dir().join(format!(
            "roma_html_report_observer_test_{}",
            SystemTime::now()
                .duration_since(UNIX_EPOCH)
                .map(|d| d.as_nanos())
                .unwrap_or(0)
        ));

        let mut observer = HtmlReportObserver::new(base);
        observer.update(&AlgorithmEvent::<bool>::Start {
            algorithm_name: "GeneticAlgorithm".to_string(),
        });
        observer.update(&AlgorithmEvent::<bool>::ExecutionStateUpdated {
            state: ObserverState::new(0, 1, 20, 10.0, 7.0, 3.0, "selected=2/3".to_string()),
        });

        observer.update(&AlgorithmEvent::<bool>::End {
            total_generations: 1,
            total_evaluations: 20,
            termination_reason: None,
        });

        let report_path = observer.resolve_output_path().join("report.html");
        assert!(report_path.exists());

        let contents = std::fs::read_to_string(report_path).expect("report should be readable");
        assert!(contents.contains("Algorithm execution report"));
        assert!(contents.contains("GeneticAlgorithm"));
        assert!(contents.contains("Generated by Roma HtmlReportObserver"));
        assert!(contents.contains("Best solution found"));
        assert!(contents.contains("<svg"));
    }

    #[test]
    fn report_uses_professional_css_and_semantic_sections() {
        let base = std::env::temp_dir().join(format!(
            "roma_html_report_style_test_{}",
            SystemTime::now()
                .duration_since(UNIX_EPOCH)
                .map(|d| d.as_nanos())
                .unwrap_or(0)
        ));

        let mut observer = HtmlReportObserver::new(base);
        observer.update(&AlgorithmEvent::<bool>::Start {
            algorithm_name: "StyleCheckAlgorithm".to_string(),
        });
        observer.update(&AlgorithmEvent::<bool>::ExecutionStateUpdated {
            state: ObserverState::new(0, 1, 5, 3.0, 2.0, 1.0, "presentation=1".to_string()),
        });
        observer.update(&AlgorithmEvent::<bool>::End {
            total_generations: 1,
            total_evaluations: 5,
            termination_reason: None,
        });

        let report_path = observer.resolve_output_path().join("report.html");
        let contents = std::fs::read_to_string(report_path).expect("report should be readable");

        assert!(contents.contains("--surface:"));
        assert!(contents.contains("<header class=\"header\">"));
        assert!(contents.contains("class=\"kpi-grid\""));
        assert!(contents.contains("class=\"table-wrap\""));
    }

    #[test]
    fn report_shows_failed_status_when_failure_event_is_received() {
        let base = std::env::temp_dir().join(format!(
            "roma_html_report_failure_test_{}",
            SystemTime::now()
                .duration_since(UNIX_EPOCH)
                .map(|d| d.as_nanos())
                .unwrap_or(0)
        ));

        let mut observer = HtmlReportObserver::new(base);
        observer.update(&AlgorithmEvent::<bool>::Start {
            algorithm_name: "FailureAlgorithm".to_string(),
        });
        observer.update(&AlgorithmEvent::<bool>::ExecutionStateUpdated {
            state: ObserverState::new(0, 2, 10, 1.5, 1.0, 0.5, "selected=2/4".to_string()),
        });
        observer.update(&AlgorithmEvent::<bool>::Failed {
            total_generations: 2,
            total_evaluations: 10,
            termination_reason: None,
            error_message: "synthetic failure".to_string(),
        });

        let report_path = observer.resolve_output_path().join("report.html");
        let contents = std::fs::read_to_string(report_path).expect("report should be readable");

        assert!(contents.contains("status-badge status-error\">Failed<"));
        assert!(contents.contains("synthetic failure"));
    }

    #[test]
    fn report_shows_interrupted_status_when_finalize_runs_without_end() {
        let base = std::env::temp_dir().join(format!(
            "roma_html_report_interrupted_test_{}",
            SystemTime::now()
                .duration_since(UNIX_EPOCH)
                .map(|d| d.as_nanos())
                .unwrap_or(0)
        ));

        let mut observer = HtmlReportObserver::new(base);
        observer.update(&AlgorithmEvent::<bool>::Start {
            algorithm_name: "InterruptedAlgorithm".to_string(),
        });
        observer.update(&AlgorithmEvent::<bool>::ExecutionStateUpdated {
            state: ObserverState::new(0, 1, 5, 1.0, 0.8, 0.6, "selected=1/4".to_string()),
        });

        <HtmlReportObserver as AlgorithmObserver<bool, f64>>::finalize(&mut observer);

        let report_path = observer.resolve_output_path().join("report.html");
        let contents = std::fs::read_to_string(report_path).expect("report should be readable");

        assert!(contents.contains("Interrupted"));
        assert!(contents.contains("Partial (no End event)"));
    }
}
