# S04: Update Operation

## Feature
Implement the `update` operation type: apply selector-based row filtering, compile assignment expressions, execute them against the working `LazyFrame`, and update system columns (`_updated_at`).

## Context
- Read: `docs/entities/operation.md` (update operation definition, Assignment structure, selector, BR-005/007)
- Read: `docs/entities/expression.md` (expression syntax and functions)
- Read: `docs/entities/project.md` (`selectors` map, `{{NAME}}` interpolation)
- Read: `docs/architecture/sample-datasets.md` (TS-03 FX conversion, TS-08 named selector)

## Scope

### In Scope
- `core::engine::ops::update` module
- Selector resolution: parse selector expression (with `{{NAME}}` interpolation), compile to Polars filter `Expr`
- Assignment execution: for each `Assignment`, compile `expression` → Polars `Expr`, apply as `with_column` on filtered rows
- When selector is present: apply assignments only to matching rows; non-matching rows pass through unchanged
- `_updated_at` set to current Run timestamp on every modified row
- New column creation: if assignment targets a column not in the schema, add it (with NULL for non-matching rows)
- Unit tests with inline DataFrames
- Test harness scenarios: TS-03 (FX conversion — requires S05 for joins, so test without joins here), TS-08 (named selector)

### Out of Scope
- RuntimeJoin loading and resolution (S05)
- The joins themselves — test `update` with working dataset columns only; join-dependent scenarios tested in S05

## Dependencies
- **S01** (DSL Parser): expression compilation
- **S02** (Test Harness): test scenario execution
- **S03** (Period Filter): filtered `LazyFrame` as input

## Parallel Opportunities
Can run in parallel with **S05, S06, S07, S08, S09, S11**.

## Key Design Decisions
- Selector is a plain Expression string with `{{NAME}}` convention
- When selector is omitted, operation applies to all non-deleted rows
- Assignments always use `expression` (no direct join column assignment — that's via expression referencing alias)
- `_updated_at` is the only system column modified by update

## Success Criteria
- Selector filters rows correctly; non-matching rows pass through unchanged
- Assignment expressions compute correct values
- New columns are added with NULL for non-matching rows
- `_updated_at` is updated on modified rows only
- `{{NAME}}` selectors interpolate correctly; undefined names produce error
- Empty selector (all rows) works
