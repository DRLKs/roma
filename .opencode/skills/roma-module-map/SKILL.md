---
name: roma-module-map
description: Use when introducing new modules, API surface, or cross-cutting refactors in Roma and you must choose the owning module before coding.
---

# Roma Module Map

## Overview
Roma keeps optimization concepts separated by module. Put changes where the abstraction belongs, not where it is first consumed.

## When to Use
- Adding a new algorithm, problem, operator, observer, experiment component, or utility.
- Refactoring shared behavior and deciding ownership boundaries.
- Reviewing a change that feels cross-cutting or hard to place.

## When Not to Use
- Pure scalar comparison changes: use `roma-objective-direction`.
- Pareto/rank/crowding behavior changes: use `roma-multiobjective-semantics`.
- Quality invalidation or reevaluation changes: use `roma-solution-quality-cache`.
- Experiment aggregation and summary changes: use `roma-experiment-reporting`.

## Module Placement Rules
- `roma/src/algorithms`: algorithm trait contracts, runtime, termination, and concrete implementations.
- `roma/src/problem`: problem trait and concrete optimization problems.
- `roma/src/operator`: mutation, crossover, and selection operators.
- `roma/src/solution`: `Solution<T, Q>`, quality semantics, and builders.
- `roma/src/solution_set`: solution containers and best-value helpers.
- `roma/src/observer`: event types and observer implementations.
- `roma/src/experiment`: experiment case execution, aggregation, and reporting.
- `roma/src/utils`: shared low-level helpers (CLI parsing, random, adapters, statistics).

## Change Checklist
1. Put new logic in its owning module.
2. Keep module internals private unless there is a stable public use case.
3. Update `roma/src/lib.rs` re-exports only for intentional API surface changes.
4. Add unit tests near the changed module and integration tests for cross-module behavior.
5. Add or update examples in `roma/examples` when the feature is user-facing.

## Common Mistakes
- Hiding domain logic in `utils` because it is convenient.
- Mixing algorithm-specific behavior into `problem` or `solution`.
- Re-exporting internal helpers in `lib.rs` without a stable API reason.
