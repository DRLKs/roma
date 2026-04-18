---
name: rmetal-solution-quality-cache
description: Use when mutating Solution variables or quality payloads in Roma to preserve cache invalidation and reevaluation invariants.
---

# Roma Solution Quality Cache

## Overview
`Solution<T, Q>` treats quality as a cache. Any variable mutation must imply stale quality and force reevaluation before comparisons.

## When Not to Use
- Read-only reporting or formatting changes that do not mutate solution variables.

## Invariants
- Variable mutation invalidates quality.
- `problem.evaluate(&mut solution)` is the operation that restores valid quality.
- Comparison paths (`quality_value`, dominance, selection) assume evaluated solutions.

## Safe Mutation Pattern
1. Mutate through `variables_mut`, `get_variable_mut`, `set_variable`, `set_variables`, or `swap_variables`.
2. Reevaluate immediately before using quality in ranking or acceptance.
3. Avoid carrying mutated-but-unevaluated solutions across algorithm boundaries.

## Minimal Example
- Before mutation: evaluated solution can be ranked.
- After mutation: quality cache is invalid and must not be trusted.
- After `problem.evaluate(&mut solution)`: ranking is valid again.

## Unsafe Patterns to Avoid
- Mutate variables and read `quality_value()` without reevaluation.
- Copy stale quality semantics into custom helper code.
- Bypass existing mutation APIs and forget invalidation.

## Key Paths
- `roma/src/solution/mod.rs`
- `roma/src/algorithms/implementations/genetic_algorithm.rs`
- `roma/src/algorithms/implementations/nsga2.rs`
- `roma/src/algorithms/implementations/hill_climbing.rs`
- `roma/src/algorithms/implementations/simulated_annealing.rs`
- `roma/src/algorithms/implementations/pso.rs`
