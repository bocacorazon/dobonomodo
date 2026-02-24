# S09: Output Operation

**Status**: Planning Complete  
**Feature ID**: 010-output-operation  
**Created**: 2026-02-23  
**Source**: `/workspace/docs/specs/S09-output-operation/prompt.md`

---

## Feature

Implement the `output` operation type: apply selector, project columns, handle `include_deleted` flag, write to destination via `OutputWriter`, and optionally register the output as a new Dataset.

---

## Context

- **Read**: `docs/entities/operation.md` (output operation definition, columns, include_deleted, register_as_dataset, BR-011/012/013)
- **Read**: `docs/architecture/sample-datasets.md` (TS-07 column projection)

---

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

---

## Dependencies

- **S01** (DSL Parser)
- **S03** (Period Filter)

---

## Parallel Opportunities

Can run in parallel with **S04, S05, S06, S07, S08, S11**.

---

## Success Criteria

- Column projection outputs only specified columns
- Default excludes deleted rows; `include_deleted: true` includes them
- Selector filters which rows are output
- Mid-pipeline output does not modify the working dataset
- `register_as_dataset` creates a valid Dataset entity

---

## User Scenarios

### US-01: Basic Output (All Rows, All Columns)

**As a** data engineer  
**I want to** output the entire working dataset to a destination  
**So that** I can persist pipeline results

**Acceptance Criteria**:
- Given a working dataset with 1000 rows and 10 columns
- When I execute an output operation with no selector and no column projection
- Then all 1000 rows and all 10 columns are written to the destination
- And the working dataset remains unchanged

---

### US-02: Column Projection (TS-07)

**As a** data engineer  
**I want to** output only specific columns from the working dataset  
**So that** I can reduce output size and comply with data minimization

**Acceptance Criteria**:
- Given a working dataset with columns: `[journal_id, line_number, posting_date, account_code, cost_center_code, currency, amount_local, amount_reporting, description, source_system]`
- When I execute an output operation with `columns: [journal_id, account_code, amount_local, amount_reporting]`
- Then the output contains only 4 columns per row
- And the column order matches the specification
- And all 10 rows from the sample dataset are present
- And the working dataset remains unchanged

**Test Scenario**: TS-07 from `docs/architecture/sample-datasets.md`

---

### US-03: Exclude Deleted Rows (Default Behavior)

**As a** data engineer  
**I want to** exclude soft-deleted rows from output by default  
**So that** downstream consumers don't process deleted data

**Acceptance Criteria**:
- Given a working dataset with 100 rows, 10 of which have `_deleted = true`
- When I execute an output operation with `include_deleted: false` (or omitted)
- Then the output contains 90 rows (deleted rows excluded)
- And no row in the output has `_deleted = true`
- And the working dataset still contains all 100 rows

---

### US-04: Include Deleted Rows

**As a** auditor  
**I want to** include soft-deleted rows in output  
**So that** I can generate audit trails of all data changes

**Acceptance Criteria**:
- Given a working dataset with 100 rows, 10 of which have `_deleted = true`
- When I execute an output operation with `include_deleted: true`
- Then the output contains all 100 rows
- And 10 rows in the output have `_deleted = true`

---

### US-05: Filtered Output (Selector)

**As a** data engineer  
**I want to** output only rows matching a condition  
**So that** I can create targeted datasets for specific use cases

**Acceptance Criteria**:
- Given a working dataset with 1000 rows
- When I execute an output operation with `selector: "amount > 10000 AND region = 'EMEA'"`
- Then only rows satisfying the condition are written to the output
- And the working dataset remains unchanged
- And the selector is evaluated before the deleted flag filter

---

### US-06: Register Output as Dataset

**As a** data engineer  
**I want to** register the output as a named Dataset  
**So that** I can reuse the output as input to another Project

**Acceptance Criteria**:
- Given a successful output write with 500 rows
- When I execute an output operation with `register_as_dataset: "monthly_summary"`
- Then a new Dataset entity is created with:
  - Name: "monthly_summary"
  - Version: 1 (or incremented if dataset already exists)
  - Schema: matching the output columns (post-projection)
  - Status: Active
- And the Dataset ID is returned in the operation result
- And the Dataset can be queried via `MetadataStore.get_dataset_by_name("monthly_summary")`

---

### US-07: Mid-Pipeline Output (Checkpoint)

**As a** data engineer  
**I want to** output intermediate results mid-pipeline  
**So that** I can checkpoint data for debugging or auditing

**Acceptance Criteria**:
- Given a pipeline with operations: [update, output (checkpoint), aggregate, output (final)]
- When the checkpoint output executes after the update operation
- Then the working dataset passed to the aggregate operation is identical to the dataset passed to the checkpoint output
- And the checkpoint output does not modify the working dataset
- And both outputs succeed independently

---

### US-08: Write Failure Handling

**As a** system operator  
**I want to** receive clear error messages when output write fails  
**So that** I can diagnose and fix the issue

**Acceptance Criteria**:
- Given an output operation with an inaccessible destination (e.g., disk full, no permissions)
- When the output operation executes
- Then the operation fails with `OutputError::WriteFailed`
- And the error message includes the root cause (e.g., "Permission denied", "Disk full")
- And no Dataset is registered (if `register_as_dataset` was set)

---

### US-09: Column Projection Validation

**As a** data engineer  
**I want to** be notified if I specify invalid column names  
**So that** I can correct my configuration before execution

**Acceptance Criteria**:
- Given a working dataset with columns: `[id, name, amount]`
- When I execute an output operation with `columns: [id, price, quantity]` (invalid columns)
- Then the operation fails with `OutputError::ColumnProjectionError { missing: ["price", "quantity"] }`
- And the error message lists the available columns
- And no data is written to the destination

---

## Business Rules

From `docs/entities/operation.md`:

- **BR-011**: `output` is the only operation type permitted to perform IO. All other types operate only in-memory on the working dataset.
- **BR-012**: `output` MAY appear at any position in the pipeline, including mid-pipeline, to support checkpointing.
- **BR-013**: `output` with `include_deleted: false` (the default) MUST NOT write rows where `_deleted = true`, regardless of the selector.
- **BR-015**: All Expression column references in an Operation MUST resolve to columns in the working dataset or to a `join` alias defined in the same operation. Unknown references are a compile-time error.

---

## Technical Requirements

### TR-01: Selector Filtering

- Selector is an optional boolean Expression
- Evaluated against the working dataset to filter rows
- Invalid or non-boolean selectors result in `OutputError::SelectorError`

### TR-02: Column Projection

- `columns` is an optional list of column names
- When specified, output contains only those columns
- Column order in output matches order in `columns` parameter
- Invalid column names result in `OutputError::ColumnProjectionError`
- System columns (`_row_id`, `_deleted`, `_period`) can be included or excluded explicitly

### TR-03: Deleted Row Handling

- Default behavior (`include_deleted: false`): exclude rows with `_deleted = true`
- Explicit opt-in (`include_deleted: true`): include all rows regardless of `_deleted` flag
- Deleted flag filter is applied AFTER selector filter

### TR-04: Output Writing

- Uses `OutputWriter` trait (defined in `core::engine::io_traits`)
- Write operation must succeed before dataset registration
- Write failures propagate as `OutputError::WriteFailed`

### TR-05: Dataset Registration

- Optional feature controlled by `register_as_dataset` parameter
- Creates new Dataset entity via `MetadataStore` trait
- Dataset schema is extracted from output DataFrame (post-projection)
- Dataset version is incremented if name already exists
- Registration occurs ONLY after successful write
- Registration failure is logged as warning but does NOT fail the operation

### TR-06: Immutability

- Output operation MUST NOT modify the working dataset
- All transformations use lazy evaluation (LazyFrame)
- Working dataset is cloned (cheap Arc-based copy) before transformations

### TR-07: Performance

- Memory usage proportional to `(rows × selected_columns)`, not `(rows × all_columns)`
- LazyFrame is collected to DataFrame exactly once (at write time)
- Filter operations applied before column projection for maximum efficiency

---

## Non-Functional Requirements

### NFR-01: Memory Efficiency

- Process datasets with millions of rows without out-of-memory errors
- Use Polars LazyFrame with deferred execution
- Apply column projection before materialization

### NFR-02: Error Clarity

- All error types use descriptive messages with context
- Error messages include actionable information (e.g., list of missing columns)

### NFR-03: Test Coverage

- Minimum 80% line coverage for new code
- 100% coverage for critical paths (selector evaluation, column projection, deleted flag handling)
- Contract test TS-07 must pass

---

## Constraints

- **C-01**: Must use existing `OutputWriter` trait (no changes to interface)
- **C-02**: Must use existing `MetadataStore` trait (no changes to interface)
- **C-03**: Rust edition 2021, compatible with Polars 0.46
- **C-04**: No external dependencies beyond workspace dependencies (anyhow, thiserror, polars, serde)

---

## Open Questions

*All open questions resolved during planning phase. See `/workspace/specs/010-output-operation/research.md` for decisions.*

---

## References

- **Entity Definition**: `/workspace/docs/entities/operation.md`
- **Test Scenarios**: `/workspace/docs/architecture/sample-datasets.md` (TS-07)
- **Implementation Plan**: `/workspace/specs/010-output-operation/plan.md`
- **Research**: `/workspace/specs/010-output-operation/research.md`
- **Data Model**: `/workspace/specs/010-output-operation/data-model.md`
- **API Contract**: `/workspace/specs/010-output-operation/contracts/api.md`
- **Quickstart**: `/workspace/specs/010-output-operation/quickstart.md`
