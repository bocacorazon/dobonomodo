# Quickstart: Aggregate Operation

**Feature**: 001-aggregate-operation  
**Audience**: Developers implementing or using the aggregate operation  
**Time to Complete**: 15 minutes

## Overview

This guide demonstrates how to use the aggregate operation to group rows and compute summary values in DobONoMoDo pipelines.

---

## What You'll Learn

- How to define aggregate operations in YAML/JSON
- How to group rows by one or more columns
- How to compute aggregate functions (SUM, COUNT, AVG, MIN_AGG, MAX_AGG)
- How summary rows are appended to the working dataset
- How to validate aggregate operation definitions

---

## Prerequisites

- Basic understanding of DobONoMoDo operations
- Familiarity with YAML/JSON syntax
- Understanding of SQL-style GROUP BY and aggregate functions (helpful but not required)

---

## Quick Example

### Input Dataset

```
| account_type | customer_id | amount | order_date | _period  |
|--------------|-------------|--------|------------|----------|
| savings      | C001        | 1000   | 2024-01-15 | 2024-01  |
| checking     | C002        | 500    | 2024-01-20 | 2024-01  |
| savings      | C003        | 1500   | 2024-01-25 | 2024-01  |
| checking     | C004        | 750    | 2024-02-10 | 2024-02  |
| savings      | C005        | 2000   | 2024-02-15 | 2024-02  |
```

### Aggregate Operation

```yaml
type: aggregate
alias: "monthly_account_totals"
parameters:
  group_by:
    - "_period"
    - "account_type"
  aggregations:
    - column: "total_amount"
      expression: "SUM(amount)"
    - column: "customer_count"
      expression: "COUNT(*)"
```

### Output Dataset

Original rows remain **unchanged**, plus 3 new summary rows appended:

```
| account_type | customer_id | amount | order_date | _period  | total_amount | customer_count |
|--------------|-------------|--------|------------|----------|--------------|----------------|
| savings      | C001        | 1000   | 2024-01-15 | 2024-01  | null         | null           |
| checking     | C002        | 500    | 2024-01-20 | 2024-01  | null         | null           |
| savings      | C003        | 1500   | 2024-01-25 | 2024-01  | null         | null           |
| checking     | C004        | 750    | 2024-02-10 | 2024-02  | null         | null           |
| savings      | C005        | 2000   | 2024-02-15 | 2024-02  | null         | null           |
| savings      | null        | null   | null       | 2024-01  | 2500         | 2              | ← Summary
| checking     | null        | null   | null       | 2024-01  | 500          | 1              | ← Summary
| savings      | null        | null   | null       | 2024-02  | 2000         | 1              | ← Summary
```

**Key Points**:
- Original 5 detail rows preserved exactly
- 3 summary rows added (one per distinct group: 2024-01/savings, 2024-01/checking, 2024-02/savings)
- Summary rows have `total_amount` and `customer_count` computed
- Non-aggregated columns (`customer_id`, `amount`, `order_date`) are `null` on summary rows

---

## Step 1: Define Group-By Columns

Group-by columns determine how rows are partitioned for aggregation.

### Single Column Grouping

```yaml
parameters:
  group_by:
    - "account_type"
```

**Result**: One summary row per distinct `account_type` value.

### Multi-Column Grouping

```yaml
parameters:
  group_by:
    - "_period"
    - "account_type"
    - "region"
```

**Result**: One summary row per distinct combination of `_period`, `account_type`, and `region`.

### Rules

- At least one column required (empty group_by is invalid)
- Columns must exist in the working dataset
- No duplicate column names allowed
- Group-by values appear unchanged on summary rows

---

## Step 2: Define Aggregations

Aggregations compute summary values for each group.

### Available Aggregate Functions

| Function | Description | Example |
|----------|-------------|---------|
| `SUM(col)` | Sum of all values in group | `SUM(amount)` |
| `COUNT(*)` | Number of rows in group | `COUNT(*)` |
| `COUNT(col)` | Number of non-null values in group | `COUNT(customer_id)` |
| `AVG(col)` | Average of all values in group | `AVG(amount)` |
| `MIN_AGG(col)` | Minimum value in group | `MIN_AGG(amount)` |
| `MAX_AGG(col)` | Maximum value in group | `MAX_AGG(amount)` |

### Single Aggregation

```yaml
aggregations:
  - column: "total_revenue"
    expression: "SUM(revenue)"
```

### Multiple Aggregations

```yaml
aggregations:
  - column: "total_amount"
    expression: "SUM(amount)"
  - column: "avg_amount"
    expression: "AVG(amount)"
  - column: "order_count"
    expression: "COUNT(*)"
  - column: "max_amount"
    expression: "MAX_AGG(amount)"
```

### Rules

- At least one aggregation required
- Output column name must not conflict with system columns (`_row_id`, `_created_at`, etc.)
- Expression must use aggregate function (validated at compile-time)
- Output column names must be unique across all aggregations

---

## Step 3: Optional Selector Filtering

Filter which rows participate in aggregation using a selector.

### Example: Aggregate Only Active Orders

```yaml
type: aggregate
selector: "status = 'active'"
parameters:
  group_by:
    - "region"
  aggregations:
    - column: "active_order_count"
      expression: "COUNT(*)"
```

**Behavior**:
- Selector applied **before** grouping
- Only rows where `status = 'active'` are grouped and aggregated
- All other rows remain in the dataset unchanged
- Summary rows only reflect active orders

### Edge Case: Selector Filters All Rows

If the selector matches zero rows:
- Aggregation completes successfully
- Zero summary rows appended
- Working dataset unchanged

---

## Step 4: Understand Summary Row Structure

Summary rows have a specific structure with three column categories:

### 1. Group-By Columns

Copied from group key values.

```
_period: "2024-01"
account_type: "savings"
```

### 2. Aggregation Output Columns

Computed via aggregate functions.

```
total_amount: 2500
customer_count: 2
```

### 3. System Metadata Columns

Automatically populated by the engine.

| Column | Value |
|--------|-------|
| `_row_id` | New UUID v7 (time-ordered) |
| `_created_at` | Execution timestamp |
| `_updated_at` | Execution timestamp |
| `_source_dataset_id` | Input dataset UUID |
| `_source_table` | Primary table name |
| `_deleted` | `false` |

### 4. Non-Aggregated Business Columns

Set to `null`.

```
customer_id: null
amount: null
order_date: null
```

---

## Step 5: Validation

### Parse-Time Validation

Catches definition errors before execution:

```yaml
# ❌ Invalid: Empty group_by
parameters:
  group_by: []
  aggregations:
    - column: "total"
      expression: "SUM(amount)"
# Error: group_by list cannot be empty
```

```yaml
# ❌ Invalid: Duplicate group_by column
parameters:
  group_by:
    - "account_type"
    - "account_type"
  aggregations:
    - column: "total"
      expression: "SUM(amount)"
# Error: duplicate group_by column: account_type
```

```yaml
# ❌ Invalid: System column conflict
parameters:
  group_by:
    - "account_type"
  aggregations:
    - column: "_row_id"
      expression: "COUNT(*)"
# Error: aggregate output column conflicts with system column: _row_id
```

### Compile-Time Validation

Validates against dataset schema:

```yaml
# ❌ Invalid: Unknown column
parameters:
  group_by:
    - "nonexistent_column"
  aggregations:
    - column: "total"
      expression: "SUM(amount)"
# Error: unknown column in group_by: nonexistent_column
```

---

## Common Patterns

### Pattern 1: Totals by Time Period

```yaml
type: aggregate
alias: "monthly_totals"
parameters:
  group_by:
    - "_period"
  aggregations:
    - column: "monthly_revenue"
      expression: "SUM(revenue)"
```

### Pattern 2: Multi-Dimensional Summary

```yaml
type: aggregate
alias: "region_product_summary"
parameters:
  group_by:
    - "region"
    - "product_category"
    - "_period"
  aggregations:
    - column: "total_sales"
      expression: "SUM(sales_amount)"
    - column: "avg_price"
      expression: "AVG(unit_price)"
    - column: "units_sold"
      expression: "COUNT(*)"
```

### Pattern 3: Conditional Aggregation (via Selector)

```yaml
type: aggregate
selector: "order_status = 'completed' AND amount > 100"
alias: "high_value_completed_orders"
parameters:
  group_by:
    - "customer_tier"
  aggregations:
    - column: "high_value_revenue"
      expression: "SUM(amount)"
```

### Pattern 4: Multiple Statistics

```yaml
type: aggregate
alias: "customer_statistics"
parameters:
  group_by:
    - "customer_id"
  aggregations:
    - column: "total_spent"
      expression: "SUM(amount)"
    - column: "avg_order_value"
      expression: "AVG(amount)"
    - column: "order_count"
      expression: "COUNT(*)"
    - column: "largest_order"
      expression: "MAX_AGG(amount)"
    - column: "smallest_order"
      expression: "MIN_AGG(amount)"
```

---

## JSON Format

All YAML examples can be expressed in JSON:

```json
{
  "type": "aggregate",
  "alias": "monthly_totals",
  "parameters": {
    "group_by": ["_period", "account_type"],
    "aggregations": [
      {
        "column": "total_amount",
        "expression": {
          "source": "SUM(amount)"
        }
      },
      {
        "column": "customer_count",
        "expression": {
          "source": "COUNT(*)"
        }
      }
    ]
  }
}
```

---

## Rust API Usage (For Developers)

### Deserializing Operation Definition

```rust
use dobo_core::model::operation::{OperationInstance, OperationKind};
use dobo_core::engine::ops::aggregate::{AggregateOperation, Aggregation};

// From JSON
let json_str = r#"{
  "order": 3,
  "type": "aggregate",
  "parameters": {
    "group_by": ["account_type"],
    "aggregations": [{"column": "total", "expression": {"source": "SUM(amount)"}}]
  }
}"#;

let op: OperationInstance = serde_json::from_str(json_str)?;
let spec: AggregateOperation = serde_json::from_value(op.parameters)?;

assert_eq!(spec.group_by, vec!["account_type"]);
assert_eq!(spec.aggregations.len(), 1);
```

### Validation

```rust
use dobo_core::engine::ops::aggregate::{validate_aggregate_spec, validate_aggregate_compile};

// Parse-time validation
validate_aggregate_spec(&spec)?;

// Compile-time validation (with schema)
validate_aggregate_compile(&spec, &dataset_schema)?;
```

### Execution

```rust
use dobo_core::engine::ops::aggregate::execute_aggregate;

let updated_dataset = execute_aggregate(
    &spec,
    working_dataset,
    selector_expr,
    execution_context,
)?;
```

---

## Edge Cases & Gotchas

### Edge Case 1: Null Group Keys

Null values in group-by columns are treated as a distinct group:

**Input**:
```
| region | amount |
|--------|--------|
| US     | 100    |
| null   | 50     |
| null   | 75     |
```

**Operation**:
```yaml
group_by: ["region"]
aggregations:
  - column: "total"
    expression: "SUM(amount)"
```

**Output Summary Rows**:
```
| region | total |
|--------|-------|
| US     | 100   |
| null   | 125   |
```

### Edge Case 2: Aggregating Null Values

Null values in aggregated columns follow SQL semantics:

- `SUM`, `AVG`, `MIN_AGG`, `MAX_AGG`: Nulls ignored
- `COUNT(col)`: Nulls not counted
- `COUNT(*)`: Nulls counted

**Input**:
```
| region | amount |
|--------|--------|
| US     | 100    |
| US     | null   |
| US     | 200    |
```

**Operation**:
```yaml
group_by: ["region"]
aggregations:
  - column: "total"
    expression: "SUM(amount)"
  - column: "count_amount"
    expression: "COUNT(amount)"
  - column: "count_all"
    expression: "COUNT(*)"
```

**Output**:
```
| region | total | count_amount | count_all |
|--------|-------|--------------|-----------|
| US     | 300   | 2            | 3         |
```

### Edge Case 3: Zero Rows Matched

If no rows match the selector:

```yaml
selector: "status = 'archived'"  # No rows match
group_by: ["region"]
aggregations:
  - column: "total"
    expression: "SUM(amount)"
```

**Result**: Zero summary rows appended, working dataset unchanged.

---

## Troubleshooting

### Error: "group_by list cannot be empty"

**Cause**: No columns specified in `group_by`.

**Fix**: Add at least one column to `group_by`.

```yaml
# ❌ Before
group_by: []

# ✅ After
group_by:
  - "account_type"
```

### Error: "unknown column in group_by: xyz"

**Cause**: Column `xyz` doesn't exist in working dataset schema.

**Fix**: Verify column name spelling and availability.

```yaml
# Check dataset schema first
# Use exact column name from schema
group_by:
  - "account_type"  # Must match schema exactly
```

### Error: "aggregate output column conflicts with system column: _row_id"

**Cause**: Aggregation output column uses reserved system column name.

**Fix**: Rename the output column.

```yaml
# ❌ Before
aggregations:
  - column: "_row_id"
    expression: "COUNT(*)"

# ✅ After
aggregations:
  - column: "row_count"
    expression: "COUNT(*)"
```

---

## Next Steps

- **Implement**: Follow the TDD workflow in `tasks.md` to implement aggregate operation
- **Test**: Run integration tests with TS-05 scenario (monthly totals by account type)
- **Extend**: Combine with other operations (update, delete, output) in pipelines
- **Optimize**: Profile performance with large datasets (1M+ rows)

---

## Summary

You now know how to:
- ✅ Define aggregate operations in YAML/JSON
- ✅ Group rows by single or multiple columns
- ✅ Compute aggregate functions (SUM, COUNT, AVG, MIN_AGG, MAX_AGG)
- ✅ Understand summary row structure and null handling
- ✅ Validate operation definitions
- ✅ Handle edge cases (nulls, zero rows, selectors)
- ✅ Use the Rust API for validation and execution

**Next**: Proceed to implementation tasks in `tasks.md`.
