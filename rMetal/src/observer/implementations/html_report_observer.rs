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
use crate::solution::traits::QualityValue;
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

/// Observer that generates an HTML execution report with metrics and charts.
///
/// # Output layout
///
/// By default, each run is written to:
/// `output/reports/<algorithm_slug>/run_<timestamp_ms>_<pid>/report.html`.
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

    /// Creates a report observer with default base directory: `output/reports`.
    pub fn new_default() -> Self {
        Self::new(PathBuf::from("output/reports"))
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

    fn generate_report(&self) -> Result<(), Box<dyn std::error::Error>> {
        let output_path = self.resolve_output_path();
        std::fs::create_dir_all(&output_path)?;
        let report_path = output_path.join("report.html");

        let algorithm_name = self
            .algorithm_name
            .as_deref()
            .map(Self::escape_html)
            .unwrap_or_else(|| "Unknown".to_string());

        let status = if let Some(message) = &self.error_message {
            format!("Error: {}", Self::escape_html(message))
        } else {
            "Completed".to_string()
        };

        let (total_generations, total_evaluations) = self.finished_summary.unwrap_or_else(|| {
            let last = self.generations.last();
            (
                last.map(|g| g.generation).unwrap_or(0),
                last.map(|g| g.evaluations).unwrap_or(0),
            )
        });

        let best_overall = self
            .generations
            .iter()
            .map(|g| g.best)
            .reduce(f64::max)
            .unwrap_or(0.0);

        let best_solution_row = self.best_snapshots.last().map(|b| {
            format!(
                "<tr><td>{}</td><td>{:.6}</td><td><code>{}</code></td></tr>",
                b.generation,
                b.quality,
                Self::escape_html(&b.variables_preview)
            )
        }).unwrap_or_else(|| "<tr><td colspan=\"3\">No solution snapshot captured.</td></tr>".to_string());

        let recent_generations: String = self
            .generations
            .iter()
            .rev()
            .take(15)
            .map(|g| {
                format!(
                    "<tr><td>{}</td><td>{}</td><td>{:.6}</td><td>{:.6}</td><td>{:.6}</td></tr>",
                    g.generation, g.evaluations, g.best, g.average, g.worst
                )
            })
            .collect();

        let best_updates_rows: String = self
            .best_snapshots
            .iter()
            .rev()
            .take(20)
            .map(|b| {
                format!(
                    "<tr><td>{}</td><td>{:.6}</td><td><code>{}</code></td></tr>",
                    b.generation,
                    b.quality,
                    Self::escape_html(&b.variables_preview)
                )
            })
            .collect();

        let chart_svg = self.build_convergence_svg();

        let html = format!(
            r#"<!doctype html>
<html lang="en">
<head>
  <meta charset="utf-8" />
  <meta name="viewport" content="width=device-width, initial-scale=1" />
  <title>Algorithm report</title>
  <style>
    body {{ font-family: Inter, system-ui, -apple-system, Segoe UI, Roboto, sans-serif; margin: 24px; color: #111827; }}
    h1, h2 {{ margin: 0 0 12px 0; }}
    .meta {{ display: grid; grid-template-columns: repeat(auto-fit,minmax(220px,1fr)); gap: 12px; margin-bottom: 20px; }}
    .card {{ border: 1px solid #e5e7eb; border-radius: 10px; padding: 12px; background: #fff; }}
    table {{ border-collapse: collapse; width: 100%; margin-top: 8px; }}
    th, td {{ border: 1px solid #e5e7eb; padding: 8px; font-size: 14px; text-align: left; }}
    th {{ background: #f9fafb; }}
    code {{ white-space: pre-wrap; word-break: break-word; }}
    .section {{ margin-top: 24px; }}
  </style>
</head>
<body>
  <h1>Algorithm execution report</h1>
  <div class="meta">
    <div class="card"><strong>Algorithm</strong><br />{algorithm_name}</div>
    <div class="card"><strong>Status</strong><br />{status}</div>
    <div class="card"><strong>Total generations</strong><br />{total_generations}</div>
    <div class="card"><strong>Total evaluations</strong><br />{total_evaluations}</div>
    <div class="card"><strong>Best fitness observed</strong><br />{best_overall:.6}</div>
    <div class="card"><strong>Report path</strong><br />{report_path}</div>
  </div>

  <div class="section">
    <h2>Best solution found</h2>
    <table>
      <thead><tr><th>Generation</th><th>Quality</th><th>Variables preview</th></tr></thead>
      <tbody>{best_solution_row}</tbody>
    </table>
  </div>

  <div class="section">
    <h2>Fitness chart</h2>
    {chart_svg}
  </div>

  <div class="section">
    <h2>Recent generation metrics (latest 15)</h2>
    <table>
      <thead><tr><th>Generation</th><th>Evaluations</th><th>Best</th><th>Average</th><th>Worst</th></tr></thead>
      <tbody>{recent_generations}</tbody>
    </table>
  </div>

  <div class="section">
        <h2>Best-solution snapshots (latest 20)</h2>
    <table>
      <thead><tr><th>Generation</th><th>Quality</th><th>Variables preview</th></tr></thead>
      <tbody>{best_updates_rows}</tbody>
    </table>
  </div>
</body>
</html>
"#,
            report_path = Self::escape_html(&report_path.display().to_string()),
        );

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
    Q: Clone + QualityValue + Send + 'static,
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
                    best: state.best_solution.quality_value(),
                    average: state.average_fitness,
                    worst: state.worst_fitness,
                });
                let preview = Self::truncate_preview(
                    format!("{:?}", state.best_solution.variables()),
                    220,
                );
                self.best_snapshots.push(BestSnapshot {
                    generation: state.iteration,
                    quality: state.best_solution.quality_value(),
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
            AlgorithmEvent::Error { message } => {
                self.error_message = Some(message.clone());
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
            _ => {}
        }
    }

    fn finalize(&mut self) {
        if let Err(error) = self.generate_report() {
            eprintln!("HtmlReportObserver: failed to generate report in finalize: {}", error);
        }
    }

    fn name(&self) -> &str {
        &self.name
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::solution::Solution;

    #[test]
    fn creates_html_report_with_summary_and_chart() {
        let base = std::env::temp_dir().join(format!(
            "rmetal_html_report_observer_test_{}",
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
            state: crate::algorithms::termination::ExecutionStateSnapshot::new(
                0,
                1,
                20,
                {
                    let mut solution = Solution::<bool>::new(vec![true, false, true]);
                    solution.set_quality(10.0);
                    solution
                },
                7.0,
                3.0,
            ),
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
        assert!(contents.contains("Best solution found"));
        assert!(contents.contains("<svg"));
    }
}
