---
name: roma-objective-direction
description: Use when implementing or refactoring scalar optimization comparisons in Roma to preserve maximize versus minimize semantics across algorithms and reports.
---

# Roma Objective Direction

## Overview
Objective direction is framework-level behavior, not algorithm-local preference. Always derive comparison semantics from the problem direction.

## When Not to Use
- Pareto dominance, rank, or crowding logic in multiobjective paths: use `roma-multiobjective-semantics`.

## Mandatory Rules
- Use `problem.get_improvement_direction()` as the source of truth.
- For scalar decisions, use `is_better(candidate, reference, direction)` from `roma/src/algorithms/objective.rs`.
- For loss-based acceptance (for example simulated annealing), use `non_improving_loss`.
- Keep report ordering and best/worst derivation aligned with the same direction.

## Use This In
- `roma/src/algorithms/implementations/*.rs` when selecting survivors or updating best solutions.
- `roma/src/experiment/executor.rs` and related report summarization paths.
- Any utility that computes best/worst scalar values.

## Do Not Do
- Do not hardcode `>` for "better" in algorithm logic.
- Do not assume all scalar qualities are maximization just because `Dominance for f64` uses `>`.
- Do not mix direction-unaware sorting with direction-aware selection in the same path.

## Clarification
- `Dominance for f64` is a generic scalar fallback.
- Algorithm and reporting paths must still use explicit problem direction (`Maximize` or `Minimize`).

## Quick Validation
1. Run one maximize problem (knapsack) and one minimize problem (TSP or ZDT objective proxy).
2. Confirm best values move in opposite numeric directions as expected.
3. Confirm experiment ranking follows objective direction.
