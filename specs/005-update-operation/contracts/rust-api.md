# Update Operation: Rust API Contract

**Feature**: Update Operation (S04)  
**Date**: 2025-02-22  
**Language**: Rust  
**Module**: `dobo_core::engine::ops::update`

---

## Public API

### Function: `execute_update`

**Purpose**: Execute an update operation on a LazyFrame, applying selector-based filtering and assignment expressions.

**Signature**:

```rust
pub fn execute_update(
    context: &UpdateExecutionContext,
    operation: &UpdateOperation,
) -> Result<LazyFrame, anyhow::Error>
```

**Parameters**:

- `context: &UpdateExecutionContext` - Execution context containing:
  - `working_dataset: LazyFrame` - Input dataset
  - `selectors: HashMap<String, String>` - Named selectors from Project
  - `run_timestamp: DateTime<Utc>` - Timestamp for `_updated_at`

- `operation: &UpdateOperation` - Update operation definition containing:
  - `selector: Option<String>` - Optional row filter (with `{{NAME}}` support)
  - `assignments: Vec<Assignment>` - Column assignments to apply

**Returns**:

- `Ok(LazyFrame)` - Updated LazyFrame with assignments applied and `_updated_at` set
- `Err(anyhow::Error)` - Error with context if:
  - Named selector `{{NAME}}` not found in selectors map
  - Selector expression fails to compile
  - Assignment expression fails to compile
  - Column reference undefined (propagated from Polars)
  - Type mismatch in assignment (propagated from Polars)

**Behavior**:

1. Resolve named selectors: If `selector` contains `{{NAME}}`, replace with expression from `context.selectors`
2. Compile selector: Parse resolved selector to Polars `Expr` (or use default: all non-deleted rows)
3. Filter rows: Apply selector `Expr` to `context.working_dataset`
4. Compile assignments: Parse each `Assignment.expression` to Polars `Expr`
5. Apply assignments: Use `.with_columns()` to apply all assignment Exprs
6. Update `_updated_at`: Set to `context.run_timestamp` for modified rows
7. Merge rows: Union updated rows with non-matching rows (unchanged)
8. Return updated `LazyFrame`

**Example Usage**:

```rust
use dobo_core::engine::ops::update::{execute_update, UpdateOperation, UpdateExecutionContext, Assignment};
use polars::prelude::*;
use chrono::Utc;
use std::collections::HashMap;

let context = UpdateExecutionContext {
    working_dataset: df![ // example LazyFrame
        "id" => [1, 2, 3],
        "status" => ["active", "active", "inactive"],
        "_updated_at" => [timestamp1, timestamp2, timestamp3],
    ]?.lazy(),
    selectors: HashMap::from([
        ("active_rows".to_string(), "status = \"active\"".to_string()),
    ]),
    run_timestamp: Utc::now(),
};

let operation = UpdateOperation {
    selector: Some("{{active_rows}}".to_string()),
    assignments: vec![
        Assignment {
            column: "status".to_string(),
            expression: "\"processed\"".to_string(),
        },
    ],
};

let result = execute_update(&context, &operation)?;
// result: LazyFrame with rows 1, 2 having status = "processed", _updated_at = run_timestamp
//         row 3 unchanged
```

---

### Struct: `UpdateOperation`

**Purpose**: Data structure representing an update operation's configuration.

**Definition**:

```rust
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
pub struct UpdateOperation {
    /// Optional row filter expression (supports {{NAME}} interpolation)
    /// If None, applies to all non-deleted rows
    #[serde(default)]
    pub selector: Option<String>,
    
    /// List of column assignments to apply
    pub assignments: Vec<Assignment>,
}
```

**Validation**:

- `assignments` must be non-empty (validated at construction or execution)

**Deserialization Example**:

```yaml
# YAML (from Operation.arguments)
selector: "{{active_orders}}"
assignments:
  - column: status
    expression: "\"processed\""
  - column: discount
    expression: "amount * 0.1"
```

```rust
// Rust deserialization
let args: UpdateOperation = serde_yaml::from_str(yaml_str)?;
```

---

### Struct: `Assignment`

**Purpose**: Represents a single column assignment within an update operation.

**Definition**:

```rust
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
pub struct Assignment {
    /// Target column name (existing or new)
    pub column: String,
    
    /// Value expression to compile and apply
    pub expression: String,
}
```

**Validation**:

- `column` must be non-empty and valid identifier (regex: `^[a-zA-Z_][a-zA-Z0-9_]*$`)
- `expression` must be non-empty and parseable

---

### Struct: `UpdateExecutionContext`

**Purpose**: Runtime context passed to the update operation executor.

**Definition**:

```rust
#[derive(Debug, Clone)]
pub struct UpdateExecutionContext {
    /// Input dataset (current working dataset)
    pub working_dataset: LazyFrame,
    
    /// Named selectors from Project for {{NAME}} interpolation
    pub selectors: HashMap<String, String>,
    
    /// Run timestamp for _updated_at system column
    pub run_timestamp: chrono::DateTime<chrono::Utc>,
}
```

**Usage**:

Created by the pipeline executor before invoking `execute_update()`.

---

## Helper Functions (Internal)

### Function: `resolve_selector`

**Purpose**: Replace `{{NAME}}` placeholders in selector with expressions from selectors map.

**Signature**:

```rust
fn resolve_selector(
    selector: &str,
    selectors: &HashMap<String, String>,
) -> Result<String, anyhow::Error>
```

**Parameters**:

- `selector: &str` - Selector string (may contain `{{NAME}}`)
- `selectors: &HashMap<String, String>` - Named selectors from Project

**Returns**:

- `Ok(String)` - Resolved selector expression
- `Err(anyhow::Error)` - If `{{NAME}}` not found in selectors map

**Behavior**:

1. Check if selector contains `{{` and `}}`
2. If yes:
   a. Extract name between `{{` and `}}`
   b. Lookup name in selectors map
   c. If found: return mapped expression
   d. If not found: return error
3. If no: return selector as-is

**Example**:

```rust
let selectors = HashMap::from([("active", "status = \"active\"")]);
let resolved = resolve_selector("{{active}}", &selectors)?;
// resolved = "status = \"active\""
```

---

### Function: `compile_selector`

**Purpose**: Compile a selector expression string to a Polars `Expr`.

**Signature**:

```rust
fn compile_selector(
    selector_expr: &str,
) -> Result<Expr, anyhow::Error>
```

**Parameters**:

- `selector_expr: &str` - Resolved selector expression (no `{{NAME}}`)

**Returns**:

- `Ok(Expr)` - Compiled Polars expression
- `Err(anyhow::Error)` - If expression fails to parse

**Behavior**:

1. Parse expression string using DSL parser (S01 dependency)
2. Convert parsed AST/IR to Polars `Expr`
3. Return compiled expression

**Note**: Implementation depends on S01 (DSL Parser) API.

---

### Function: `compile_assignments`

**Purpose**: Compile a list of assignments to Polars `Expr` list.

**Signature**:

```rust
fn compile_assignments(
    assignments: &[Assignment],
) -> Result<Vec<Expr>, anyhow::Error>
```

**Parameters**:

- `assignments: &[Assignment]` - List of column assignments

**Returns**:

- `Ok(Vec<Expr>)` - List of compiled Polars expressions (with `.alias(column)`)
- `Err(anyhow::Error)` - If any expression fails to parse

**Behavior**:

1. For each assignment:
   a. Parse `expression` string to Polars `Expr`
   b. Apply `.alias(column)` to set target column name
2. Collect all Exprs into Vec
3. Return compiled expressions

**Example**:

```rust
let assignments = vec![
    Assignment { column: "status".to_string(), expression: "\"done\"".to_string() },
    Assignment { column: "discount".to_string(), expression: "amount * 0.1".to_string() },
];

let exprs = compile_assignments(&assignments)?;
// exprs = [lit("done").alias("status"), (col("amount") * lit(0.1)).alias("discount")]
```

---

## Error Types

All functions return `Result<T, anyhow::Error>` for flexibility and context propagation.

**Common Error Messages**:

| Scenario | Error Message |
|----------|---------------|
| Empty assignments | `"Update operation requires at least one assignment"` |
| Undefined selector name | `"Selector '{{NAME}}' not defined in Project"` |
| Selector parse failure | `"Failed to compile selector expression: {err}"` |
| Assignment parse failure | `"Failed to compile assignment for column '{column}': {err}"` |
| Polars column not found | `"Column '{name}' not found in working dataset"` (from Polars) |
| Polars type mismatch | `"Type mismatch in assignment to '{column}'"` (from Polars) |

**Error Handling Pattern**:

```rust
execute_update(&context, &operation)
    .context("Failed to execute update operation")?;
```

---

## Testing Contract

### Unit Test Requirements

1. **Selector Resolution**:
   - Test valid `{{NAME}}` interpolation
   - Test undefined `{{NAME}}` returns error
   - Test selector without interpolation passes through

2. **Assignment Execution**:
   - Test single assignment updates matching rows
   - Test multiple assignments apply in batch
   - Test new column creation
   - Test existing column modification

3. **System Column Update**:
   - Test `_updated_at` set to run timestamp for modified rows
   - Test `_updated_at` unchanged for non-matching rows

4. **Row Filtering**:
   - Test selector filters correct rows
   - Test no selector applies to all non-deleted rows
   - Test non-matching rows pass through unchanged

5. **Error Cases**:
   - Test empty assignments returns error
   - Test invalid selector expression returns error
   - Test invalid assignment expression returns error

### Integration Test Requirements (via Test Harness)

1. **TS-03**: FX conversion scenario (without joins)
2. **TS-08**: Named selector interpolation scenario

---

## Dependencies

- **S01 (DSL Parser)**: Expression parsing (selector and assignment expressions)
- **Polars 0.46**: LazyFrame, Expr API
- **chrono**: DateTime for `_updated_at`
- **anyhow**: Error handling
- **serde**: Serialization/deserialization

---

## Version

**API Version**: 1.0.0 (initial implementation)  
**Stability**: Unstable (pre-1.0 project)
