# Research: Output Operation

**Feature**: 010-output-operation  
**Date**: 2026-02-23  
**Status**: Complete

---

## Overview

This document consolidates research findings for implementing the `output` operation type. All NEEDS CLARIFICATION items from the Technical Context have been resolved.

---

## R1: Memory-Efficient Data Processing with Polars

### Decision
Use **Polars LazyFrame** with late materialization for memory-efficient processing of large datasets.

### Rationale
- LazyFrame defers computation until `.collect()`, enabling query optimization
- Predicate pushdown automatically optimizes filter operations
- Column projection reduces memory footprint before materialization
- Reference counting (Arc) makes LazyFrame clones cheap
- Aligns with existing `DataLoader` trait returning LazyFrame

### Implementation Pattern
```rust
pub fn execute_output(
    working_dataset: LazyFrame,      // Input (immutable)
    selector: Option<Expr>,          // Row filter
    columns: Option<Vec<String>>,    // Column projection
    include_deleted: bool,
    writer: &dyn OutputWriter,
    destination: &OutputDestination,
) -> Result<()> {
    // 1. Apply selector filter (if present)
    let mut output = working_dataset.clone();
    if let Some(filter_expr) = selector {
        output = output.filter(filter_expr);
    }
    
    // 2. Exclude deleted rows (default behavior)
    if !include_deleted {
        output = output.filter(col("_deleted").eq(lit(false)));
    }
    
    // 3. Project columns (if specified)
    if let Some(col_names) = columns {
        output = output.select(col_names.iter().map(|s| col(s)).collect());
    }
    
    // 4. Collect and write (ONLY materialize here)
    let df = output.collect()?;
    writer.write(&df, destination)?;
    
    Ok(())
}
```

### Performance Goals (Resolved)
- **Target**: Process datasets with millions of rows without OOM
- **Strategy**: Memory ∝ (rows × selected_columns), not all columns
- **Constraint**: Defer `.collect()` until write operation

### Alternatives Considered
- **Eager DataFrame processing**: Rejected due to high memory usage for large datasets
- **Manual chunking/streaming**: Rejected as Polars LazyFrame handles this automatically
- **Arrow RecordBatch**: Rejected as Polars provides higher-level API with same underlying format

---

## R2: Dataset Registration Pattern

### Decision
Register output as a **versioned Dataset entity** via MetadataStore, transactionally bound to successful write.

### Rationale
- Enables output reuse as input to other Projects
- Versioning provides immutable history and time-travel queries
- Atomic registration prevents orphaned metadata on write failure
- Aligns with existing Dataset entity model

### Implementation Pattern
```rust
pub fn register_as_dataset(
    output_schema: &TableRef,
    dataset_name: String,
    metadata_store: &dyn MetadataStore,
) -> Result<Uuid> {
    // 1. Check if dataset already exists
    let existing = metadata_store.get_dataset_by_name(&dataset_name)?;
    
    // 2. Create new dataset or new version
    let dataset = if let Some(existing_ds) = existing {
        // Increment version for existing dataset
        Dataset {
            id: Uuid::new_v4(),  // New version gets new ID
            name: dataset_name.clone(),
            version: existing_ds.version + 1,
            status: DatasetStatus::Active,
            main_table: output_schema.clone(),
            created_at: Utc::now(),
            ..existing_ds  // Inherit owner, resolver_id, etc.
        }
    } else {
        // Create new dataset (version 1)
        Dataset {
            id: Uuid::new_v4(),
            name: dataset_name.clone(),
            version: 1,
            status: DatasetStatus::Active,
            main_table: output_schema.clone(),
            created_at: Utc::now(),
            updated_at: Utc::now(),
            ..Default::default()
        }
    };
    
    // 3. Register (validates schema, uniqueness, etc.)
    metadata_store.register_dataset(dataset.clone())?;
    
    Ok(dataset.id)
}
```

### Required Validations
- Dataset name must be non-empty and unique per owner
- Output schema must have at least one column
- Column types must be valid (align with ColumnType enum)
- Temporal mode must be specified if using period/bitemporal columns

### Error Handling
- `WriteFailed`: Output write operation failed
- `RegistrationFailed`: Dataset registration failed after successful write (log orphan warning)
- `InvalidSchema`: Output schema validation failed
- `DuplicateDataset`: Name collision with incompatible existing dataset

### Transaction Strategy
```
1. Validate output schema (early failure)
2. Execute write operation via OutputWriter
3. If write succeeds: register dataset
4. If registration fails: log error but don't fail the operation
   (write succeeded, metadata update is best-effort)
```

### Alternatives Considered
- **Pre-registration**: Rejected as it creates orphan metadata on write failure
- **Two-phase commit**: Rejected as OutputWriter doesn't support rollback
- **Best-effort post-write**: **SELECTED** — write is the critical operation

---

## R3: Error Handling Strategy

### Decision
Use **thiserror** for operation-specific errors, **anyhow** for context propagation.

### Rationale
- Already used in codebase (`anyhow::Result` in io_traits.rs)
- thiserror provides structured error types for library code
- anyhow::Error provides good context chains for debugging

### Error Types
```rust
#[derive(Debug, thiserror::Error)]
pub enum OutputError {
    #[error("Selector evaluation failed: {0}")]
    SelectorError(String),
    
    #[error("Column projection failed: missing columns {missing:?}")]
    ColumnProjectionError { missing: Vec<String> },
    
    #[error("Write operation failed: {0}")]
    WriteFailed(#[from] anyhow::Error),
    
    #[error("Dataset registration failed: {0}")]
    RegistrationFailed(String),
    
    #[error("Invalid output schema: {0}")]
    InvalidSchema(String),
}
```

### Alternatives Considered
- **anyhow only**: Rejected as structured errors improve error handling
- **std::error only**: Rejected as context chains are valuable for debugging

---

## R4: Testing Strategy

### Decision
Implement **3-tier test coverage**: unit, integration, contract tests.

### Test Scenarios

#### Unit Tests (`tests/unit/output_op_test.rs`)
1. **Selector filtering**: Apply boolean expression, verify filtered rows
2. **Column projection**: Project subset of columns, verify schema
3. **Delete flag handling**: Verify `include_deleted=false` excludes `_deleted=true` rows
4. **Delete flag inclusion**: Verify `include_deleted=true` includes deleted rows
5. **Schema extraction**: Extract TableRef from output DataFrame
6. **Error cases**: Invalid selector, missing columns, write failure

#### Integration Tests (`tests/integration/output_integration_test.rs`)
1. **End-to-end output**: Load → filter → project → write via mock OutputWriter
2. **Dataset registration**: Verify MetadataStore receives correct Dataset entity
3. **Mid-pipeline output**: Verify working dataset unchanged after output
4. **Multiple outputs**: Execute output operation twice in pipeline

#### Contract Tests (`tests/contract/ts07_column_projection.rs`)
Implement **test scenario TS-07** from sample-datasets.md:
- Load GL transactions dataset
- Execute output operation with column projection: `[journal_id, account_code, amount_local, amount_reporting]`
- Verify output contains only 4 columns per row
- Verify all rows present (10 rows from sample data)
- Verify exact column values match expectations

### Test Infrastructure
- Use existing `cargo test` framework
- Mock `OutputWriter` and `MetadataStore` for isolated testing
- Use `test-resolver` crate for test fixtures
- Leverage Polars' in-memory DataFrame for test data

### Alternatives Considered
- **Only contract tests**: Rejected as unit tests catch logic errors faster
- **Only integration tests**: Rejected as unit tests provide better failure isolation

---

## R5: Immutability & Working Dataset Preservation

### Decision
Output operation is **read-only** — it never modifies the working dataset.

### Rationale
- Aligns with business rule BR-012: output can appear mid-pipeline
- Enables checkpointing without side effects
- Simplifies reasoning about pipeline execution
- LazyFrame cloning is cheap (Arc-based reference counting)

### Implementation
```rust
pub fn apply_output_operation(
    working_dataset: &LazyFrame,  // Immutable reference
    // ... other params
) -> Result<()> {
    let output_frame = working_dataset.clone();  // Cheap clone (Arc)
    // ... apply filters, projections
    // ... write
    // NOTE: working_dataset is never mutated
    Ok(())
}
```

### Verification
Integration test verifies working dataset unchanged:
```rust
#[test]
fn output_preserves_working_dataset() {
    let original = create_test_dataset();
    let original_hash = hash_dataframe(&original);
    
    execute_output_operation(&original, /* ... */)?;
    
    let after_hash = hash_dataframe(&original);
    assert_eq!(original_hash, after_hash, "Working dataset was mutated");
}
```

---

## Summary of Resolved Clarifications

| Item | Resolution |
|------|------------|
| Performance goals | Memory-efficient streaming via LazyFrame; target: millions of rows |
| Dataset registration | Versioned entities via MetadataStore; transactional with write |
| Error handling | thiserror for structured errors, anyhow for context |
| Test coverage | 3-tier: unit, integration, contract (TS-07) |
| Immutability | Output operation is read-only; working dataset never modified |

---

## References

- **Polars Documentation**: Lazy API, predicate pushdown, column selection
- **Existing Code**: `engine/io_traits.rs` (OutputWriter), `model/dataset.rs` (Dataset entity)
- **Entity Definition**: `docs/entities/operation.md` (BR-011, BR-012, BR-013)
- **Test Scenario**: `docs/architecture/sample-datasets.md` (TS-07)
