# Aggregate Operation API Contract

**Version**: 1.0.0  
**Feature**: 001-aggregate-operation  
**Date**: 2026-02-22

## Overview

This document defines the API contract for the aggregate operation, including serialization formats, validation rules, and runtime behavior.

---

## 1. Operation Definition (YAML/JSON)

### Schema

```yaml
# Aggregate operation embedded in OperationInstance
type: aggregate
alias: "monthly_totals"  # optional
parameters:
  group_by:
    - "account_type"
    - "_period"
  aggregations:
    - column: "total_amount"
      expression: "SUM(amount)"
    - column: "order_count"
      expression: "COUNT(*)"
```

### JSON Schema

```json
{
  "$schema": "http://json-schema.org/draft-07/schema#",
  "title": "AggregateOperation",
  "type": "object",
  "required": ["group_by", "aggregations"],
  "properties": {
    "group_by": {
      "type": "array",
      "items": {
        "type": "string",
        "minLength": 1
      },
      "minItems": 1,
      "uniqueItems": true,
      "description": "List of column names to group by"
    },
    "aggregations": {
      "type": "array",
      "items": {
        "$ref": "#/definitions/Aggregation"
      },
      "minItems": 1,
      "description": "List of aggregate computations"
    }
  },
  "definitions": {
    "Aggregation": {
      "type": "object",
      "required": ["column", "expression"],
      "properties": {
        "column": {
          "type": "string",
          "minLength": 1,
          "pattern": "^(?!_row_id|_created_at|_updated_at|_source_dataset_id|_source_table|_deleted|_period).*$",
          "description": "Output column name for aggregate result"
        },
        "expression": {
          "type": "object",
          "required": ["source"],
          "properties": {
            "source": {
              "type": "string",
              "pattern": ".*(SUM|COUNT|AVG|MIN_AGG|MAX_AGG).*",
              "description": "Aggregate expression string"
            }
          }
        }
      }
    }
  }
}
```

---

## 2. Rust API

### Public Types

```rust
// Re-exported from crate::model::operation
pub use crate::model::operation::{OperationInstance, OperationKind};
pub use crate::model::expression::Expression;

/// Aggregate operation specification
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
pub struct AggregateOperation {
    pub group_by: Vec<String>,
    pub aggregations: Vec<Aggregation>,
}

/// Single aggregate computation
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
pub struct Aggregation {
    pub column: String,
    pub expression: Expression,
}
```

### Error Types

```rust
/// Errors specific to aggregate operation
#[derive(Debug, thiserror::Error)]
pub enum AggregateError {
    #[error("group_by list cannot be empty")]
    EmptyGroupBy,
    
    #[error("aggregations list cannot be empty")]
    EmptyAggregations,
    
    #[error("duplicate group_by column: {0}")]
    DuplicateGroupByColumn(String),
    
    #[error("unknown column in group_by: {0}")]
    UnknownGroupByColumn(String),
    
    #[error("unknown column in aggregation expression: {0}")]
    UnknownAggregationColumn(String),
    
    #[error("aggregate output column conflicts with system column: {0}")]
    SystemColumnConflict(String),
    
    #[error("duplicate aggregation output column: {0}")]
    DuplicateAggregationColumn(String),
    
    #[error("invalid aggregate expression: {0}")]
    InvalidExpression(String),
    
    #[error("aggregate function not allowed in this context: {0}")]
    InvalidAggregateContext(String),
    
    #[error("execution failed: {0}")]
    ExecutionError(String),
}
```

### Public Functions

```rust
/// Validate aggregate operation definition (parse-time)
pub fn validate_aggregate_spec(spec: &AggregateOperation) -> Result<(), AggregateError>;

/// Validate aggregate operation against dataset schema (compile-time)
pub fn validate_aggregate_compile(
    spec: &AggregateOperation,
    schema: &SchemaRef,
) -> Result<(), AggregateError>;

/// Execute aggregate operation on working dataset
pub fn execute_aggregate(
    spec: &AggregateOperation,
    working_dataset: LazyFrame,
    selector: Option<PolarsExpr>,
    execution_context: ExecutionContext,
) -> Result<LazyFrame, AggregateError>;
```

---

## 3. Validation Rules (Contract)

### Parse-Time Validation

| Rule ID | Check | Error |
|---------|-------|-------|
| V-001 | `group_by` is non-empty | `AggregateError::EmptyGroupBy` |
| V-002 | `aggregations` is non-empty | `AggregateError::EmptyAggregations` |
| V-003 | No duplicate columns in `group_by` | `AggregateError::DuplicateGroupByColumn(name)` |
| V-004 | No duplicate `column` in `aggregations` | `AggregateError::DuplicateAggregationColumn(name)` |
| V-005 | Aggregation output column not a system column | `AggregateError::SystemColumnConflict(name)` |

**System Column Names** (reserved):
- `_row_id`
- `_created_at`
- `_updated_at`
- `_source_dataset_id`
- `_source_table`
- `_deleted`
- `_period` (allowed if not in group_by, but discouraged)

### Compile-Time Validation

| Rule ID | Check | Error |
|---------|-------|-------|
| V-101 | All `group_by` columns exist in dataset schema | `AggregateError::UnknownGroupByColumn(name)` |
| V-102 | All column references in expressions exist | `AggregateError::UnknownAggregationColumn(name)` |
| V-103 | All expressions contain valid aggregate functions | `AggregateError::InvalidExpression(details)` |
| V-104 | Aggregate functions only in aggregate context | `AggregateError::InvalidAggregateContext(func)` |

### Runtime Validation

| Rule ID | Check | Behavior |
|---------|-------|----------|
| V-201 | Selector produces valid row subset | Return empty dataset if zero rows match |
| V-202 | Group-by columns have compatible types | Execution error if incompatible |

---

## 4. Execution Behavior Contract

### Inputs

| Input | Type | Required | Description |
|-------|------|----------|-------------|
| `spec` | `AggregateOperation` | Yes | Operation specification |
| `working_dataset` | `LazyFrame` | Yes | Current working dataset |
| `selector` | `Option<PolarsExpr>` | No | Row filter (default: all non-deleted rows) |
| `execution_context` | `ExecutionContext` | Yes | Execution metadata (timestamp, dataset ID, etc.) |

### Outputs

| Output | Type | Description |
|--------|------|-------------|
| Success | `LazyFrame` | Updated working dataset with summary rows appended |
| Failure | `AggregateError` | Validation or execution error |

### Guarantees

1. **Atomicity**: Operation either fully succeeds or fully fails (no partial results)
2. **Row Preservation**: All input rows present in output dataset
3. **Row Appending**: Summary rows added at end (after detail rows)
4. **Schema Consistency**: Output schema matches input schema
5. **System Metadata**: All summary rows have valid system columns
6. **Null Handling**: Non-aggregated columns are null on summary rows

### Side Effects

- **None**: Aggregate operation is purely functional (no I/O, no external state mutation)

---

## 5. Serialization Examples

### Example 1: Basic Aggregation

**YAML**:
```yaml
type: aggregate
parameters:
  group_by:
    - "account_type"
  aggregations:
    - column: "total_balance"
      expression: "SUM(balance)"
```

**JSON**:
```json
{
  "type": "aggregate",
  "parameters": {
    "group_by": ["account_type"],
    "aggregations": [
      {
        "column": "total_balance",
        "expression": {
          "source": "SUM(balance)"
        }
      }
    ]
  }
}
```

**Rust Deserialization**:
```rust
let op_instance: OperationInstance = serde_json::from_str(json_str)?;
assert_eq!(op_instance.kind, OperationKind::Aggregate);

let spec: AggregateOperation = serde_json::from_value(op_instance.parameters)?;
assert_eq!(spec.group_by, vec!["account_type"]);
assert_eq!(spec.aggregations.len(), 1);
```

### Example 2: Multi-Column Group By

**YAML**:
```yaml
type: aggregate
alias: "monthly_account_summary"
parameters:
  group_by:
    - "_period"
    - "account_type"
    - "region"
  aggregations:
    - column: "total_amount"
      expression: "SUM(amount)"
    - column: "avg_amount"
      expression: "AVG(amount)"
    - column: "transaction_count"
      expression: "COUNT(*)"
    - column: "max_amount"
      expression: "MAX_AGG(amount)"
```

### Example 3: Embedded in OperationInstance

**YAML**:
```yaml
order: 3
type: aggregate
alias: "region_totals"
parameters:
  group_by:
    - "region"
  aggregations:
    - column: "region_revenue"
      expression: "SUM(revenue)"
```

**Rust**:
```rust
let op = OperationInstance {
    order: 3,
    kind: OperationKind::Aggregate,
    alias: Some("region_totals".to_string()),
    parameters: serde_json::json!({
        "group_by": ["region"],
        "aggregations": [
            {
                "column": "region_revenue",
                "expression": { "source": "SUM(revenue)" }
            }
        ]
    }),
};
```

---

## 6. Test Contracts

### Unit Test Requirements

All implementations MUST pass these test cases:

1. **Empty Group By**: Reject with `EmptyGroupBy` error
2. **Empty Aggregations**: Reject with `EmptyAggregations` error
3. **Duplicate Group By Column**: Reject with `DuplicateGroupByColumn` error
4. **Unknown Group By Column**: Reject with `UnknownGroupByColumn` error
5. **System Column Conflict**: Reject with `SystemColumnConflict` error
6. **Duplicate Aggregation Column**: Reject with `DuplicateAggregationColumn` error

### Integration Test Requirements

1. **Basic Aggregation**: SUM, COUNT on single group-by column
2. **Multi-Column Grouping**: 2+ group-by columns, multiple aggregations
3. **All Aggregate Functions**: SUM, COUNT, AVG, MIN_AGG, MAX_AGG
4. **Selector Filtering**: Aggregate subset of rows via selector
5. **Row Preservation**: Verify original rows unchanged
6. **Summary Row Structure**: Verify system columns, nulls for non-aggregated
7. **Edge Case - Zero Rows**: Selector filters all rows → zero summaries
8. **Edge Case - Null Group Keys**: Null values treated as distinct group
9. **TS-05 Scenario**: Monthly totals by account type (from spec)

---

## 7. Breaking Changes Policy

### Backward Compatible Changes

- Adding new aggregate functions
- Adding optional fields to `AggregateOperation`
- Adding new validation warnings (non-blocking)
- Performance optimizations

### Breaking Changes (Require Version Bump)

- Removing or renaming fields in `AggregateOperation`
- Changing validation rules to reject previously valid specs
- Changing aggregate function semantics
- Modifying system column names or types

---

## Summary

This contract defines:
- ✅ YAML/JSON schema for aggregate operations
- ✅ Rust API types and functions
- ✅ Validation rules (parse-time, compile-time, runtime)
- ✅ Execution behavior and guarantees
- ✅ Serialization examples
- ✅ Test requirements
- ✅ Breaking changes policy

**Contract Version**: 1.0.0  
**Status**: Draft  
**Implementation**: Pending
