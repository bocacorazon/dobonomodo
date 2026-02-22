# S07: Aggregate Operation

## Feature
Implement the `aggregate` operation type: group rows by specified columns, compute aggregate expressions, and **append** summary rows to the working dataset (not replace existing rows).

## Context
- Read: `docs/entities/operation.md` (aggregate operation definition, BR-009)
- Read: `docs/entities/expression.md` (aggregate functions: SUM, COUNT, AVG, MIN_AGG, MAX_AGG)
- Read: `docs/architecture/sample-datasets.md` (TS-05 monthly totals by account type)

## Scope

### In Scope
- `core::engine::ops::aggregate` module
- Group-by column specification
- Aggregate expression compilation (SUM, COUNT, AVG, MIN_AGG, MAX_AGG) → Polars `.group_by().agg()`
- Generate new `_row_id` (UUID v7) for each summary row
- Set system columns on summary rows: `_created_at`, `_updated_at`, `_source_dataset_id`, `_source_table`, `_deleted: false`
- Append summary rows to working dataset (original rows preserved)
- Columns not in group-by or aggregations are NULL on summary rows
- Test scenario TS-05: monthly totals by account type

### Out of Scope
- Aggregate functions in non-aggregate context (compile error — handled by S01)

## Dependencies
- **S01** (DSL Parser), **S03** (Period Filter)

## Parallel Opportunities
Can run in parallel with **S04, S05, S06, S08, S09, S11**.

## Success Criteria
- Summary rows are appended, not replacing originals
- Group-by produces correct groups
- Aggregate expressions compute correct values
- New rows have valid `_row_id`, system columns
- Non-aggregated columns are NULL on summary rows
