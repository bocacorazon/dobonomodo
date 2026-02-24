# Data Model: Aggregate Operation

**Feature**: 001-aggregate-operation  
**Date**: 2026-02-22  
**Phase**: 1 - Design & Contracts

## Overview

This document defines the data structures and entities for the aggregate operation feature, including operation definitions, runtime state, and output row schemas.

---

## 1. Operation Definition Schema

### AggregateOperation

Represents the configuration for an aggregate operation as defined in the project DSL.

| Field | Type | Required | Description |
|-------|------|----------|-------------|
| `group_by` | `Vec<String>` | Yes | List of column references to group by (e.g., `["account_type", "_period"]`) |
| `aggregations` | `Vec<Aggregation>` | Yes | List of aggregate computations to perform |

**Validation Rules**:
- `group_by` must be non-empty
- `aggregations` must be non-empty
- Column names in `group_by` must not contain duplicates
- All column references must exist in working dataset schema

**Rust Structure**:
```rust
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
pub struct AggregateOperation {
    pub group_by: Vec<String>,
    pub aggregations: Vec<Aggregation>,
}
```

---

### Aggregation

Defines a single aggregate computation within an aggregate operation.

| Field | Type | Required | Description |
|-------|------|----------|-------------|
| `column` | `String` | Yes | Output column name for the aggregate result |
| `expression` | `Expression` | Yes | Aggregate expression (must use aggregate function) |

**Validation Rules**:
- `column` must not conflict with system column names (`_row_id`, `_created_at`, `_updated_at`, `_source_dataset_id`, `_source_table`, `_deleted`, `_period`)
- `expression` must contain valid aggregate function (SUM, COUNT, AVG, MIN_AGG, MAX_AGG)
- `column` must be unique across all aggregations in the same operation

**Rust Structure**:
```rust
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
pub struct Aggregation {
    pub column: String,
    pub expression: Expression,
}
```

---

## 2. Runtime Execution State

### AggregateContext

Represents the runtime context for executing an aggregate operation.

| Field | Type | Description |
|-------|------|-------------|
| `operation_spec` | `AggregateOperation` | The operation definition |
| `working_dataset` | `LazyFrame` | Current working dataset (detail rows) |
| `dataset_schema` | `SchemaRef` | Schema of the working dataset |
| `selector_filter` | `Option<PolarsExpr>` | Optional row filter expression |
| `execution_time` | `DateTime<Utc>` | Timestamp for system metadata |
| `source_dataset_id` | `Uuid` | Input dataset UUID for lineage |
| `source_table` | `String` | Primary table name for lineage |

**Lifecycle**:
1. Created from operation definition + execution context
2. Validates operation against dataset schema
3. Compiles aggregate expressions
4. Executes aggregation
5. Produces summary rows
6. Destroyed after operation completes

**Rust Structure**:
```rust
pub struct AggregateContext<'a> {
    operation_spec: &'a AggregateOperation,
    working_dataset: LazyFrame,
    dataset_schema: SchemaRef,
    selector_filter: Option<PolarsExpr>,
    execution_time: DateTime<Utc>,
    source_dataset_id: Uuid,
    source_table: String,
}
```

---

## 3. Output Row Schema

### Summary Row Structure

Summary rows produced by aggregate operations follow this schema:

**Column Categories**:

1. **Group-by Columns** (from `group_by` specification):
   - Type: Matches original column type
   - Value: Group key value
   - Example: `account_type: "savings"`, `_period: "2024-01"`

2. **Aggregation Output Columns** (from `aggregations` specification):
   - Type: Determined by aggregate function
   - Value: Computed aggregate result
   - Example: `total_amount: 125000.50`, `order_count: 42`

3. **System Metadata Columns** (automatically populated):
   - `_row_id`: `Uuid` (v7, newly generated)
   - `_created_at`: `DateTime<Utc>` (execution time)
   - `_updated_at`: `DateTime<Utc>` (execution time, same as created)
   - `_source_dataset_id`: `Uuid` (from execution context)
   - `_source_table`: `String` (from execution context)
   - `_deleted`: `bool` (always `false`)
   - `_period`: `Option<String>` (from group-by if present, else null)

4. **Non-Aggregated Business Columns** (not in group-by or aggregations):
   - Type: Matches original column type
   - Value: `null`
   - Example: `customer_id: null`, `order_date: null`

**Example Summary Row**:

Working dataset columns: `account_type`, `customer_id`, `amount`, `order_date`, `_period`

Aggregate operation:
- group_by: `["account_type", "_period"]`
- aggregations: `[{column: "total_amount", expression: "SUM(amount)"}, {column: "order_count", expression: "COUNT(*)"}]`

Resulting summary row:
```json
{
  "_row_id": "01933e4f-8a12-7c9a-8e5d-123456789abc",
  "_created_at": "2026-02-22T10:30:00Z",
  "_updated_at": "2026-02-22T10:30:00Z",
  "_source_dataset_id": "550e8400-e29b-41d4-a716-446655440000",
  "_source_table": "transactions",
  "_deleted": false,
  "_period": "2024-01",
  "account_type": "savings",
  "total_amount": 125000.50,
  "order_count": 42,
  "customer_id": null,
  "order_date": null
}
```

---

## 4. Validation State Machine

### Validation Phases

```
┌─────────────────────┐
│  Operation Defined  │
└──────────┬──────────┘
           │
           ▼
┌─────────────────────┐
│  Parse Validation   │ ← Check: non-empty group_by, non-empty aggregations,
└──────────┬──────────┘         no duplicate group_by columns
           │
           ▼
┌─────────────────────┐
│ Compile Validation  │ ← Check: columns exist, expressions valid,
└──────────┬──────────┘         no system column conflicts
           │
           ▼
┌─────────────────────┐
│  Ready to Execute   │
└──────────┬──────────┘
           │
           ▼
┌─────────────────────┐
│ Runtime Execution   │ ← Check: selector produces valid rows,
└──────────┬──────────┘         group-by types compatible
           │
           ▼
┌─────────────────────┐
│  Summary Rows       │
│  Appended           │
└─────────────────────┘
```

**State Transitions**:
- `Defined → Parse Validation`: Automatic on operation load
- `Parse Validation → Compile Validation`: After schema resolution
- `Compile Validation → Ready`: After all compile checks pass
- `Ready → Runtime Execution`: On operation execution
- `Runtime Execution → Appended`: After successful aggregation

**Error States**:
- Parse validation failure: Return `AggregateError`, halt before execution
- Compile validation failure: Return `AggregateError`, halt before execution
- Runtime execution failure: Return execution error, rollback (no partial results)

---

## 5. Entity Relationships

```
┌────────────────────────────────────┐
│       OperationInstance            │
│  (from model::operation)           │
│  - kind: OperationKind::Aggregate  │
│  - parameters: JSON                │
└────────────┬───────────────────────┘
             │ deserializes to
             ▼
┌────────────────────────────────────┐
│      AggregateOperation            │
│  - group_by: Vec<String>           │
│  - aggregations: Vec<Aggregation>  │
└────────────┬───────────────────────┘
             │ contains
             ▼
┌────────────────────────────────────┐
│         Aggregation                │
│  - column: String                  │
│  - expression: Expression          │
└────────────────────────────────────┘

Execution Flow:
┌────────────────────────────────────┐
│      Working Dataset               │
│   (Polars LazyFrame)               │
└────────────┬───────────────────────┘
             │ filtered by selector
             ▼
┌────────────────────────────────────┐
│    Filtered Detail Rows            │
└────────────┬───────────────────────┘
             │ grouped by group_by
             ▼
┌────────────────────────────────────┐
│        Grouped Data                │
│   (Polars GroupBy)                 │
└────────────┬───────────────────────┘
             │ aggregated
             ▼
┌────────────────────────────────────┐
│     Summary Rows                   │
│  (grouped + aggregated cols)       │
└────────────┬───────────────────────┘
             │ enriched with metadata
             ▼
┌────────────────────────────────────┐
│  Complete Summary Rows             │
│  (with system cols + nulls)        │
└────────────┬───────────────────────┘
             │ appended to
             ▼
┌────────────────────────────────────┐
│   Updated Working Dataset          │
│   (detail + summary rows)          │
└────────────────────────────────────┘
```

---

## 6. Data Constraints

### Invariants

1. **Row Preservation**: `output_row_count = input_row_count + summary_row_count`
2. **Unique Row IDs**: All `_row_id` values across working dataset are unique
3. **Schema Consistency**: Summary rows match working dataset schema exactly
4. **System Column Integrity**: All summary rows have non-null system metadata (except nullable `_period`)
5. **Deletion State**: All summary rows have `_deleted = false` initially
6. **Null Columns**: Non-aggregated, non-grouped business columns are always null on summary rows

### Type Mappings (Aggregate Functions)

| Aggregate Function | Input Type | Output Type |
|--------------------|------------|-------------|
| `SUM(col)` | Numeric (Integer, Decimal) | Decimal |
| `COUNT(*)` | Any | Integer |
| `COUNT(col)` | Any | Integer |
| `AVG(col)` | Numeric (Integer, Decimal) | Decimal |
| `MIN_AGG(col)` | Comparable (Numeric, Date, Timestamp, String) | Same as input |
| `MAX_AGG(col)` | Comparable (Numeric, Date, Timestamp, String) | Same as input |

---

## 7. Edge Cases & Null Handling

### Edge Case: Zero Input Rows

- **Input**: Working dataset is empty or selector filters all rows
- **Behavior**: Aggregation produces zero summary rows
- **Output**: Working dataset unchanged
- **Rationale**: No groups exist to aggregate

### Edge Case: Single Group

- **Input**: All rows belong to same group
- **Behavior**: Exactly one summary row produced
- **Output**: Working dataset with +1 row

### Edge Case: Null Values in Group-By Columns

- **Input**: Group-by column contains null values
- **Behavior**: Null is treated as a distinct group key
- **Output**: Separate summary row for the null group
- **Rationale**: Matches SQL GROUP BY semantics

### Edge Case: Null Values in Aggregated Columns

- **Input**: Aggregate function input column contains nulls
- **Behavior**: Follows aggregate function null semantics:
  - `SUM`, `AVG`: Nulls ignored (sum of non-null values)
  - `COUNT(col)`: Nulls not counted
  - `COUNT(*)`: Nulls counted
  - `MIN_AGG`, `MAX_AGG`: Nulls ignored
- **Output**: Aggregate result based on non-null values
- **Rationale**: Standard SQL aggregate null handling

### Edge Case: All Nulls in Aggregated Column

- **Input**: All values in aggregate column are null
- **Behavior**:
  - `SUM`, `AVG`, `MIN_AGG`, `MAX_AGG`: Result is null
  - `COUNT(col)`: Result is 0
  - `COUNT(*)`: Result is group size
- **Output**: Summary row with null or zero aggregate value

---

## Summary

This data model defines:
- ✅ Operation definition schema (AggregateOperation, Aggregation)
- ✅ Runtime execution state (AggregateContext)
- ✅ Output row schema (summary row structure)
- ✅ Validation state machine
- ✅ Entity relationships and execution flow
- ✅ Data constraints and invariants
- ✅ Type mappings for aggregate functions
- ✅ Edge case handling and null semantics

**Ready to proceed to contract definitions and quickstart guide.**
