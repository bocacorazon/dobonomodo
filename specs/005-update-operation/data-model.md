# Data Model: Update Operation

**Feature**: Update Operation (S04)  
**Date**: 2025-02-22  
**Status**: Phase 1 Complete

## Overview

This document defines the data model for the `update` operation implementation, including entities, relationships, validation rules, and state transitions.

---

## Entities

### 1. UpdateOperation

**Description**: Runtime representation of an update operation that applies assignments to rows matching a selector.

**Fields**:

| Field | Type | Required | Validation | Description |
|-------|------|----------|------------|-------------|
| `selector` | `Option<String>` | No | Valid boolean expression or `{{NAME}}` reference | Row filter; None means all non-deleted rows |
| `assignments` | `Vec<Assignment>` | Yes | Non-empty | List of column assignments to apply |

**Lifecycle**: 
- Created from deserialized Operation YAML/JSON
- Validated before execution
- Executed once per Run

**Relationships**:
- Belongs to an `Operation` instance
- References `Project.selectors` map (for named selector resolution)
- Produces a modified `LazyFrame` (transient, not persisted)

---

### 2. Assignment

**Description**: A single column assignment within an update operation.

**Fields**:

| Field | Type | Required | Validation | Description |
|-------|------|----------|------------|-------------|
| `column` | `String` | Yes | Non-empty, valid column name | Target column (existing or new) |
| `expression` | `String` | Yes | Non-empty, valid DSL expression | Value expression to compile |

**Validation Rules**:
- `column` must match regex `^[a-zA-Z_][a-zA-Z0-9_]*$` (standard identifier)
- `expression` must parse successfully to a Polars `Expr`
- Expression column references must resolve to working dataset columns (validated at compile time)

**Relationships**:
- Embedded in `UpdateOperation`
- Expression may reference columns from the working dataset

---

### 3. UpdateExecutionContext

**Description**: Runtime context provided to the update operation executor.

**Fields**:

| Field | Type | Required | Description |
|-------|------|----------|-------------|
| `working_dataset` | `LazyFrame` | Yes | Current state of the dataset (input to update) |
| `selectors` | `HashMap<String, String>` | Yes | Named selectors from Project |
| `run_timestamp` | `DateTime<Utc>` | Yes | Timestamp for `_updated_at` system column |

**Lifecycle**: 
- Created by the pipeline executor before invoking update operation
- Passed as immutable reference to update executor
- Consumed to produce output LazyFrame

**Relationships**:
- Aggregates data from `Run`, `Project`, and previous operation outputs
- Provided to `execute_update()` function

---

## State Transitions

### UpdateOperation Execution Flow

```
[Input: LazyFrame] 
    → [Resolve Selector] 
    → [Compile Selector to Expr] 
    → [Filter Rows] 
    → [Compile Assignments to Exprs] 
    → [Apply with_columns()] 
    → [Update _updated_at] 
    → [Union with Non-Matching Rows] 
    → [Output: LazyFrame]
```

**State Definitions**:

1. **Input**: LazyFrame with working dataset (all rows, including non-deleted)
2. **Resolve Selector**: If selector contains `{{NAME}}`, replace with expression from selectors map
3. **Compile Selector**: Parse resolved selector string to Polars `Expr`
4. **Filter Rows**: Apply selector Expr to identify matching rows
5. **Compile Assignments**: Parse each assignment expression to Polars `Expr`
6. **Apply with_columns()**: Execute all assignment Exprs on filtered rows
7. **Update _updated_at**: Add system column update with run timestamp
8. **Union with Non-Matching Rows**: Merge updated rows with unchanged rows
9. **Output**: Updated LazyFrame (input to next operation)

**Error States**:
- **Selector Resolution Failed**: Named selector `{{NAME}}` not found in selectors map
- **Selector Compilation Failed**: Invalid expression syntax in selector
- **Assignment Compilation Failed**: Invalid expression syntax in assignment
- **Column Reference Failed**: Expression references undefined column
- **Type Mismatch**: Assignment expression type incompatible with target column

---

## Validation Rules

### Compile-Time Validation

Performed before execution (during operation setup):

| Rule ID | Rule | Error Type |
|---------|------|------------|
| VR-001 | Assignments list MUST NOT be empty | `anyhow::Error("Update operation requires at least one assignment")` |
| VR-002 | Assignment `column` MUST be valid identifier | `anyhow::Error("Invalid column name: '{name}'")` |
| VR-003 | Named selector `{{NAME}}` MUST exist in selectors map | `anyhow::Error("Selector '{name}' not defined in Project")` |
| VR-004 | Selector expression MUST parse successfully | `anyhow::Error("Invalid selector expression: {err}")` |
| VR-005 | Assignment expression MUST parse successfully | `anyhow::Error("Invalid assignment expression for '{column}': {err}")` |

### Runtime Validation

Performed during execution (deferred to Polars):

| Rule ID | Rule | Error Type |
|---------|------|------------|
| VR-006 | Expression column references MUST resolve | `PolarsError::ColumnNotFound` |
| VR-007 | Expression types MUST be compatible | `PolarsError::SchemaMismatch` |
| VR-008 | Working dataset MUST contain `_updated_at` column | `PolarsError::ColumnNotFound` |

---

## System Column Behavior

### _updated_at

- **Type**: `DateTime<Utc>` (mapped to Polars `Datetime` dtype)
- **Update Strategy**: Set to `run_timestamp` for all modified rows
- **Non-Modified Rows**: Retain original value (pass through unchanged)
- **New Rows**: N/A (update doesn't create new rows)

### _deleted

- **Behavior**: Update operation respects `_deleted` column by default
- **Filtering**: Selector filter is applied to the working dataset (which excludes deleted rows upstream)
- **Modification**: Update does NOT modify `_deleted` (that's the `delete` operation's job)

---

## Schema Evolution

### Adding New Columns

When an assignment targets a non-existent column:

1. Polars `with_column()` automatically adds the column to the schema
2. Non-matching rows (filtered out by selector) receive `NULL` for the new column
3. Schema change propagates to downstream operations

**Example**:
```yaml
# Before: schema = [id, name, amount]
assignments:
  - column: discount_rate
    expression: "0.1"

# After: schema = [id, name, amount, discount_rate]
# Matching rows: discount_rate = 0.1
# Non-matching rows: discount_rate = NULL
```

### Modifying Existing Columns

When an assignment targets an existing column:

1. Column type is preserved (or coerced if expression type differs)
2. Only matching rows have values updated
3. Non-matching rows retain original values

**Type Coercion**: Polars handles type coercion based on expression result type. If incompatible, runtime error.

---

## Polars Integration

### Expression Compilation Mapping

| DSL Concept | Polars API |
|-------------|------------|
| Column reference (e.g., `orders.amount`) | `col("orders.amount")` or `col("amount")` |
| Literal value (e.g., `0.1`, `"EMEA"`) | `lit(0.1)`, `lit("EMEA")` |
| Arithmetic (e.g., `a + b`) | `col("a").add(col("b"))` or `col("a") + col("b")` |
| Comparison (e.g., `x > 10`) | `col("x").gt(lit(10))` |
| Function call (e.g., `CONCAT(a, b)`) | Custom function mapping (TBD in S01) |
| Conditional (e.g., `IF(cond, then, else)`) | `when(cond).then(then_val).otherwise(else_val)` |

**Note**: Exact mapping depends on S01 (DSL Parser) expression AST.

### LazyFrame Operations

| Operation | Polars Method | Description |
|-----------|---------------|-------------|
| Filter by selector | `.filter(selector_expr)` | Apply selector to get matching rows |
| Apply assignments | `.with_columns(assignment_exprs)` | Bulk column updates |
| Update system column | `.with_column(lit(timestamp).alias("_updated_at"))` | Set _updated_at |
| Merge rows | `.vstack(non_matching_rows)` or similar | Union updated + unchanged rows |

**Lazy Evaluation**: All operations return `LazyFrame`, no materialization until final `collect()`.

---

## Relationships to Other Entities

### Operation (parent entity)

- `UpdateOperation` is created from `Operation.arguments` when `Operation.type == OperationKind::Update`
- Deserialization: `serde_json::from_value::<UpdateOperationArgs>(operation.parameters)`

### Project

- Provides `selectors` map for named selector resolution
- Update operation executor receives selectors via `UpdateExecutionContext`

### Expression (dependency)

- Update operation depends on expression parsing (S01)
- Expression strings in selector and assignments are compiled to Polars `Expr`

### LazyFrame (input/output)

- Input: Working dataset from previous operation
- Output: Updated working dataset for next operation

---

## Example Data Flow

**Input**:
```rust
UpdateOperation {
    selector: Some("{{active_orders}}"),
    assignments: vec![
        Assignment { column: "status", expression: "\"processed\"" },
        Assignment { column: "processed_at", expression: "NOW()" },
    ]
}

UpdateExecutionContext {
    working_dataset: LazyFrame { /* 1000 rows */ },
    selectors: HashMap::from([("active_orders", "orders.status = \"active\"")]),
    run_timestamp: DateTime::parse_from_rfc3339("2025-02-22T10:00:00Z").unwrap(),
}
```

**Execution**:
1. Resolve selector: `{{active_orders}}` → `orders.status = "active"`
2. Compile selector: `col("orders.status").eq(lit("active"))`
3. Filter: 800 rows match, 200 rows don't match
4. Compile assignments:
   - `lit("processed").alias("status")`
   - `lit(NOW()).alias("processed_at")` (simplified; actual impl depends on DSL)
5. Apply: `with_columns([status_expr, processed_at_expr])`
6. Update system column: `with_column(lit(run_timestamp).alias("_updated_at"))`
7. Union: Merge 800 updated rows + 200 unchanged rows
8. Output: LazyFrame with 1000 rows

**Output**:
```rust
LazyFrame {
    // 800 rows: status = "processed", processed_at = NOW(), _updated_at = run_timestamp
    // 200 rows: unchanged (original values retained)
}
```

---

## Open Questions

| ID | Question | Status |
|----|----------|--------|
| DM-001 | How are expressions represented in S01 (AST vs. IR vs. string)? | Dependency on S01 |
| DM-002 | Should we support join aliases in update (deferred to S05)? | Out of scope for S04 |
| DM-003 | How to handle NOW() or other runtime functions in expressions? | Dependency on S01 |

---

## Summary

- **Core entities**: UpdateOperation, Assignment, UpdateExecutionContext
- **Validation**: Compile-time (syntax, selector resolution) + runtime (Polars schema checks)
- **State flow**: Resolve → Compile → Filter → Apply → Update → Union
- **Integration**: Polars LazyFrame API, expression compilation from DSL (S01 dependency)
