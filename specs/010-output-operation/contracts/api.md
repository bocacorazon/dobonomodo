# Output Operation API Contract

**Feature**: 010-output-operation  
**Date**: 2026-02-23  
**Language**: Rust

---

## Module: `core::engine::ops::output`

Public API for executing output operations.

---

## Public Functions

### `execute_output`

Executes an output operation on the working dataset.

**Signature**:
```rust
pub fn execute_output(
    working_dataset: &LazyFrame,
    operation: &OutputOperation,
    output_writer: &dyn OutputWriter,
    metadata_store: Option<&dyn MetadataStore>,
) -> Result<OutputResult, OutputError>
```

**Parameters**:
- `working_dataset`: Reference to the current pipeline working dataset (LazyFrame)
- `operation`: Configuration for the output operation
- `output_writer`: Trait object for writing output to destination
- `metadata_store`: Optional metadata store for dataset registration (required if `register_as_dataset` is set)

**Returns**:
- `Ok(OutputResult)`: Execution succeeded, contains row count, columns, and optional dataset ID
- `Err(OutputError)`: Execution failed (see error types below)

**Behavior**:
1. Validates operation configuration (selector, columns, destination)
2. Applies selector filter (if present)
3. Applies deleted flag filter (if `include_deleted=false`)
4. Projects columns (if specified)
5. Collects LazyFrame to DataFrame
6. Writes DataFrame via `output_writer`
7. Optionally registers output as Dataset via `metadata_store`
8. Returns execution result with observability metrics

**MetadataStore Registration Requirements**:
- For `register_as_dataset`, the provided `MetadataStore` must support:
    - `get_dataset_by_name(&str) -> Result<Option<Dataset>>`
    - `register_dataset(Dataset) -> Result<Uuid>`
- Registration failures are warning-logged and remain non-fatal after a successful write.

**Errors**:
- `OutputError::InvalidSelector`: Selector expression is invalid or non-boolean
- `OutputError::ColumnProjectionError`: One or more specified columns don't exist
- `OutputError::MissingMetadataStore`: `register_as_dataset` set but `metadata_store` is None
- `OutputError::WriteFailed`: OutputWriter returned error
- `OutputError::RegistrationFailed`: Dataset registration failed (logged, not fatal)

**Example**:
```rust
let operation = OutputOperation {
    destination: OutputDestination::Table {
        datasource_id: datasource_id,
        table: "output_table".to_string(),
    },
    selector: Some(parse_expression("amount > 1000")?),
    columns: Some(vec!["id".to_string(), "amount".to_string()]),
    include_deleted: false,
    register_as_dataset: Some("high_value_transactions".to_string()),
};

let result = execute_output(
    &working_dataset,
    &operation,
    &csv_writer,
    Some(&metadata_store),
)?;

println!("Wrote {} rows", result.rows_written);
if let Some(dataset_id) = result.dataset_id {
    println!("Registered dataset: {}", dataset_id);
}
```

---

### `extract_schema`

Extracts schema information from a DataFrame for dataset registration.

**Signature**:
```rust
pub fn extract_schema(df: &DataFrame) -> Result<OutputSchema, OutputError>
```

**Parameters**:
- `df`: DataFrame to extract schema from

**Returns**:
- `Ok(OutputSchema)`: Schema with columns and temporal mode
- `Err(OutputError::EmptyDataFrame)`: DataFrame has no columns

**Behavior**:
- Iterates over DataFrame columns and builds ColumnDef list
- Detects temporal mode based on presence of `_period`, `_valid_from/_valid_to` columns
- Maps Polars DataType to project ColumnType enum

**Example**:
```rust
let schema = extract_schema(&output_df)?;
assert_eq!(schema.columns.len(), 4);
assert_eq!(schema.temporal_mode, TemporalMode::Period);
```

---

## Data Structures

### `OutputOperation`

Configuration for an output operation.

```rust
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
pub struct OutputOperation {
    /// Target destination for output data
    pub destination: OutputDestination,
    
    /// Optional row filter (boolean expression).
    ///
    /// Uses a Polars `Expr` directly (e.g., `col("amount").gt(lit(1000))`).
    /// The expression must evaluate to a boolean series.
    #[serde(default)]
    pub selector: Option<Expr>,
    
    /// Optional column projection (subset of columns to output)
    #[serde(default)]
    pub columns: Option<Vec<String>>,
    
    /// Whether to include rows with _deleted=true (default: false)
    #[serde(default)]
    pub include_deleted: bool,
    
    /// Optional dataset name for registration in metadata store
    #[serde(default)]
    pub register_as_dataset: Option<String>,
}
```

---

### `OutputDestination`

Specifies where to write output data.

```rust
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
#[serde(tag = "type", rename_all = "snake_case")]
pub enum OutputDestination {
    /// Reference to a DataSource and table name
    Table {
        datasource_id: Uuid,
        table: String,
    },
    
    /// Direct file/object location
    Location {
        path: String,
        format: OutputFormat,
    },
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
#[serde(rename_all = "lowercase")]
pub enum OutputFormat {
    Csv,
    Parquet,
    Json,
}
```

---

### `OutputResult`

Result of executing an output operation.

```rust
#[derive(Debug, Clone, PartialEq)]
pub struct OutputResult {
    /// Number of rows written to destination
    pub rows_written: usize,
    
    /// List of column names in output
    pub columns_written: Vec<String>,
    
    /// ID of registered dataset (if register_as_dataset was set)
    pub dataset_id: Option<Uuid>,
    
    /// Time taken to write data in milliseconds
    pub write_duration_ms: u64,
}
```

---

### `OutputSchema`

Schema extracted from output DataFrame.

```rust
#[derive(Debug, Clone, PartialEq)]
pub struct OutputSchema {
    /// Ordered list of columns in output
    pub columns: Vec<ColumnDef>,
    
    /// Temporal mode (inherited from working dataset)
    pub temporal_mode: TemporalMode,
}

#[derive(Debug, Clone, PartialEq)]
pub struct ColumnDef {
    /// Column name
    pub name: String,
    
    /// Data type
    pub data_type: ColumnType,
    
    /// Whether column can contain NULL values
    pub nullable: bool,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
#[serde(rename_all = "lowercase")]
pub enum TemporalMode {
    None,
    Period,
    Bitemporal,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
#[serde(rename_all = "lowercase")]
pub enum ColumnType {
    String,
    Integer,
    Decimal,
    Date,
    DateTime,
    Boolean,
    Uuid,
}
```

---

## Error Types

### `OutputError`

Errors that can occur during output operation execution.

```rust
#[derive(Debug, thiserror::Error)]
pub enum OutputError {
    /// Selector expression evaluation failed
    #[error("Selector evaluation failed: {0}")]
    SelectorError(String),
    
    /// One or more columns specified for projection don't exist
    #[error("Column projection failed: missing columns {missing:?}")]
    ColumnProjectionError {
        missing: Vec<String>,
    },
    
    /// Write operation failed
    #[error("Write operation failed: {0}")]
    WriteFailed(#[from] anyhow::Error),
    
    /// Dataset registration failed (non-fatal, logged as warning)
    #[error("Dataset registration failed: {0}")]
    RegistrationFailed(String),
    
    /// Invalid output schema (empty columns)
    #[error("Invalid output schema: {0}")]
    InvalidSchema(String),
    
    /// DataFrame has no columns
    #[error("DataFrame is empty (no columns)")]
    EmptyDataFrame,
    
    /// register_as_dataset set but no metadata_store provided
    #[error("Dataset registration requested but no MetadataStore provided")]
    MissingMetadataStore,
    
    /// Invalid destination configuration
    #[error("Invalid output destination: {0}")]
    InvalidDestination(String),
    
    /// Polars operation failed
    #[error("Polars error: {0}")]
    PolarsError(#[from] polars::error::PolarsError),
}
```

---

## Summary

This contract defines the complete public API for the output operation, including:
- Function signatures with clear parameter semantics
- Data structures with validation rules
- Error types with descriptive messages
- Behavioral contracts for immutability, filtering, and registration
- Performance contracts for memory efficiency
- Testing requirements for verification
