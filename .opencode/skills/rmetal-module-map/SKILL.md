---
name: rmetal-module-map
description: Use when introducing new modules, API surface, or cross-cutting refactors in rMetal and you must choose the owning module before coding.
---

# rMetal Module Map

## Overview
rMetal keeps optimization concepts separated by module. Put changes where the abstraction belongs, not where it is first consumed.

## When to Use
- Adding a new algorithm, problem, operator, observer, experiment component, or utility.
- Refactoring shared behavior and deciding ownership boundaries.
- Reviewing a change that feels cross-cutting or hard to place.

## When Not to Use
- Pure scalar comparison changes: use `rmetal-objective-direction`.
- Pareto/rank/crowding behavior changes: use `rmetal-multiobjective-semantics`.
- Quality invalidation or reevaluation changes: use `rmetal-solution-quality-cache`.
- Experiment aggregation and summary changes: use `rmetal-experiment-reporting`.

## Module Placement Rules
- `rMetal/src/algorithms`: algorithm trait contracts, runtime, termination, and concrete implementations.
- `rMetal/src/problem`: problem trait and concrete optimization problems.
- `rMetal/src/operator`: mutation, crossover, and selection operators.
- `rMetal/src/solution`: `Solution<T, Q>`, quality semantics, and builders.
- `rMetal/src/solution_set`: solution containers and best-value helpers.
- `rMetal/src/observer`: event types and observer implementations.
- `rMetal/src/experiment`: experiment case execution, aggregation, and reporting.
- `rMetal/src/utils`: shared low-level helpers (CLI parsing, random, adapters, statistics).

## Change Checklist
1. Put new logic in its owning module.
2. Keep module internals private unless there is a stable public use case.
3. Update `rMetal/src/lib.rs` re-exports only for intentional API surface changes.
4. Add unit tests near the changed module and integration tests for cross-module behavior.
5. Add or update examples in `rMetal/examples` when the feature is user-facing.

## Common Mistakes
- Hiding domain logic in `utils` because it is convenient.
- Mixing algorithm-specific behavior into `problem` or `solution`.
- Re-exporting internal helpers in `lib.rs` without a stable API reason.
