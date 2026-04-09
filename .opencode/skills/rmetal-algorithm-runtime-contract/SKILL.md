---
name: rmetal-algorithm-runtime-contract
description: Use when adding, refactoring, or overriding Algorithm execution paths in rMetal to keep the shared initialize-step-snapshot-finalize runtime lifecycle intact.
---

# rMetal Algorithm Runtime Contract

## Overview
All algorithms in rMetal implement one lifecycle through the `Algorithm` trait and runtime helpers. Keep behavior consistent so observers, termination, and reports stay correct.

## Contract
- Implement `Algorithm<T, Q>` with coherent `StepState`.
- Lifecycle order is: initialize state -> snapshot/report -> step loop with snapshots -> finalize into solution set.
- `validate_parameters` must reject invalid configurations early.
- `termination_criteria` must be non-empty and passed unchanged to runtime.
- If overriding default execution behavior, preserve observer and termination semantics from runtime helpers.

## Required Implementation Checklist
1. `initialize_step_state`: create evaluated initial state.
2. `step`: mutate state and update counters deterministically.
3. `snapshot`: return coherent iteration/evaluation metrics and best solution.
4. `finalize_step_state`: produce final `SolutionSet` from state.
5. `set_solution_set` and `get_solution_set`: preserve last run output.

## Observer and Termination Notes
- Do not emit observer events manually when using trait default `run`; runtime handles this.
- Keep `iteration` and `evaluations` monotonic to avoid misleading termination and reports.
- Respect `ExecutionContext` boundaries; do not duplicate runtime concerns inside algorithm logic.

## Minimum Verification
- Confirm snapshots keep monotonic `iteration` and `evaluations` values.
- Confirm observer `Start` and `End` events are still emitted once per run.

## Relevant Files
- `rMetal/src/algorithms/traits.rs`
- `rMetal/src/algorithms/runtime.rs`
- `rMetal/src/algorithms/termination.rs`
