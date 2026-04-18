# Observer Modernization Implementation Plan

> **For agentic workers:** REQUIRED SUB-SKILL: Use superpowers:subagent-driven-development (recommended) or superpowers:executing-plans to implement this plan task-by-task. Steps use checkbox (`- [ ]`) syntax for tracking.

**Goal:** Modernize `ChartObserver` and `HtmlReportObserver` with a sober professional report UI, internal refactoring, and non-breaking output improvements while preserving zero external dependencies.

**Architecture:** Keep the public observer API and run-output contract unchanged, then incrementally improve behavior through strict TDD. `HtmlReportObserver` is refactored into private rendering/data-preparation helpers, and `ChartObserver` gains an additional `metrics.json` artifact derived from the same consolidated series used for charts.

**Tech Stack:** Rust (`std`), existing rMetal observer/event pipeline, existing SVG chart utilities (`crate::utils::chart`), Cargo tests.

---

### Task 1: Add failing tests for sober professional HTML report styling and structure

**Files:**
- Create: `rMetal/tests/html_report_observer_styling_test.rs`
- Modify: `rMetal/src/observer/implementations/html_report_observer.rs` (if test helper exposure is needed)
- Test: `rMetal/tests/html_report_observer_styling_test.rs`

- [ ] **Step 1: Write the failing test**

```rust
use rmetal::algorithms::{
    Algorithm, HillClimbing, HillClimbingParameters, ImprovementDirection, TerminationCriteria,
    TerminationCriterion,
};
use rmetal::observer::{HtmlReportObserver, Observable};
use rmetal::operator::BitFlipMutation;
use rmetal::problem::Problem;
use rmetal::solution::Solution;
use rmetal::utils::Random;
use std::time::{SystemTime, UNIX_EPOCH};

struct StylingProblem {
    description: String,
    variables: usize,
}

impl Problem<bool> for StylingProblem {
    fn new() -> Self {
        Self {
            description: "Styling problem".to_string(),
            variables: 10,
        }
    }

    fn evaluate(&self, solution: &mut Solution<bool>) {
        let selected = solution.variables().iter().filter(|&&v| v).count() as f64;
        solution.set_quality(selected);
    }

    fn create_solution(&self, rng: &mut Random) -> Solution<bool> {
        let variables = (0..self.variables).map(|_| rng.coin_flip()).collect();
        Solution::new(variables)
    }

    fn set_problem_description(&mut self, description: String) {
        self.description = description;
    }

    fn get_problem_description(&self) -> String {
        self.description.clone()
    }

    fn get_improvement_direction(&self) -> ImprovementDirection {
        ImprovementDirection::Maximize
    }
}

#[test]
fn html_report_uses_professional_css_and_semantic_sections() {
    let problem = StylingProblem::new();
    let run_id = SystemTime::now()
        .duration_since(UNIX_EPOCH)
        .map(|duration| duration.as_nanos())
        .unwrap_or(0);
    let output_dir = std::env::temp_dir().join(format!("rmetal_html_style_test_{}", run_id));

    let observer = HtmlReportObserver::new(output_dir.clone()).with_flat_output();
    let parameters = HillClimbingParameters::new(
        BitFlipMutation::new(),
        0.3,
        TerminationCriteria::new(vec![TerminationCriterion::MaxIterations(4)]),
    )
    .with_seed(99);
    let mut algorithm = HillClimbing::new(parameters);
    algorithm.add_observer(Box::new(observer));

    let _ = algorithm.run(&problem).expect("run should succeed");

    let report_path = output_dir.join("report.html");
    let report = std::fs::read_to_string(&report_path).expect("report should be readable");

    assert!(report.contains("--surface:"), "report should define CSS tokens");
    assert!(report.contains("<header class=\"header\">"), "report should use semantic header");
    assert!(report.contains("class=\"kpi-grid\""), "report should include KPI grid");
    assert!(report.contains("class=\"table-wrap\""), "report should include responsive table wrappers");
}
```

- [ ] **Step 2: Run test to verify it fails**

Run: `cargo test --test html_report_observer_styling_test -- --nocapture`  
Expected: FAIL because current HTML does not include the new CSS token/semantic class markers.

- [ ] **Step 3: Write minimal implementation to satisfy styling structure**

```rust
// In HtmlReportObserver private renderer area, add first minimal CSS token and semantic shell:
fn render_css() -> &'static str {
    r#"
    :root {
      --surface: #ffffff;
      --surface-subtle: #f8fafc;
      --text: #111827;
      --muted: #6b7280;
      --border: #e5e7eb;
      --accent: #1d4ed8;
      --radius: 10px;
      --space-2: 8px;
      --space-3: 12px;
      --space-4: 16px;
      --space-6: 24px;
    }
    "#
}

// Ensure the document includes:
// <header class="header"> ... </header>
// <section class="section"> ... </section>
// <div class="kpi-grid"> ... </div>
// <div class="table-wrap"> <table ...> ... </table> </div>
```

- [ ] **Step 4: Run test to verify it passes**

Run: `cargo test --test html_report_observer_styling_test -- --nocapture`  
Expected: PASS.

- [ ] **Step 5: Commit**

```bash
git add rMetal/tests/html_report_observer_styling_test.rs rMetal/src/observer/implementations/html_report_observer.rs
git commit -m "test: cover professional html report styling baseline"
```

### Task 2: Refactor `HtmlReportObserver` into explicit report model + rendering helpers

**Files:**
- Modify: `rMetal/src/observer/implementations/html_report_observer.rs`
- Test: `rMetal/tests/html_report_observer_styling_test.rs`
- Test: `rMetal/tests/html_observer_uses_problem_presentation_test.rs`

- [ ] **Step 1: Write failing test for stable section rendering with existing problem-specific snapshot text**

```rust
#[test]
fn html_report_keeps_problem_specific_solution_text_after_refactor() {
    let problem = StylingProblem::new();
    let run_id = SystemTime::now()
        .duration_since(UNIX_EPOCH)
        .map(|duration| duration.as_nanos())
        .unwrap_or(0);
    let output_dir = std::env::temp_dir().join(format!("rmetal_html_refactor_text_{}", run_id));

    let observer = HtmlReportObserver::new(output_dir.clone()).with_flat_output();
    let parameters = HillClimbingParameters::new(
        BitFlipMutation::new(),
        0.2,
        TerminationCriteria::new(vec![TerminationCriterion::MaxIterations(3)]),
    )
    .with_seed(7);
    let mut algorithm = HillClimbing::new(parameters);
    algorithm.add_observer(Box::new(observer));

    let _ = algorithm.run(&problem).expect("run should succeed");
    let report = std::fs::read_to_string(output_dir.join("report.html")).expect("report readable");

    assert!(report.contains("Best solution found"));
    assert!(report.contains("binary-summary") || report.contains("selected="));
}
```

- [ ] **Step 2: Run tests to verify at least one failure before refactor implementation**

Run: `cargo test --test html_report_observer_styling_test -- --nocapture`  
Expected: FAIL on the new refactor-stability assertion.

- [ ] **Step 3: Implement refactor with private model + renderer functions**

```rust
// Sketch for html_report_observer.rs
#[derive(Clone, Debug)]
struct ReportModel {
    algorithm_name: String,
    status_label: String,
    total_generations: usize,
    total_evaluations: usize,
    best_overall: f64,
    report_path: String,
    best_solution_row_html: String,
    recent_generations_rows_html: String,
    best_updates_rows_html: String,
}

impl HtmlReportObserver {
    fn build_report_model(&self, report_path_display: &str) -> ReportModel {
        // Compute and escape all render inputs once.
    }

    fn render_summary_cards(&self, model: &ReportModel) -> String {
        // Render cards only.
    }

    fn render_html_document(&self, model: &ReportModel, chart_svg: &str) -> String {
        // Assemble final HTML from helper fragments.
    }
}
```

- [ ] **Step 4: Run tests for HTML observer compatibility**

Run: `cargo test --test html_report_observer_styling_test --test html_observer_uses_problem_presentation_test -- --nocapture`  
Expected: PASS.

- [ ] **Step 5: Commit**

```bash
git add rMetal/src/observer/implementations/html_report_observer.rs rMetal/tests/html_report_observer_styling_test.rs
git commit -m "refactor: split html report observer rendering into focused helpers"
```

### Task 3: Add failing test and implement `metrics.json` export in `ChartObserver`

**Files:**
- Create: `rMetal/tests/chart_observer_metrics_artifact_test.rs`
- Modify: `rMetal/src/observer/implementations/chart_observer.rs`
- Test: `rMetal/tests/chart_observer_metrics_artifact_test.rs`

- [ ] **Step 1: Write the failing test for `metrics.json`**

```rust
use rmetal::observer::{AlgorithmEvent, ChartObserver, ObserverState};
use rmetal::observer::AlgorithmObserver;
use std::time::{SystemTime, UNIX_EPOCH};

#[test]
fn chart_observer_writes_metrics_json_alongside_svg_files() {
    let run_id = SystemTime::now()
        .duration_since(UNIX_EPOCH)
        .map(|duration| duration.as_nanos())
        .unwrap_or(0);
    let base = std::env::temp_dir().join(format!("rmetal_chart_metrics_test_{}", run_id));

    let mut observer = ChartObserver::new(base.clone());
    observer.update(&AlgorithmEvent::<bool>::Start {
        algorithm_name: "MetricsAlgorithm".to_string(),
    });
    observer.update(&AlgorithmEvent::<bool>::ExecutionStateUpdated {
        state: ObserverState::new(0, 1, 10, 2.0, 1.5, 1.0, "s=1".to_string()),
    });
    observer.update(&AlgorithmEvent::<bool>::ExecutionStateUpdated {
        state: ObserverState::new(1, 2, 20, 2.5, 2.0, 1.2, "s=2".to_string()),
    });
    observer.update(&AlgorithmEvent::<bool>::End {
        total_generations: 2,
        total_evaluations: 20,
        termination_reason: None,
    });

    let run_path = std::fs::read_dir(base.join("metricsalgorithm"))
        .expect("algorithm folder should exist")
        .next()
        .expect("run directory entry expected")
        .expect("run directory entry valid")
        .path();

    let metrics_path = run_path.join("metrics.json");
    assert!(metrics_path.exists(), "metrics.json should exist");

    let metrics = std::fs::read_to_string(metrics_path).expect("metrics json readable");
    assert!(metrics.contains("\"convergence\""));
    assert!(metrics.contains("\"best_by_evaluations\""));
}
```

- [ ] **Step 2: Run test to verify it fails**

Run: `cargo test --test chart_observer_metrics_artifact_test -- --nocapture`  
Expected: FAIL because `metrics.json` is not written yet.

- [ ] **Step 3: Implement minimal `metrics.json` writer**

```rust
impl ChartObserver {
    fn generate_metrics_json(&self) -> Result<(), Box<dyn std::error::Error>> {
        if self.generations.is_empty() {
            return Ok(());
        }

        let output_file = self.resolve_output_path().join("metrics.json");
        let (generations, best_fitness, avg_fitness, worst_fitness) = self.consolidate_data();

        let mut json = String::from("{\n  \"convergence\": [\n");
        for index in 0..generations.len() {
            let comma = if index + 1 == generations.len() { "" } else { "," };
            json.push_str(&format!(
                "    {{\"generation\":{},\"best\":{:.6},\"average\":{:.6},\"worst\":{:.6}}}{}\n",
                generations[index], best_fitness[index], avg_fitness[index], worst_fitness[index], comma
            ));
        }
        json.push_str("  ],\n  \"best_by_evaluations\": [\n");
        // Build from deduplicated evaluation points.
        json.push_str("  ]\n}\n");

        std::fs::write(output_file, json)?;
        Ok(())
    }
}

// Call generate_metrics_json() from End and finalize.
```

- [ ] **Step 4: Run test to verify it passes**

Run: `cargo test --test chart_observer_metrics_artifact_test -- --nocapture`  
Expected: PASS.

- [ ] **Step 5: Commit**

```bash
git add rMetal/tests/chart_observer_metrics_artifact_test.rs rMetal/src/observer/implementations/chart_observer.rs
git commit -m "feat: export chart observer metrics as json artifact"
```

### Task 4: Compatibility and full observer verification

**Files:**
- Modify: `rMetal/src/observer/implementations/html_report_observer.rs` (polish if needed)
- Modify: `rMetal/src/observer/implementations/chart_observer.rs` (polish if needed)
- Test: `rMetal/tests/html_observer_uses_problem_presentation_test.rs`
- Test: `rMetal/tests/observer_solution_presentation_integration_test.rs`
- Test: `rMetal/tests/html_report_observer_styling_test.rs`
- Test: `rMetal/tests/chart_observer_metrics_artifact_test.rs`

- [ ] **Step 1: Add/adjust regression assertions for output filenames and layout contract**

```rust
assert!(run_path.join("convergence.svg").exists());
assert!(run_path.join("best_by_evaluations.svg").exists());
assert!(run_path.join("metrics.json").exists());
assert!(report.contains("Algorithm execution report"));
assert!(report.contains("Recent generation metrics"));
```

- [ ] **Step 2: Run focused observer test suite**

Run: `cargo test observer -- --nocapture`  
Expected: PASS for observer-related tests.

- [ ] **Step 3: Run full project tests to catch regressions**

Run: `cargo test -- --nocapture`  
Expected: PASS.

- [ ] **Step 4: Final tidy refactor (no behavior changes)**

```rust
// Keep only private helper extraction and naming improvements,
// no public API signature changes.
```

- [ ] **Step 5: Commit**

```bash
git add rMetal/src/observer/implementations/html_report_observer.rs rMetal/src/observer/implementations/chart_observer.rs rMetal/tests/html_report_observer_styling_test.rs rMetal/tests/chart_observer_metrics_artifact_test.rs
git commit -m "test: harden observer compatibility and presentation regressions"
```

## Completion Criteria Checklist

- [ ] `HtmlReportObserver` still works with existing usage and outputs `report.html`.
- [ ] Report uses sober professional native CSS and semantic layout sections.
- [ ] `ChartObserver` still writes both SVG files and now also `metrics.json`.
- [ ] Existing observer tests and new tests all pass.
- [ ] No external dependencies introduced.
