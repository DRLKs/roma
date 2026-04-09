---
name: rmetal-skill-router
description: Use when starting a rMetal coding task and you need to pick the most relevant project skill before changing code.
---

# rMetal Skill Router

## Overview
This skill is an index. Use it to choose one primary `rmetal-*` skill quickly, then load that skill before implementing changes.

## Quick Routing
- Module ownership, API surface placement, cross-cutting refactor: `rmetal-module-map`.
- Scalar maximize/minimize comparisons and ranking direction: `rmetal-objective-direction`.
- `Solution<T, Q>` mutation and reevaluation invariants: `rmetal-solution-quality-cache`.
- Algorithm lifecycle, runtime contract, observer/termination behavior: `rmetal-algorithm-runtime-contract`.
- NSGA-II, Pareto dominance, rank/crowding semantics: `rmetal-multiobjective-semantics`.
- CSV/JSON/YAML parsing and key-path mapping into problems: `rmetal-data-adapter-policy`.
- Experiment cases, aggregation, failures, and report summaries: `rmetal-experiment-reporting`.

## Common Combinations
- New scalar algorithm behavior: `rmetal-algorithm-runtime-contract` + `rmetal-objective-direction`.
- NSGA-II operator/refactor work: `rmetal-multiobjective-semantics` + `rmetal-solution-quality-cache`.
- Data ingestion benchmark flow: `rmetal-data-adapter-policy` + `rmetal-experiment-reporting`.

## When Not to Use
- Tasks that do not modify rMetal source behavior (for example external docs-only edits).
