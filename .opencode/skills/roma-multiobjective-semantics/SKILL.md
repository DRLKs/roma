---
name: roma-multiobjective-semantics
description: Use when changing NSGA-II or Pareto quality logic in Roma to preserve strict separation between Pareto dominance and rank-crowding selection metadata.
---

# Roma Multiobjective Semantics

## Overview
In Roma, Pareto dominance and rank/crowding are different layers. Dominance defines partial order on objectives; rank and crowding only guide selection among non-dominated candidates.

## Semantic Rules
- Pareto dominance for `ParetoCrowdingDistanceQuality` compares objective vectors only.
- Rank is lower-is-better metadata from non-dominated sorting.
- Crowding distance is a tie-breaker within the same rank.
- Missing or inconsistent objective vectors must not produce false dominance.

## When Not to Use
- Scalar maximize/minimize decisions in single-objective paths: use `roma-objective-direction`.

## Implementation Anchors
- Dominance semantics: `roma/src/solution/traits/dominance.rs`
- Quality payload shape: `roma/src/solution/traits/pareto_crowding_distance_quality.rs`
- NSGA-II ranking/crowding flow: `roma/src/algorithms/implementations/nsga2.rs`

## Do Not Do
- Do not include rank or crowding in dominance checks.
- Do not assume total ordering where Pareto relation is partial.
- Do not invert minimization semantics for objectives in NSGA-II paths.

## Validation
1. Verify dominance tests still pass for non-dominated pairs.
2. Verify front assignment and crowding boundaries remain stable.
3. Verify tournament/replacement uses rank first, crowding second.

## Clarification
- If objective vectors are missing, empty, or mismatched in length, treat solutions as incomparable for dominance in that step.
