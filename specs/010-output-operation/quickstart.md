# Quickstart: Output Operation

**Feature**: 010-output-operation  
**Date**: 2026-02-23  
**Audience**: Developers implementing or using the output operation

---

## Overview

The **output operation** writes data from the pipeline's working dataset to an external destination. It supports:
- **Row filtering** via selector expressions
- **Column projection** to output a subset of columns
- **Deleted row handling** (exclude by default, include on demand)
- **Dataset registration** to make output reusable as input to other projects

---

## Quick Example

```rust
use core::engine::ops::output::{execute_output, OutputOperation, OutputDestination};
use core::engine::io_traits::OutputWriter;
use core::model::metadata_store::MetadataStore;
use polars::prelude::*;

// Assume we have a working dataset (LazyFrame) from previous operations
let working_dataset: LazyFrame = /* ... */;

// Configure output operation
let operation = OutputOperation {
    destination: OutputDestination::Table {
        datasource_id: uuid!("12345678-1234-1234-1234-123456789012"),
        table: "monthly_summary".to_string(),
    },
    selector: None,  // Output all rows
    columns: Some(vec![
        "account_code".to_string(),
        "amount_local".to_string(),
        "amount_reporting".to_string(),
    ]),
    include_deleted: false,  // Exclude deleted rows
    register_as_dataset: Some("monthly_gl_summary".to_string()),
};

// Execute output operation
let result = execute_output(
    &working_dataset,
    &operation,
    &my_output_writer,  // Implements OutputWriter trait
    Some(&my_metadata_store),  // Implements MetadataStore trait
)?;

println!("✓ Wrote {} rows to {}", 
    result.rows_written, 
    operation.destination.table
);
if let Some(dataset_id) = result.dataset_id {
    println!("✓ Registered dataset: {}", dataset_id);
}
```

---

## Common Use Cases

### 1. Simple Output (All Rows, All Columns)

Write the entire working dataset to a destination:

```rust
let operation = OutputOperation {
    destination: OutputDestination::Location {
        path: "/data/output/results.parquet".to_string(),
        format: OutputFormat::Parquet,
    },
    selector: None,
    columns: None,
    include_deleted: false,
    register_as_dataset: None,
};

execute_output(&working_dataset, &operation, &writer, None)?;
```

---

### 2. Filtered Output (Specific Rows)

Output only rows matching a condition:

```rust
use core::model::expression::Expression;

let operation = OutputOperation {
    destination: /* ... */,
    selector: Some(Expression::parse("amount > 10000 AND region = 'EMEA'")?),
    columns: None,
    include_deleted: false,
    register_as_dataset: None,
};

execute_output(&working_dataset, &operation, &writer, None)?;
```

---

### 3. Column Projection

Output only specific columns:

```rust
let operation = OutputOperation {
    destination: /* ... */,
    selector: None,
    columns: Some(vec![
        "journal_id".to_string(),
        "account_code".to_string(),
        "amount_local".to_string(),
    ]),
    include_deleted: false,
    register_as_dataset: None,
};

execute_output(&working_dataset, &operation, &writer, None)?;
```

**Result**: Output contains only 3 columns (plus system columns if not excluded).

---

### 4. Include Deleted Rows

Include soft-deleted rows in output:

```rust
let operation = OutputOperation {
    destination: /* ... */,
    selector: None,
    columns: None,
    include_deleted: true,  // Include rows with _deleted=true
    register_as_dataset: None,
};

execute_output(&working_dataset, &operation, &writer, None)?;
```

**Use case**: Audit logs, deleted record archives.

---

### 5. Register Output as Dataset

Make output available as input to other projects:

```rust
let operation = OutputOperation {
    destination: /* ... */,
    selector: Some(Expression::parse("status = 'active'")?),
    columns: None,
    include_deleted: false,
    register_as_dataset: Some("active_records_snapshot".to_string()),
};

let result = execute_output(
    &working_dataset,
    &operation,
    &writer,
    Some(&metadata_store),  // REQUIRED when register_as_dataset is set
)?;

// Use the registered dataset ID
let dataset_id = result.dataset_id.unwrap();
println!("Dataset registered: {}", dataset_id);
```

**Note**: `metadata_store` parameter MUST be `Some(...)` when `register_as_dataset` is set.

---

### 6. Mid-Pipeline Checkpoint

Output intermediate results without affecting the pipeline:

```rust
// Step 1: Update operation
let updated_dataset = execute_update(&working_dataset, &update_op)?;

// Step 2: Checkpoint (output intermediate results)
execute_output(&updated_dataset, &checkpoint_op, &writer, None)?;
// ↑ working dataset unchanged

// Step 3: Continue pipeline with aggregate
let aggregated_dataset = execute_aggregate(&updated_dataset, &agg_op)?;

// Step 4: Final output
execute_output(&aggregated_dataset, &final_op, &writer, None)?;
```

**Key**: Output operation does NOT modify the working dataset.

---

## Configuration Reference

### OutputOperation Fields

| Field | Type | Required | Default | Description |
|-------|------|----------|---------|-------------|
| `destination` | `OutputDestination` | Yes | — | Where to write output |
| `selector` | `Option<Expression>` | No | `None` | Row filter (boolean expression) |
| `columns` | `Option<Vec<String>>` | No | `None` | Column projection (subset to output) |
| `include_deleted` | `bool` | No | `false` | Include rows with `_deleted=true` |
| `register_as_dataset` | `Option<String>` | No | `None` | Dataset name for registration |

### OutputDestination Variants

**Table Reference**:
```rust
OutputDestination::Table {
    datasource_id: Uuid,  // Reference to DataSource entity
    table: String,        // Table name within datasource
}
```

**Direct Location**:
```rust
OutputDestination::Location {
    path: String,         // File/object path
    format: OutputFormat, // Csv | Parquet | Json
}
```

---

## Error Handling

### Common Errors

| Error | Cause | Solution |
|-------|-------|----------|
| `ColumnProjectionError` | Column in `columns` doesn't exist in working dataset | Check column names; use `working_dataset.schema()` to list available columns |
| `MissingMetadataStore` | `register_as_dataset` set but `metadata_store` is `None` | Pass `Some(&metadata_store)` when calling `execute_output` |
| `WriteFailed` | OutputWriter returned error | Check destination accessibility, permissions, disk space |
| `SelectorError` | Invalid or non-boolean selector expression | Validate expression syntax and return type |

### Example Error Handling

```rust
match execute_output(&working_dataset, &operation, &writer, metadata_store) {
    Ok(result) => {
        println!("Success: {} rows written", result.rows_written);
    }
    Err(OutputError::ColumnProjectionError { missing }) => {
        eprintln!("Missing columns: {:?}", missing);
        eprintln!("Available columns: {:?}", working_dataset.schema().column_names());
    }
    Err(OutputError::WriteFailed(e)) => {
        eprintln!("Write failed: {}", e);
        // Handle write failure (retry, log, etc.)
    }
    Err(e) => {
        eprintln!("Output operation failed: {}", e);
    }
}
```

---

## Performance Tips

### 1. Filter Before Projecting

Apply selector filter BEFORE column projection to reduce data volume:

```rust
// Good: Filter reduces rows, then projection reduces columns
let operation = OutputOperation {
    selector: Some(Expression::parse("amount > 1000")?),  // ← Filter first
    columns: Some(vec!["id".to_string(), "amount".to_string()]),  // ← Then project
    // ...
};
```

**Why**: Memory usage ∝ (filtered_rows × selected_columns), not (all_rows × all_columns).

---

### 2. Use Column Projection for Large Schemas

If working dataset has 50 columns but you only need 5:

```rust
let operation = OutputOperation {
    columns: Some(vec![
        "id".to_string(),
        "amount".to_string(),
        "date".to_string(),
        "status".to_string(),
        "region".to_string(),
    ]),
    // ...
};
```

**Impact**: Reduces memory usage by 90% (5/50 columns).

---

### 3. Exclude Deleted Rows by Default

Unless you need deleted rows, keep `include_deleted: false`:

```rust
let operation = OutputOperation {
    include_deleted: false,  // Excludes _deleted=true rows (default)
    // ...
};
```

**Why**: Reduces output size and improves write performance.

---

## Testing

### Unit Test Example

```rust
#[cfg(test)]
mod tests {
    use super::*;
    
    #[test]
    fn test_column_projection() {
        let df = df! {
            "id" => [1, 2, 3],
            "name" => ["Alice", "Bob", "Charlie"],
            "amount" => [100, 200, 300],
        }.unwrap();
        let working_dataset = df.lazy();
        
        let operation = OutputOperation {
            destination: OutputDestination::Location {
                path: "/tmp/test.csv".to_string(),
                format: OutputFormat::Csv,
            },
            columns: Some(vec!["id".to_string(), "amount".to_string()]),
            include_deleted: false,
            selector: None,
            register_as_dataset: None,
        };
        
        let mock_writer = MockOutputWriter::new();
        let result = execute_output(&working_dataset, &operation, &mock_writer, None).unwrap();
        
        assert_eq!(result.rows_written, 3);
        assert_eq!(result.columns_written, vec!["id", "amount"]);
    }
}
```

---

## Integration with Existing Code

### Using Existing Traits

The output operation uses existing traits from the codebase:

**OutputWriter** (from `core::engine::io_traits`):
```rust
pub trait OutputWriter {
    fn write(&self, frame: &DataFrame, destination: &OutputDestination) -> Result<()>;
}
```

**MetadataStore** (from `core::model::metadata_store`):
```rust
pub trait MetadataStore {
    fn register_dataset(&self, dataset: Dataset) -> Result<Uuid>;
    fn get_dataset_by_name(&self, name: &str) -> Result<Option<Dataset>>;
}
```

### Example Implementation (OutputWriter)

```rust
struct CsvOutputWriter;

impl OutputWriter for CsvOutputWriter {
    fn write(&self, frame: &DataFrame, destination: &OutputDestination) -> Result<()> {
        match destination {
            OutputDestination::Location { path, format: OutputFormat::Csv } => {
                let mut file = std::fs::File::create(path)?;
                CsvWriter::new(&mut file).finish(frame)?;
                Ok(())
            }
            _ => Err(anyhow!("Unsupported destination")),
        }
    }
}
```

---

## Next Steps

1. **Implement OutputWriter**: Create concrete implementations for your target formats (CSV, Parquet, database, etc.)
2. **Implement MetadataStore**: If using dataset registration, implement the MetadataStore trait
3. **Write Tests**: Use the contract test (TS-07) as a starting point
4. **Integrate into Pipeline**: Add output operations to your project YAML definitions

---

## Related Documentation

- **API Contract**: `/specs/010-output-operation/contracts/api.md`
- **Data Model**: `/specs/010-output-operation/data-model.md`
- **Research**: `/specs/010-output-operation/research.md`
- **Entity Definition**: `/docs/entities/operation.md` (BR-011, BR-012, BR-013)
- **Test Scenario TS-07**: `/docs/architecture/sample-datasets.md`

---

## FAQ

### Q: Can I use multiple output operations in the same pipeline?

**A**: Yes! Output operations can appear anywhere in the pipeline and don't modify the working dataset.

```rust
// Checkpoint after update
execute_output(&updated_dataset, &checkpoint_op, &writer, None)?;

// Final output after aggregate
execute_output(&final_dataset, &final_op, &writer, None)?;
```

---

### Q: What happens if write succeeds but registration fails?

**A**: The output write is successful (data is persisted). Registration failure is logged as a warning but does NOT fail the operation. You can manually register the dataset later.

---

### Q: How do I output system columns (_row_id, _deleted, _period)?

**A**: Include them in the `columns` parameter:

```rust
let operation = OutputOperation {
    columns: Some(vec![
        "_row_id".to_string(),
        "_deleted".to_string(),
        "id".to_string(),
        "amount".to_string(),
    ]),
    // ...
};
```

---

### Q: Can I use named selectors ({{SELECTOR_NAME}})?

**A**: Yes, if the selector expression is interpolated at the Project level before being passed to the operation. The `execute_output` function accepts a pre-evaluated `Expression`.

---

### Q: What's the difference between `include_deleted: false` and `selector: "_deleted != true"`?

**A**: Functionally equivalent, but `include_deleted: false` is clearer intent. Both exclude deleted rows. Use `include_deleted` for readability.

---

## Support

For issues or questions:
1. Check test scenarios in `/docs/architecture/sample-datasets.md`
2. Review API contract in `/specs/010-output-operation/contracts/api.md`
3. Run contract test TS-07 to validate implementation
