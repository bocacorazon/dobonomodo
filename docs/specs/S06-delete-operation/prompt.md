# S06: Delete Operation

## Feature
Implement the `delete` operation type: apply selector-based row filtering and set `_deleted = true` on matching rows. Verify automatic exclusion of deleted rows from all subsequent operations.

## Context
- Read: `docs/entities/operation.md` (delete operation definition, BR-004, BR-019)
- Read: `docs/entities/dataset.md` (BR-022 — `_deleted` lifecycle)
- Read: `docs/architecture/sample-datasets.md` (TS-04 soft delete scenario)

## Scope

### In Scope
- `core::engine::ops::delete` module
- Selector compilation and application (reuse from S04)
- Set `_deleted = true` on matching rows; `_updated_at` updated
- Verify deleted rows are excluded from downstream operations in the pipeline
- Test scenario TS-04: soft delete inactive account lines

### Out of Scope
- Hard delete (not in the model)
- Undeletion

## Dependencies
- **S01** (DSL Parser), **S03** (Period Filter)

## Parallel Opportunities
Can run in parallel with **S04, S05, S07, S08, S09, S11**.

## Success Criteria
- Matching rows have `_deleted = true` after operation
- Non-matching rows are unchanged
- Deleted rows are excluded from subsequent operations in the pipeline
- Empty selector (delete all) works
