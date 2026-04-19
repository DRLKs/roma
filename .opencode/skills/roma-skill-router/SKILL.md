---
name: roma-skill-router
description: Use when starting a Roma coding task and you need to pick the most relevant project skill before changing code.
---

# Roma Skill Router

## Overview
This skill is an index. Use it to choose one primary `roma-*` skill quickly, then load that skill before implementing changes.

## Quick Routing
- Module ownership, API surface placement, cross-cutting refactor: `roma-module-map`.
- Scalar maximize/minimize comparisons and ranking direction: `roma-objective-direction`.
- `Solution<T, Q>` mutation and reevaluation invariants: `roma-solution-quality-cache`.
- Algorithm lifecycle, runtime contract, observer/termination behavior: `roma-algorithm-runtime-contract`.
- NSGA-II, Pareto dominance, rank/crowding semantics: `roma-multiobjective-semantics`.
- CSV/JSON/YAML parsing and key-path mapping into problems: `roma-data-adapter-policy`.
- Experiment cases, aggregation, failures, and report summaries: `roma-experiment-reporting`.

## Common Combinations
- New scalar algorithm behavior: `roma-algorithm-runtime-contract` + `roma-objective-direction`.
- NSGA-II operator/refactor work: `roma-multiobjective-semantics` + `roma-solution-quality-cache`.
- Data ingestion benchmark flow: `roma-data-adapter-policy` + `roma-experiment-reporting`.

## When Not to Use
- Tasks that do not modify Roma source behavior (for example external docs-only edits).
