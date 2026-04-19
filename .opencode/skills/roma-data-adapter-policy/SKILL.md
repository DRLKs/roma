---
name: roma-data-adapter-policy
description: Use when loading external CSV, JSON, or YAML records into Roma problems so key mapping, parsing, and error behavior remain consistent.
---

# Roma Data Adapter Policy

## Overview
Data ingestion is adapter-first, then problem-builder validation. Keep source-format parsing and domain mapping responsibilities separate.

## When Not to Use
- In-memory toy datasets that do not parse external CSV/JSON/YAML files.

## Policy
- Use format adapters in `roma/src/utils` to produce normalized records.
- Apply domain mapping in problem builders (for example `build_knapsack_from_records`, `build_tsp_from_records`).
- Keep mapping keys explicit and predictable.
- Prefer deterministic behavior for malformed rows: skip invalid records, fail with clear error when no valid dataset remains.

## Placement
- Parsing concerns belong to:
  - `roma/src/utils/csv_adapter.rs`
  - `roma/src/utils/json_adapter.rs`
  - `roma/src/utils/yaml_adapter.rs`
- Domain construction belongs to:
  - `roma/src/problem/implementations/knapsack_problem.rs`
  - `roma/src/problem/implementations/tsp_problem.rs`

## CLI Integration
- Keep format inference and flag parsing in `roma/src/utils/cli.rs`.
- Keep sample input-policy behavior in examples, not in core traits.

## Common Mistakes
- Mixing schema discovery logic into core problem implementations.
- Treating source-specific quirks as algorithm behavior.
- Returning generic parse errors instead of actionable key-path messages.

## Error Message Pattern
- Prefer explicit field-path guidance, for example: `Missing JSON key 'problem.capacity'`.
