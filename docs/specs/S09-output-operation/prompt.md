# S09: Output Operation

## Feature
Implement the `output` operation type: apply selector, project columns, handle `include_deleted` flag, write to destination via `OutputWriter`, and optionally register the output as a new Dataset.

## Context
- Read: `docs/entities/operation.md` (output operation definition, columns, include_deleted, register_as_dataset, BR-011/012/013)
- Read: `docs/architecture/sample-datasets.md` (TS-07 column projection)

## Scope

### In Scope
- `core::engine::ops::output` module
- Selector filtering (which rows to output)
- Column projection: when `columns` is specified, output only those columns (plus system columns if not stripped)
- `include_deleted: false` (default): exclude `_deleted = true` rows
- `include_deleted: true`: include deleted rows in output
- Write via `OutputWriter` trait
- `register_as_dataset`: create a new Dataset entity via `MetadataStore` with the output schema
- Output can appear mid-pipeline (not just at the end)
- Test scenario TS-07: column projection

### Out of Scope
- Physical write implementations (S16)
- Multiple simultaneous destinations (deferred per operation.md OQ-001)

## Dependencies
- **S01** (DSL Parser), **S03** (Period Filter)

## Parallel Opportunities
Can run in parallel with **S04, S05, S06, S07, S08, S11**.

## Success Criteria
- Column projection outputs only specified columns
- Default excludes deleted rows; `include_deleted: true` includes them
- Selector filters which rows are output
- Mid-pipeline output does not modify the working dataset
- `register_as_dataset` creates a valid Dataset entity
