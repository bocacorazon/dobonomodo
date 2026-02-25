# Data Model: Output Operation

**Feature**: 010-output-operation  
**Date**: 2026-02-23  
**Phase**: Design (Phase 1)

---

## Overview

This document defines the data structures and state model for the `output` operation type. The operation reads from the working dataset (LazyFrame), applies filtering and projection, writes to a destination, and optionally registers the output as a Dataset.

---

## Entities

### OutputOperation

Represents a configured output operation within a pipeline.

**Fields**:
| Field | Type | Required | Validation | Description |
|-------|------|----------|------------|-------------|
| `destination` | `OutputDestination` | Yes | Non-null | Target location for output |
| `selector` | `Option<Expression>` | No | Valid boolean expr if present | Row filter (null = all non-deleted rows) |
| `columns` | `Option<Vec<String>>` | No | Non-empty if present, valid column names | Column projection (null = all columns) |
| `include_deleted` | `bool` | No | — | Default `false`; when `true`, include rows with `_deleted=true` |
| `register_as_dataset` | `Option<String>` | No | Non-empty if present | Dataset name for registration |

**Relationships**:
- **Uses**: `OutputDestination` (target location)
- **Evaluates**: `Expression` (selector)
- **Produces**: `OutputResult` (execution outcome)
- **May create**: `Dataset` (if `register_as_dataset` is set)

**State Transitions**: None (operations are stateless; state is in the enclosing Project)

**Business Rules**:
- BR-011: Output is the only operation type permitted to perform IO
- BR-012: Output may appear at any position in the pipeline
- BR-013: `include_deleted=false` MUST NOT write rows where `_deleted=true`
- BR-015: Column references in selector must resolve to working dataset columns

---

### OutputDestination

Represents the target location for output data.

**Fields**:
| Field | Type | Required | Description |
|-------|------|----------|-------------|
| `datasource_id` | `Uuid` | Yes (one of) | Reference to a DataSource entity |
| `table` | `String` | Yes (one of) | Table name within the datasource |
| `location` | `ResolvedLocation` | Yes (one of) | Direct file/object location |

**Variants**:
1. **DataSource reference**: `{ datasource_id, table }`
2. **Direct location**: `{ location }` (inline path/URI)

**Validation**:
- Exactly one variant must be specified
- If `datasource_id` is set, `table` must be non-empty
- If `location` is set, it must be a valid ResolvedLocation

---

### OutputResult

Represents the outcome of executing an output operation.

**Fields**:
| Field | Type | Description |
|-------|------|-------------|
| `rows_written` | `usize` | Number of rows written to destination |
| `columns_written` | `Vec<String>` | List of column names in output |
| `dataset_id` | `Option<Uuid>` | ID of registered Dataset (if `register_as_dataset` was set) |
| `write_duration_ms` | `u64` | Time taken to write data (excluding registration) |

**Usage**: Returned from `OutputOperation::execute()` for observability and tracing.

---

### OutputSchema

Represents the schema of data being output (extracted from the projected DataFrame).

**Fields**:
| Field | Type | Description |
|-------|------|-------------|
| `columns` | `Vec<ColumnDef>` | Ordered list of columns in output |
| `temporal_mode` | `TemporalMode` | Period/Bitemporal/None (inherited from working dataset) |

**ColumnDef**:
| Field | Type | Description |
|-------|------|-------------|
| `name` | `String` | Column name |
| `data_type` | `ColumnType` | Data type (String, Integer, Decimal, Date, Boolean, etc.) |
| `nullable` | `bool` | Whether column can contain NULL values |

**Usage**: Extracted from output DataFrame and passed to `MetadataStore.register_dataset()` when creating a Dataset.

---

## State Machine

### OutputOperation Execution Flow

```
┌─────────────────────────────────────────────────────────────────────┐
│ Input: working_dataset (LazyFrame), OutputOperation config          │
└────────────────────────────┬────────────────────────────────────────┘
                             │
                             ▼
                    ┌────────────────────┐
                    │ Clone working      │
                    │ dataset (LazyFrame)│
                    └────────┬───────────┘
                             │
                             ▼
                    ┌────────────────────┐
                    │ Apply selector     │ (if present)
                    │ filter expression  │
                    └────────┬───────────┘
                             │
                             ▼
                    ┌────────────────────┐
                    │ Apply _deleted     │ (if include_deleted=false)
                    │ filter (!=true)    │
                    └────────┬───────────┘
                             │
                             ▼
                    ┌────────────────────┐
                    │ Project columns    │ (if specified)
                    │ (select subset)    │
                    └────────┬───────────┘
                             │
                             ▼
                    ┌────────────────────┐
                    │ Collect LazyFrame  │
                    │ → DataFrame        │
                    └────────┬───────────┘
                             │
                             ▼
                    ┌────────────────────┐
                    │ Extract schema     │ (for potential registration)
                    │ from DataFrame     │
                    └────────┬───────────┘
                             │
                             ▼
                    ┌────────────────────┐
                    │ Write via          │
                    │ OutputWriter trait │
                    └────────┬───────────┘
                             │
                        Success? ──No──► ┌─────────────┐
                             │           │ Return      │
                             │           │ WriteFailed │
                             │           │ error       │
                             Yes         └─────────────┘
                             │
                             ▼
                    ┌────────────────────┐
                    │ register_as_dataset│ (if set)
                    │ set?               │
                    └────────┬───────────┘
                             │
                             │ No ──────────────────┐
                             │ Yes                  │
                             ▼                      │
                    ┌────────────────────┐          │
                    │ Register Dataset   │          │
                    │ via MetadataStore  │          │
                    └────────┬───────────┘          │
                             │                      │
                        Success?                    │
                             │                      │
                             Yes (capture ID)       │
                             │                      │
                             │◄─────────────────────┘
                             │
                             ▼
                    ┌────────────────────┐
                    │ Return             │
                    │ OutputResult       │
                    └────────────────────┘
```

**Error States**:
- **SelectorEvaluationFailed**: Invalid or non-boolean selector expression
- **ColumnProjectionFailed**: Specified column doesn't exist in working dataset
- **WriteFailed**: OutputWriter.write() returned error
- **RegistrationFailed**: MetadataStore.register_dataset() failed (warning, not fatal)

**Invariants**:
1. Working dataset is **never mutated** (read-only operation)
2. Output DataFrame is materialized **only once** (at .collect())
3. Registration **only occurs after successful write**
4. Deleted rows are excluded **unless explicitly opted in**

---

## Validation Rules

### Pre-Execution Validation

| Rule ID | Validation | Error Type |
|---------|------------|------------|
| V-001 | Selector (if present) must be a valid boolean Expression | `InvalidSelector` |
| V-002 | Columns (if present) must all exist in working dataset schema | `ColumnProjectionError` |
| V-003 | Destination must specify exactly one of: datasource_id+table OR location | `InvalidDestination` |
| V-004 | If `register_as_dataset` is set, it must be non-empty string | `InvalidDatasetName` |

### Runtime Validation

| Rule ID | Validation | Error Type |
|---------|------------|------------|
| V-005 | Selector evaluation must produce boolean values | `SelectorEvaluationError` |
| V-006 | OutputWriter.write() must succeed | `WriteFailed` |
| V-007 | Registered Dataset schema must be valid (non-empty columns) | `InvalidSchema` |

---

## Schema Evolution

### Column Projection Impact

When `columns` is specified, the output schema is a **subset** of the working dataset schema:

**Example**:
```yaml
# Working dataset schema:
columns: [journal_id, line_number, posting_date, account_code, amount_local, amount_reporting]

# Output operation:
columns: [journal_id, account_code, amount_local, amount_reporting]

# Output schema (projected):
columns: [journal_id, account_code, amount_local, amount_reporting]
```

**Rules**:
- Column order in output matches order specified in `columns` parameter
- Column types are preserved from working dataset
- System columns (`_row_id`, `_deleted`, `_period`) are included unless explicitly excluded

### Dataset Registration Schema

When `register_as_dataset` is set, the registered Dataset uses the **output schema** (post-projection):

```rust
Dataset {
    id: <generated-uuid>,
    name: register_as_dataset.clone(),
    version: <1 or incremented>,
    main_table: TableRef {
        name: destination.table.clone(),
        temporal_mode: <inherited-from-working-dataset>,
        columns: output_schema.columns,  // Projected columns
    },
    // ... other fields
}
```

**Temporal Mode Inheritance**:
- If working dataset has `_period` column → `temporal_mode: Period`
- If working dataset has `_valid_from/_valid_to` → `temporal_mode: Bitemporal`
- Otherwise → `temporal_mode: None`

---

## Integration Points

### Dependencies

| Component | Interface | Usage |
|-----------|-----------|-------|
| **OutputWriter** | `write(&DataFrame, &OutputDestination) -> Result<()>` | Physical write implementation |
| **MetadataStore** | `register_dataset(Dataset) -> Result<Uuid>` | Dataset registration |
| **Expression Engine** | Evaluate selector expression → boolean column | Row filtering |
| **Polars** | LazyFrame → DataFrame transformation | Data processing |

### External Contracts

- **OutputWriter implementations** (S16): Must handle various destination types (CSV, Parquet, database, etc.)
- **MetadataStore implementations**: Must validate Dataset schema and enforce uniqueness
- **Expression Engine**: Must compile selector into Polars `Expr`

---

## Performance Considerations

### Memory Footprint

| Scenario | Memory Usage | Optimization |
|----------|--------------|--------------|
| Full dataset output (no projection) | `rows × all_columns` | Use LazyFrame until .collect() |
| Projected output | `rows × selected_columns` | **Best**: Filter → Project → Collect |
| Large dataset with delete flag | `rows × columns` (includes deleted) | Early filtering reduces rows |

### Recommended Execution Order

1. **Filter by selector** (reduces row count)
2. **Filter by `_deleted` flag** (further reduces rows)
3. **Project columns** (reduces column count)
4. **Collect to DataFrame** (materialize once)
5. **Write**

This order minimizes memory usage by reducing data volume before materialization.

---

## Summary

The output operation is designed as a **pure function** that:
1. Reads from the working dataset (immutable)
2. Applies transformations (filter, project) lazily
3. Materializes output once at write time
4. Optionally registers output as a reusable Dataset

This design ensures memory efficiency, immutability, and composability within the pipeline.
