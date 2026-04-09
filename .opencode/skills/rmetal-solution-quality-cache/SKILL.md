---
name: rmetal-solution-quality-cache
description: Use when mutating Solution variables or quality payloads in rMetal to preserve cache invalidation and reevaluation invariants.
---

# rMetal Solution Quality Cache

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
- `rMetal/src/solution/mod.rs`
- `rMetal/src/algorithms/implementations/genetic_algorithm.rs`
- `rMetal/src/algorithms/implementations/nsga2.rs`
- `rMetal/src/algorithms/implementations/hill_climbing.rs`
- `rMetal/src/algorithms/implementations/simulated_annealing.rs`
- `rMetal/src/algorithms/implementations/pso.rs`
