# Quickstart: Append Operation

**Feature**: 009-append-operation  
**Date**: 2026-02-22  
**Audience**: Developers implementing or using the append operation

---

## Overview

The **append** operation loads rows from a source dataset and adds them to the working dataset. This enables combining data from multiple datasets for comparative analysis (e.g., budget vs actual, forecast vs actuals).

**Key Capabilities**:
- ✅ Load rows from any dataset in the metadata store
- ✅ Filter source rows with `source_selector` expressions
- ✅ Aggregate source data before appending (group_by + aggregations)
- ✅ Automatic column alignment (fill missing columns with NULL)
- ✅ Automatic temporal filtering based on source dataset's temporal_mode
- ✅ System column generation (_row_id, _source_dataset, etc.)

---

## Quick Examples

### Example 1: Simple Append (Budget + Actuals)

**Use Case**: Combine budget rows with actual transaction rows for side-by-side comparison.

```yaml
type: append
parameters:
  source:
    dataset_id: "550e8400-e29b-41d4-a716-446655440000"
```

**What happens**:
1. Load all rows from budget dataset (id: `550e8400...`)
2. Align budget columns with working dataset schema (fill missing columns with NULL)
3. Generate system columns (_row_id, _source_dataset, etc.)
4. Append rows to working dataset

**Result**: Working dataset now contains both original rows + budget rows.

---

### Example 2: Filtered Append (Original Budget Only)

**Use Case**: Append only specific budget rows (e.g., "original" budget type, excluding revised/forecast).

```yaml
type: append
parameters:
  source:
    dataset_id: "b2c3d4e5-f6a7-8901-bcde-f12345678901"
  source_selector: "budget_type = 'original'"
```

**What happens**:
1. Load budget dataset (12 rows total)
2. Filter to rows where `budget_type = 'original'` (4 rows)
3. Align columns + generate system columns
4. Append 4 filtered rows

**Result**: Only "original" budget rows are appended.

---

### Example 3: Aggregated Append (Monthly Totals by Account)

**Use Case**: Append pre-aggregated summary rows (e.g., monthly budget totals by account) alongside detailed transactions.

```yaml
type: append
parameters:
  source:
    dataset_id: "c3d4e5f6-a7b8-9012-cdef-123456789012"
  aggregation:
    group_by:
      - account_code
      - cost_center_code
    aggregations:
      - column: total_budget
        expression: "SUM(amount)"
      - column: budget_count
        expression: "COUNT(budget_id)"
```

**What happens**:
1. Load budget dataset (100 rows)
2. Group by `account_code` and `cost_center_code` (10 unique combinations)
3. Compute `SUM(amount)` as `total_budget` for each group
4. Compute `COUNT(budget_id)` as `budget_count` for each group
5. Align + generate system columns
6. Append 10 aggregated rows

**Result**: Working dataset contains detailed transactions + 10 summary budget rows.

---

### Example 4: Filtered + Aggregated Append

**Use Case**: Append aggregated high-value transactions from another dataset.

```yaml
type: append
parameters:
  source:
    dataset_id: "d4e5f6a7-b8c9-0123-def1-234567890123"
  source_selector: "amount > 10000 AND status = 'approved'"
  aggregation:
    group_by:
      - department_code
    aggregations:
      - column: high_value_total
        expression: "SUM(amount)"
      - column: high_value_count
        expression: "COUNT(*)"
      - column: avg_high_value
        expression: "AVG(amount)"
```

**What happens**:
1. Load source dataset (1000 rows)
2. Filter to `amount > 10000 AND status = 'approved'` (50 rows)
3. Group filtered rows by `department_code` (5 departments)
4. Compute 3 aggregations per department
5. Align + generate system columns
6. Append 5 aggregated summary rows

**Result**: Working dataset + 5 high-value department summary rows.

**Note**: Filtering happens **before** aggregation (per FR-006).

---

## Common Patterns

### Pattern 1: Multi-Source Append (Budget vs Actual Pipeline)

```yaml
operations:
  # 1. Load actual transactions
  - type: load
    order: 1
    parameters:
      dataset_id: "actuals-2026-01"
  
  # 2. Append original budget
  - type: append
    order: 2
    parameters:
      source:
        dataset_id: "budgets-2026"
      source_selector: "budget_type = 'original'"
  
  # 3. Append revised budget
  - type: append
    order: 3
    parameters:
      source:
        dataset_id: "budgets-2026"
      source_selector: "budget_type = 'revised'"
  
  # 4. Output combined dataset
  - type: output
    order: 4
    parameters:
      format: parquet
```

**Use Case**: Create a dataset with actuals + original budget + revised budget for variance analysis.

---

### Pattern 2: Hierarchical Data (Detail + Summary)

```yaml
operations:
  # 1. Load detailed transactions
  - type: load
    order: 1
    parameters:
      dataset_id: "transactions-detail-2026-01"
  
  # 2. Append monthly summaries
  - type: append
    order: 2
    parameters:
      source:
        dataset_id: "transactions-detail-2026-01"
      aggregation:
        group_by:
          - account_code
          - month
        aggregations:
          - column: monthly_total
            expression: "SUM(amount)"
  
  # 3. Output with detail + summary
  - type: output
    order: 3
```

**Use Case**: Include both transaction-level detail and monthly summary rows in the same output for drill-down reporting.

---

### Pattern 3: Period-Filtered Multi-Dataset Append

```yaml
operations:
  # 1. Load working dataset (period: 2026-01)
  - type: load
    order: 1
    parameters:
      dataset_id: "actuals-2026"
      period: "2026-01"
  
  # 2. Append budget (automatically filtered to 2026-01)
  - type: append
    order: 2
    parameters:
      source:
        dataset_id: "budgets-2026"  # temporal_mode: period
  
  # 3. Append forecast (automatically filtered to 2026-01)
  - type: append
    order: 3
    parameters:
      source:
        dataset_id: "forecast-2026"  # temporal_mode: period
```

**Use Case**: Combine actuals, budgets, and forecasts all for the same period (2026-01). Temporal filtering is automatic based on source datasets' `temporal_mode`.

**Note**: No explicit `_period` filter needed - handled automatically.

---

## Configuration Reference

### AppendOperation Structure

```yaml
type: append                    # Required: operation type
order: <integer>                # Required: execution order
alias: <string>                 # Optional: operation alias
parameters:                     # Required: operation parameters
  source:                       # Required: source dataset reference
    dataset_id: <uuid>          # Required: UUID of source dataset
    dataset_version: <integer>  # Optional: pin to specific version
  source_selector: <expression> # Optional: filter source rows
  aggregation:                  # Optional: aggregate before appending
    group_by:                   # Required if aggregation: columns to group by
      - <column_name>
    aggregations:               # Required if aggregation: list of aggregates
      - column: <output_column> # Output column name
        expression: <agg_expr>  # Aggregate function expression
```

---

### Supported Aggregate Functions

| Function | Description | Example | Output Type |
|----------|-------------|---------|-------------|
| `SUM(col)` | Sum of values | `SUM(amount)` | Numeric |
| `COUNT(col)` | Count of non-null values | `COUNT(budget_id)` | Integer |
| `AVG(col)` | Average of values | `AVG(amount)` | Numeric |
| `MIN_AGG(col)` | Minimum value | `MIN_AGG(date_posted)` | Same as input |
| `MAX_AGG(col)` | Maximum value | `MAX_AGG(amount)` | Same as input |

**Note**: `COUNT(*)` counts all rows (including null values).

---

### Source Selector Syntax

Supported operators:
- **Comparison**: `=`, `>`, `<`, `>=`, `<=`, `!=`
- **Logical**: `AND`, `OR`, `NOT`
- **Literals**: `'string'`, `123`, `true`, `false`

Examples:
```yaml
# Equality
source_selector: "budget_type = 'original'"

# Numeric comparison
source_selector: "amount > 10000"

# Compound condition
source_selector: "status = 'approved' AND amount > 5000"

# Multiple conditions
source_selector: "fiscal_year = 2026 AND department_code IN ('HR', 'IT')"
```

---

## Temporal Filtering Behavior

The append operation **automatically** filters source rows based on the source dataset's `temporal_mode`:

### Period Mode

**Source dataset**: `temporal_mode = "period"`

**Filter**: `_period = run_period.identifier`

**Example**:
- Run period: `"2026-01"`
- Source rows: 100 rows across 12 months
- Filtered: Only rows where `_period = "2026-01"` (8 rows)
- Appended: 8 rows

---

### Bitemporal Mode

**Source dataset**: `temporal_mode = "bitemporal"`

**Filter**: `valid_from <= asOf_date AND (valid_to > asOf_date OR valid_to IS NULL)`

**Example**:
- Run asOf date: `2026-01-15`
- Source rows: 50 rows with various valid_from/valid_to ranges
- Filtered: Only rows valid as of 2026-01-15 (12 rows)
- Appended: 12 rows

---

### Snapshot Mode

**Source dataset**: `temporal_mode = NULL` (not set)

**Filter**: None

**Example**:
- Source rows: 200 rows
- Filtered: None
- Appended: All 200 rows

---

## Column Alignment Rules

### Rule 1: Source Columns Must Be Subset of Working Columns

**Valid**:
- Working dataset: `{journal_id, account_code, amount, description}`
- Source dataset: `{account_code, amount}`
- ✅ Result: Source columns are subset of working columns

**Invalid**:
- Working dataset: `{journal_id, account_code, amount}`
- Source dataset: `{account_code, amount, budget_type}`
- ❌ Error: Extra column `budget_type` not in working dataset

---

### Rule 2: Missing Columns Filled with NULL

**Example**:
- Working dataset: `{journal_id, account_code, amount, description}`
- Source dataset: `{account_code, amount}`
- Missing: `journal_id`, `description`
- Result: Appended rows have `journal_id = NULL`, `description = NULL`

---

### Rule 3: Column Order Matches Working Dataset

**Example**:
- Working dataset columns (ordered): `{_row_id, account_code, amount, description}`
- Source dataset columns (unordered): `{amount, account_code}`
- After alignment: `{_row_id, account_code, amount, description}`

---

## System Columns

All appended rows automatically receive system columns:

| Column | Type | Value | Description |
|--------|------|-------|-------------|
| `_row_id` | UUID v7 | Generated | Unique row identifier (time-ordered) |
| `_source_dataset` | UUID | Source dataset_id | Audit trail: where this row came from |
| `_operation_seq` | u32 | Operation order | Which operation created this row |
| `_deleted` | Boolean | `false` | Deletion flag (always false for appended rows) |

**Example**:
```yaml
# Appended row after system column generation
{
  "_row_id": "018e3c4a-b2d3-7890-abcd-ef1234567890",  # UUID v7
  "_source_dataset": "550e8400-e29b-41d4-a716-446655440000",
  "_operation_seq": 2,
  "_deleted": false,
  "account_code": "4100",
  "amount": 5000.00,
  "description": null
}
```

---

## Error Handling

### Common Errors

#### Dataset Not Found

**Cause**: `dataset_id` doesn't exist in metadata store

**Example**:
```yaml
source:
  dataset_id: "00000000-0000-0000-0000-000000000000"
```

**Error**:
```yaml
error:
  code: APPEND_001
  message: "Dataset not found"
  details:
    dataset_id: "00000000-0000-0000-0000-000000000000"
```

**Solution**: Verify dataset_id exists using metadata query

---

#### Column Mismatch

**Cause**: Source rows contain columns not in working dataset

**Example**:
```yaml
# Working columns: {account_code, amount}
# Source columns: {account_code, amount, budget_type}
```

**Error**:
```yaml
error:
  code: APPEND_003
  message: "Appended rows contain columns not in working dataset"
  details:
    extra_columns: ["budget_type"]
```

**Solution**: Either remove extra columns from source or add them to working dataset schema

---

#### Invalid Expression

**Cause**: `source_selector` or `aggregation.expression` has syntax error

**Example**:
```yaml
source_selector: "invalid syntax here"
```

**Error**:
```yaml
error:
  code: APPEND_004
  message: "Failed to parse source_selector expression"
  details:
    expression: "invalid syntax here"
    parse_error: "Expected comparison operator at position 8"
```

**Solution**: Fix expression syntax (check operator spelling, quotes, etc.)

---

#### Unsupported Aggregate Function

**Cause**: Aggregation expression uses unsupported function

**Example**:
```yaml
aggregations:
  - column: median_amount
    expression: "MEDIAN(amount)"
```

**Error**:
```yaml
error:
  code: APPEND_005
  message: "Invalid aggregation function"
  details:
    expression: "MEDIAN(amount)"
    supported_functions: ["SUM", "COUNT", "AVG", "MIN_AGG", "MAX_AGG"]
```

**Solution**: Use supported functions only (SUM, COUNT, AVG, MIN_AGG, MAX_AGG)

---

## Performance Considerations

### Expected Latencies

Based on research findings:

| Scenario | Row Count | Target Latency |
|----------|-----------|----------------|
| Simple append | 10,000 | <10ms |
| Simple append | 100,000 | <10ms |
| Filtered append | 10,000 | <15ms |
| Filtered append | 100,000 | <20ms |
| Aggregated append | 10,000 | <30ms |
| Aggregated append | 100,000 | <50ms |

### Optimization Tips

1. **Filter Early**: Use `source_selector` to reduce row count before aggregation
   ```yaml
   # Good: Filter 1M rows → 10k rows → aggregate
   source_selector: "fiscal_year = 2026"
   aggregation:
     group_by: [account_code]
     aggregations: [...]
   ```

2. **Version Pinning**: Pin dataset versions for reproducible results
   ```yaml
   source:
     dataset_id: "..."
     dataset_version: 3  # Always use version 3
   ```

3. **Batch Appends**: Combine multiple source_selector conditions instead of multiple append operations
   ```yaml
   # Better: Single append with OR condition
   source_selector: "budget_type IN ('original', 'revised')"
   
   # Worse: Two separate append operations
   # - First: budget_type = 'original'
   # - Second: budget_type = 'revised'
   ```

---

## Testing Checklist

When testing append operations:

- [ ] **Basic append**: Verify row count increases correctly
- [ ] **Column alignment**: Check NULL values in missing columns
- [ ] **System columns**: Verify _row_id, _source_dataset, _operation_seq populated
- [ ] **Filtering**: Verify source_selector reduces row count as expected
- [ ] **Aggregation**: Verify aggregated values match manual calculations
- [ ] **Temporal filtering**: Verify period/bitemporal filtering works automatically
- [ ] **Error handling**: Test with non-existent dataset_id, invalid expressions
- [ ] **Edge cases**: Test zero-row append (should succeed), empty group_by (should fail)

---

## Next Steps

1. **Review Spec**: See [spec.md](./spec.md) for detailed requirements
2. **Check Data Model**: See [data-model.md](./data-model.md) for entity definitions
3. **Review Contracts**: See [contracts/](./contracts/) for JSON/YAML schemas
4. **Implementation**: Follow [tasks.md](./tasks.md) (generated by `/speckit.tasks`)

---

**Quickstart version**: 1.0.0  
**Last updated**: 2026-02-22  
**Questions?**: See spec.md or data-model.md for more details
