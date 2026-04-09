---
name: rmetal-experiment-reporting
description: Use when extending rMetal experiment cases, execution, or reporting so run aggregation, failures, and ranking remain internally consistent.
---

# rMetal Experiment Reporting

## Overview
Experiment reporting in rMetal is a data pipeline: case metadata -> run outcomes -> summaries -> text output. Preserve this flow to avoid misleading comparisons.

## When Not to Use
- Pure algorithm runtime internals with no experiment or reporting impact.

## Consistency Rules
- Keep case identity stable (`algorithm_name`, `case_name`, parameters text).
- Record every failed run with algorithm, case, run index, and explicit error.
- Build summaries only from successful run values.
- Sort summaries according to `ImprovementDirection`.

## Key Components
- Case interface: `rMetal/src/experiment/traits.rs`
- Execution and aggregation: `rMetal/src/experiment/executor.rs`
- Summary formatting: `rMetal/src/experiment/report.rs`
- Parallel job partitioning: `rMetal/src/experiment/parallel.rs`

## Required Checks When Editing
1. Failure accounting remains separate from successful result aggregation.
2. `runs_ok` matches successful values count per case.
3. Best/mean/worst/std-dev are computed from the same value set.
4. Ranking order changes correctly between maximize and minimize objectives.

## Practical Validation
- Run `cargo run --example mono_objective_experiment` and confirm `runs_ok`, ranking order, and failure counts are coherent.

## Common Pitfalls
- Accidentally dropping failures during merge from worker outputs.
- Ranking by mean when current behavior ranks by best.
- Mixing metadata labels across cases with similar parameter values.
