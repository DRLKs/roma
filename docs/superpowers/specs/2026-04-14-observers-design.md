# Observer Modernization Design (ChartObserver + HtmlReportObserver)

Date: 2026-04-14  
Project: rMetal  
Status: Approved in conversation (ready for implementation planning)

## 1. Context and Problem

The current observer outputs are functional but visually weak and internally hard to evolve:

- `ChartObserver` produces useful charts, but they are perceived as basic and offer limited reusable data artifacts.
- `HtmlReportObserver` creates a valid report, but the page design is plain and the rendering logic is monolithic (`generate_report` contains too many responsibilities).

The user requested a significant improvement, specifically:

1. better chart/report quality,
2. CSS-based report styling,
3. refactor of `HtmlReportObserver`,
4. ideas for future observer improvements.

## 2. Goals

### Functional goals

- Keep current public API compatibility for both observers.
- Keep current output file names and default output folder behavior.
- Improve `report.html` with a sober professional style.
- Refactor `HtmlReportObserver` internals into clearer units.
- Improve `ChartObserver` robustness and add reusable metrics export.

### Non-functional goals

- Preserve zero external dependencies philosophy.
- Use native HTML/CSS and, at most, minimal optional JS.
- Keep behavior deterministic and testable.

## 3. Constraints and Invariants

- No breaking API changes:
  - `ChartObserver::{new,new_default,with_flat_output,with_dimensions}` unchanged.
  - `HtmlReportObserver::{new,new_default,with_flat_output}` unchanged.
- Keep existing output artifacts:
  - `convergence.svg`,
  - `best_by_evaluations.svg`,
  - `report.html`.
- Keep existing run directory policy (`<base>/<algorithm_slug>/run_<timestamp_ms>_<pid>/`) unless `with_flat_output` is used.
- No third-party CSS/JS/CDN dependencies.

## 4. Chosen Approach (A)

Refactor both observers internally while preserving external API and outputs, then add visual and data-quality upgrades.

Why this approach:

- Large user-visible quality improvement.
- Low compatibility risk.
- Better maintainability without architectural overreach.

## 5. Detailed Design

### 5.1 `HtmlReportObserver` internal refactor

The observer will keep the same events and outputs, but `generate_report` will be split into private responsibilities.

#### Proposed internal structure (private functions)

1. **Data preparation**
   - `build_report_model(...) -> ReportModel`
   - Computes all data needed for rendering:
     - sanitized algorithm name,
     - status text and severity,
     - totals,
     - best overall,
     - bounded rows for recent metrics and snapshots.

2. **Rendering helpers**
   - `render_css() -> &'static str`
   - `render_summary_cards(model: &ReportModel) -> String`
   - `render_best_solution_table(model: &ReportModel) -> String`
   - `render_recent_metrics_table(model: &ReportModel) -> String`
   - `render_snapshots_table(model: &ReportModel) -> String`
   - `render_chart_section(chart_svg: &str) -> String`
   - `render_html_document(model: &ReportModel, chart_svg: &str) -> String`

3. **IO and paths**
   - keep `report_file_path`, `report_file_url`, and terminal hyperlink behavior.

#### Refactor outcomes

- Smaller, testable rendering functions.
- Easier styling changes without touching state/event logic.
- Lower risk of accidental regressions when extending report sections.

### 5.2 `HtmlReportObserver` visual system (sober professional)

The report becomes a technical-document style page:

- Neutral palette (light background, subtle borders, restrained accent color).
- Dense but readable information hierarchy.
- Consistent spacing scale and border radius.
- Numeric readability improvements (`tabular-nums`, right-aligned numeric columns).
- Responsive behavior for narrow screens.

#### CSS architecture

- Use `:root` CSS variables for:
  - colors,
  - typography,
  - spacing,
  - radius,
  - shadows.
- Semantic class names:
  - `.page`, `.header`, `.kpi-grid`, `.card`, `.status-badge`,
  - `.section`, `.table-wrap`, `.table`, `.numeric`, `.muted`.
- Keep styles inline inside `report.html` (self-contained artifact).

#### HTML structure

- Semantic layout with `header` + `main` + `section` blocks.
- KPI summary as compact cards.
- Chart section with framed container.
- Two metric tables with horizontal overflow handling.
- Error block only when execution ended with error.

### 5.3 `ChartObserver` improvements (non-breaking)

#### Data consolidation and stability

- Clarify and isolate consolidation logic:
  - deterministic ordering,
  - stable handling of repeated generation/evaluation points,
  - explicit behavior for empty/degenerate windows.

#### Reusable metrics artifact

- Add `metrics.json` in the same output directory (in addition to existing SVG files).
- Content includes processed series for:
  - `convergence` (`generation`, `best`, `average`, `worst`),
  - `best_by_evaluations` (`evaluations`, `best`).
- JSON generated using standard library only.

#### User-visible outcome

- Existing chart files remain.
- Consumers gain machine-readable observer data for downstream tooling.

### 5.4 Event/runtime contract preservation

No changes to observer event handling contract:

- Keep handling of `Start`, `ExecutionStateUpdated`, `End`.
- Keep `seq_id` monotonic filtering behavior.
- Keep `finalize()` generation fallback behavior.

## 6. Testing Strategy

### Existing tests kept green

- `html_observer_uses_problem_presentation_test.rs`
- `observer_solution_presentation_integration_test.rs`
- existing observer unit tests in implementation files.

### New/updated tests

1. **HTML structure and style tests**
   - verify `report.html` includes CSS variables and expected semantic blocks.
   - verify key sections are present (`summary`, `fitness chart`, metric tables).

2. **Compatibility tests**
   - verify existing file names and output paths remain unchanged.

3. **Chart metrics artifact tests**
   - verify `metrics.json` is generated.
   - verify expected keys and non-empty arrays when snapshots exist.

4. **Degenerate data tests**
   - ensure report/chart generation still succeeds with minimal snapshots.

## 7. Risks and Mitigations

- **Risk:** Large inline HTML template regressions.  
  **Mitigation:** split renderer into helper functions + focused tests per section.

- **Risk:** JSON export drift from chart rendering data.  
  **Mitigation:** derive JSON from the same consolidated series used for chart creation.

- **Risk:** accidental output path behavior changes.  
  **Mitigation:** preserve path helper methods and validate with tests.

## 8. Acceptance Criteria

The implementation is accepted when all are true:

1. Public API signatures and usage remain compatible.
2. `report.html` shows sober professional styling using native CSS only.
3. `HtmlReportObserver` code is internally refactored into clear private units.
4. `ChartObserver` still generates existing SVG files and additionally writes `metrics.json`.
5. Observer tests pass, including new coverage for style/structure/artifacts.

## 9. Future Improvement Ideas (Backlog)

These are intentionally out of scope for this increment but recommended next:

1. **Observer composition layer**
   - shared internal utilities for output layout, run metadata, and path naming.

2. **Multi-run comparison report**
   - aggregate `metrics.json` from multiple runs into one comparative HTML report.

3. **Quality-direction-aware visualization modes**
   - optional labels/legends reflecting maximize/minimize semantics explicitly.

4. **Delta annotations**
   - show improvements per generation/evaluation step in tables.

5. **Lightweight filtering controls**
   - optional tiny JS (native) for in-page row filtering and column sorting.

6. **Export bundle mode**
   - single zip-like folder contract with report + charts + metrics metadata manifest.
