# S08: Append Operation

## Feature
Implement the `append` operation type: load rows from a source Dataset, optionally filter with `source_selector`, optionally aggregate before appending, align columns with the working dataset, and append the rows.

## Context
- Read: `docs/entities/operation.md` (append operation definition, DatasetRef source, source_selector, AppendAggregation, BR-010)
- Read: `docs/architecture/sample-datasets.md` (TS-06 budget vs actual append)

## Scope

### In Scope
- `core::engine::ops::append` module
- Load source Dataset via `MetadataStore` + `DataLoader` (same Resolver precedence as RuntimeJoin)
- Period filter source data by its `temporal_mode`
- Optional `source_selector` filter on source rows before appending
- Optional aggregation on source rows before appending (group_by + aggregations)
- Column alignment: incoming columns must be a subset of working dataset; extra working columns set to NULL
- Generate `_row_id` for appended rows; set system columns
- Test scenario TS-06: append budgets alongside transactions

### Out of Scope
- Appending from the same working dataset (not a defined use case)

## Dependencies
- **S01** (DSL Parser), **S03** (Period Filter)

## Parallel Opportunities
Can run in parallel with **S04, S05, S06, S07, S09, S11**.

## Success Criteria
- Source Dataset rows are appended to working dataset
- Column alignment: missing columns filled with NULL
- Extra source columns not in working dataset produce error
- `source_selector` filters correctly
- Aggregation before append produces correct summary rows
- System columns are set on appended rows
