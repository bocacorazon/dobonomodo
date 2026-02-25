# Data Model: Delete Operation

**Feature**: Delete Operation  
**Date**: 2026-02-22  
**Status**: Phase 1 Design

## Overview

This document defines the data structures and relationships for implementing the delete operation in DobONoMoDo pipeline engine.

## Entities

### 1. DeleteOperation (Parameters)

**Description**: Configuration parameters for delete operation instances

**Schema**:

| Field | Type | Required | Default | Description |
|-------|------|----------|---------|-------------|
| `selector` | `Option<String>` | No | `None` | Boolean expression string or `{{NAME}}` reference to project selector. When `None`, deletes all active rows. |

**Rust Type**:
```rust
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
pub struct DeleteOperationParams {
    #[serde(default)]
    pub selector: Option<String>,
}
```

**YAML Example**:
```yaml
# Delete with selector
- seq: 3
  type: delete
  selector: "orders.amount = 0"

# Delete with named selector reference
- seq: 4
  type: delete
  selector: "{{invalid_orders}}"

# Delete all active rows (no selector)
- seq: 5
  type: delete
```

**Validation Rules**:
- If `selector` is provided and contains `{{NAME}}`, NAME must exist in `Project.selectors`
- If `selector` is a direct expression, it must parse as valid boolean expression
- Type-checked selector must return boolean (not arithmetic or aggregate)

**Relationships**:
- Embedded in `OperationInstance.parameters` for operations with `kind: OperationKind::Delete`
- References `Project.selectors` map when using `{{NAME}}` syntax

---

### 2. WorkingRow (Metadata Extensions)

**Description**: Row-level metadata for pipeline working dataset

**Schema** (existing + delete-related):

| Column | Type | Required | Mutability | Description |
|--------|------|----------|------------|-------------|
| `_row_id` | `Uuid` | Yes | Immutable | Unique row identifier (existing) |
| `_deleted` | `bool` | Yes | Mutable | Soft-delete flag; `true` when row logically deleted |
| `_modified_at` | `Timestamp` | Yes | Mutable | Last modification timestamp; updated when `_deleted` changes |
| `[business_columns]` | Various | Varies | Mutable | Business data columns from source dataset |

**Polars DataFrame Schema**:
```rust
// Expected DataFrame columns
Schema {
    "_row_id": DataType::Utf8,           // UUID as string
    "_deleted": DataType::Boolean,        // Soft-delete flag
    "_modified_at": DataType::Datetime,   // Modification timestamp
    // ... business columns from dataset schema
}
```

**Lifecycle**:
1. **Initialization**: When loading data, `_deleted` defaults to `false`, `_modified_at` set to load time
2. **Delete Operation**: Matching rows -> `_deleted = true`, `_modified_at = current_timestamp()`
3. **Subsequent Operations**: Non-deleted rows (`_deleted == false`) passed to next operation
4. **Output**: By default, only non-deleted rows written; `include_deleted: true` includes all

**State Transitions**:
```
[Active Row: _deleted = false]
         ->
    Delete matches
         ->
[Deleted Row: _deleted = true, _modified_at updated]
         ->
    (No undeletion in current scope)
```

---

### 3. OperationInstance (Context)

**Description**: Generic operation container (existing structure with delete context)

**Schema** (relevant fields):

| Field | Type | Required | Description |
|-------|------|----------|-------------|
| `order` | `u32` | Yes | Execution sequence number |
| `kind` | `OperationKind` | Yes | Operation type; `OperationKind::Delete` for delete ops |
| `alias` | `Option<String>` | No | Optional operation alias for tracing |
| `parameters` | `serde_json::Value` | Yes | Operation-specific parameters; holds `DeleteOperationParams` for delete |

**Rust Type** (existing):
```rust
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
pub struct OperationInstance {
    pub order: u32,
    #[serde(rename = "type")]
    pub kind: OperationKind,
    #[serde(default)]
    pub alias: Option<String>,
    #[serde(default)]
    pub parameters: serde_json::Value,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
#[serde(rename_all = "snake_case")]
pub enum OperationKind {
    Update,
    Aggregate,
    Append,
    Output,
    Delete,  // Already defined
}
```

**Delete-Specific Usage**:
```rust
let op = OperationInstance {
    order: 3,
    kind: OperationKind::Delete,
    alias: Some("remove_cancelled".to_string()),
    parameters: serde_json::json!({
        "selector": "orders.status = \"cancelled\""
    }),
};

// Deserialize parameters
let params: DeleteOperationParams = serde_json::from_value(op.parameters)?;
```

---

### 4. OutputOperation (Extended Parameters)

**Description**: Output operation parameters with delete visibility control

**Schema** (relevant fields):

| Field | Type | Required | Default | Description |
|-------|------|----------|---------|-------------|
| `destination` | `OutputDestination` | Yes | N/A | Target datasource + table |
| `include_deleted` | `bool` | No | `false` | Whether to include deleted rows in output |
| `selector` | `Option<String>` | No | `None` | Additional filter beyond delete status |

**Rust Type** (expected extension):
```rust
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
pub struct OutputOperationParams {
    pub destination: OutputDestination,
    #[serde(default)]
    pub include_deleted: bool,
    #[serde(default)]
    pub selector: Option<String>,
}
```

**YAML Example**:
```yaml
# Default output (excludes deleted rows)
- seq: 10
  type: output
  arguments:
    destination:
      datasource_id: "ds-warehouse"
      table: "processed_orders"
    # include_deleted defaults to false

# Archival output (includes deleted rows)
- seq: 11
  type: output
  arguments:
    destination:
      datasource_id: "ds-archive"
      table: "all_orders_with_deleted"
    include_deleted: true
```

**Behavior**:
- `include_deleted: false` (default): Output filters `_deleted == false` before write
- `include_deleted: true`: Output writes all rows regardless of `_deleted` status
- `selector` further filters rows if provided (applied after delete filtering)

---

## Relationships

### Entity Relationship Diagram

```
Project
|-- operations: Vec<OperationInstance>
|  `-- (kind: Delete) -> parameters: DeleteOperationParams
|     `-- selector: Option<String> --references--> Project.selectors[NAME]
|
`-- selectors: BTreeMap<String, String>

OperationInstance (Delete)
|-- executes on -> WorkingDataFrame
|  `-- columns: [_row_id, _deleted, _modified_at, business_cols]
|
`-- updates -> WorkingRow._deleted, WorkingRow._modified_at

OperationInstance (Output)
|-- parameters: OutputOperationParams
|  `-- include_deleted: bool --controls-> WorkingRow._deleted visibility
|
`-- writes -> OutputDestination (filtered or unfiltered)
```

### Key Constraints

1. **Delete Selector Validation**:
   - If `selector` contains `{{NAME}}`, NAME in `Project.selectors.keys()`
   - Selector expression must type-check as boolean

2. **Metadata Consistency**:
   - `_deleted` column MUST exist in all working DataFrames
   - `_modified_at` MUST update atomically with `_deleted` changes
   - `_row_id` remains immutable across all operations

3. **Operation Sequencing**:
   - Delete operation at position N affects operations at position N+1, N+2, ...
   - Non-output operations MUST filter `_deleted == false` automatically
   - Output operations MAY include deleted rows via `include_deleted: true`

4. **Idempotency**:
   - Deleting already-deleted rows is no-op (metadata unchanged)
   - Selector matching zero rows is valid (no changes)

---

## Edge Cases

### 1. Selector Matches Zero Rows

**Behavior**: No rows modified, `_deleted` and `_modified_at` remain unchanged

```rust
// Implementation handles gracefully
let updated_df = df.with_column(
    when(selector_expr)  // If selector never true
        .then(lit(true))
        .otherwise(col("_deleted"))  // All rows keep existing _deleted value
        .alias("_deleted")
);
// Result: DataFrame unchanged
```

### 2. Selector Matches All Active Rows

**Behavior**: All active rows marked deleted, subsequent operations receive empty dataset

```rust
// After delete matching all 1000 active rows:
// working_df.filter(col("_deleted").eq(false)) -> 0 rows
// Next operation (e.g., aggregate) processes empty dataset
// Output: Valid result with zero rows
```

### 3. Already-Deleted Rows in Selector Match

**Behavior**: Already-deleted rows are NOT modified again (metadata already set)

```rust
// Implementation via conditional update
when(selector_expr.and(col("_deleted").eq(lit(false))))  // Only update active rows
    .then(lit(true))
    .otherwise(col("_deleted"))
```

### 4. No Selector Provided

**Behavior**: Treated as "delete all active rows" (selector = `true`)

```rust
let selector_expr = match params.selector {
    Some(ref sel) if !sel.is_empty() => compile_selector(sel)?,
    _ => lit(true),  // Match all rows
};
```

### 5. Invalid Selector Expression

**Behavior**: Validation fails before execution, pipeline rejected

**Validation error example**:
```
Error: Invalid selector in operation 3 (delete)
  Selector: "orders.invalid_field = 5"
  Reason: Column 'invalid_field' not found in schema
```

---

## Schema Evolution

### Version Compatibility

**Current Version**: 1.0 (initial implementation)

**Future Extensions** (out of scope for this feature):
- Undo/undelete operation
- Soft-delete retention policies (auto-archive after N days)
- Cascading delete rules (delete related rows across joins)

**Breaking Changes Prohibited**:
- Removing `_deleted` column (existing pipelines depend on it)
- Changing `_deleted` type from boolean (breaks filter logic)
- Renaming metadata columns (breaks serialization)

**Non-Breaking Changes Allowed**:
- Adding new metadata columns (e.g., `_deleted_by`, `_deleted_reason`)
- Adding new operation parameters (with defaults)
- Extending `OutputOperationParams` with new flags

---

## Summary

**Core Entities**:
1. **DeleteOperationParams**: Simple struct with optional `selector` field
2. **WorkingRow Metadata**: `_deleted` boolean + `_modified_at` timestamp
3. **OperationInstance**: Container with `kind: Delete` and params
4. **OutputOperationParams**: Extended with `include_deleted` flag

**Key Relationships**:
- Delete operation -> Selector -> Project selectors map
- Delete operation -> Working DataFrame -> Row metadata updates
- Output operation -> `include_deleted` flag -> Row visibility filtering

**Validation Rules**:
- Selector must be valid boolean expression
- Metadata columns required in all DataFrames
- Operation sequencing enforced via `order` field
