# Append Operation YAML Schema

This schema defines the structure for the `append` operation type in YAML format.

## Schema Definition (OpenAPI 3.0)

```yaml
openapi: 3.0.0
info:
  title: Append Operation API
  version: 1.0.0
  description: Schema for append operation configuration

components:
  schemas:
    AppendOperation:
      type: object
      required:
        - type
        - parameters
      properties:
        type:
          type: string
          enum: [append]
          description: Operation type identifier
        order:
          type: integer
          minimum: 0
          description: Execution order in the operation pipeline
        alias:
          type: string
          description: Optional alias for referencing this operation
        parameters:
          $ref: '#/components/schemas/AppendParameters'
    
    AppendParameters:
      type: object
      required:
        - source
      properties:
        source:
          $ref: '#/components/schemas/DatasetRef'
        source_selector:
          type: string
          description: Optional filter expression for source rows
          example: "budget_type = 'original'"
        aggregation:
          $ref: '#/components/schemas/AppendAggregation'
    
    DatasetRef:
      type: object
      required:
        - dataset_id
      properties:
        dataset_id:
          type: string
          format: uuid
          description: UUID of the source dataset
          example: "550e8400-e29b-41d4-a716-446655440000"
        dataset_version:
          type: integer
          minimum: 1
          description: Optional version pinning
          example: 2
    
    AppendAggregation:
      type: object
      required:
        - group_by
        - aggregations
      properties:
        group_by:
          type: array
          items:
            type: string
          minItems: 1
          description: Columns to group by (must exist in source dataset)
          example:
            - account_code
            - cost_center_code
        aggregations:
          type: array
          items:
            $ref: '#/components/schemas/Aggregation'
          minItems: 1
          description: List of aggregate computations
    
    Aggregation:
      type: object
      required:
        - column
        - expression
      properties:
        column:
          type: string
          description: Output column name for aggregated value
          example: total_budget
        expression:
          type: string
          pattern: '^(SUM|COUNT|AVG|MIN_AGG|MAX_AGG)\(.+\)$'
          description: Aggregate function expression
          example: "SUM(amount)"
```

## Example Instances

### Example 1: Simple Append (Budget to Transactions)

```yaml
type: append
order: 1
parameters:
  source:
    dataset_id: a1b2c3d4-e5f6-7890-abcd-ef1234567890
```

**Description**: Append all budget rows to transaction dataset  
**User Story**: US-1, TS-01  
**Expected Behavior**: Load all rows from budget dataset, align columns, append to working dataset

---

### Example 2: Filtered Append (Original Budgets Only)

```yaml
type: append
order: 2
alias: append_original_budgets
parameters:
  source:
    dataset_id: b2c3d4e5-f6a7-8901-bcde-f12345678901
  source_selector: "budget_type = 'original'"
```

**Description**: Append only "original" budget type rows  
**User Story**: US-2, TS-06  
**Expected Behavior**: 
- Load budget dataset (12 rows)
- Filter to `budget_type = 'original'` (4 rows)
- Append 4 rows to working dataset

---

### Example 3: Aggregated Append (Monthly Totals by Account)

```yaml
type: append
order: 3
parameters:
  source:
    dataset_id: c3d4e5f6-a7b8-9012-cdef-123456789012
    dataset_version: 2
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

**Description**: Append monthly budget totals grouped by account and cost center  
**User Story**: US-3, TS-13  
**Expected Behavior**:
- Load budget dataset version 2
- Group by `account_code` and `cost_center_code`
- Compute `SUM(amount)` as `total_budget`
- Compute `COUNT(budget_id)` as `budget_count`
- Append aggregated rows (one per group combination)

---

### Example 4: Period-Filtered Append (Temporal Mode)

```yaml
type: append
order: 4
parameters:
  source:
    dataset_id: d4e5f6a7-b8c9-0123-def1-234567890123
```

**Description**: Append budget rows filtered by run period (temporal_mode: period)  
**User Story**: US-4, TS-16  
**Expected Behavior**:
- Load budget dataset with `temporal_mode = period`
- Automatically filter `_period = run_period.identifier` (e.g., "2026-01")
- Append only rows matching the run period

**Note**: Temporal filtering is automatic based on source dataset's `temporal_mode`. No explicit filter parameter needed.

---

### Example 5: Complex Filtered + Aggregated Append

```yaml
type: append
order: 5
alias: high_value_department_summary
parameters:
  source:
    dataset_id: e5f6a7b8-c9d0-1234-ef12-345678901234
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
      - column: max_high_value
        expression: "MAX_AGG(amount)"
```

**Description**: Append aggregated high-value approved transactions by department  
**User Story**: US-3, TS-15  
**Expected Behavior**:
1. Load source dataset
2. Apply temporal filtering (if applicable)
3. Filter: `amount > 10000 AND status = 'approved'`
4. Group by `department_code`
5. Compute 4 aggregations (sum, count, avg, max)
6. Append aggregated rows

**Execution Order**: Filter → Aggregate → Append (per FR-006)

---

## Validation Examples

### Valid Configurations

#### Minimal Configuration

```yaml
type: append
parameters:
  source:
    dataset_id: f6a7b8c9-d0e1-2345-f123-456789012345
```

✅ **Valid**: Only `dataset_id` is required

---

#### With Source Selector

```yaml
type: append
parameters:
  source:
    dataset_id: f6a7b8c9-d0e1-2345-f123-456789012345
  source_selector: "fiscal_year = 2026"
```

✅ **Valid**: Simple equality filter

---

#### With Aggregation

```yaml
type: append
parameters:
  source:
    dataset_id: f6a7b8c9-d0e1-2345-f123-456789012345
  aggregation:
    group_by:
      - account_code
    aggregations:
      - column: total
        expression: "SUM(amount)"
```

✅ **Valid**: Single group-by column, single aggregation

---

### Invalid Configurations

#### Missing Required Field (dataset_id)

```yaml
type: append
parameters:
  source: {}
```

❌ **Invalid**: `dataset_id` is required in `source`

**Error**: Schema validation failure - missing required property `dataset_id`

---

#### Empty group_by

```yaml
type: append
parameters:
  source:
    dataset_id: f6a7b8c9-d0e1-2345-f123-456789012345
  aggregation:
    group_by: []
    aggregations:
      - column: total
        expression: "SUM(amount)"
```

❌ **Invalid**: `group_by` must have at least one column

**Error**: Schema validation failure - `group_by` minItems is 1

---

#### Invalid Aggregate Function

```yaml
type: append
parameters:
  source:
    dataset_id: f6a7b8c9-d0e1-2345-f123-456789012345
  aggregation:
    group_by:
      - account_code
    aggregations:
      - column: median_amount
        expression: "MEDIAN(amount)"
```

❌ **Invalid**: `MEDIAN` is not a supported aggregate function

**Error**: Expression pattern validation failure - must match `^(SUM|COUNT|AVG|MIN_AGG|MAX_AGG)\(.+\)$`

---

#### Malformed Expression

```yaml
type: append
parameters:
  source:
    dataset_id: f6a7b8c9-d0e1-2345-f123-456789012345
  aggregation:
    group_by:
      - account_code
    aggregations:
      - column: total
        expression: "SUM amount"  # Missing parentheses
```

❌ **Invalid**: Expression doesn't match required pattern

**Error**: Expression pattern validation failure

---

## Runtime Validation (Beyond Schema)

### Dataset Existence Check

```yaml
type: append
parameters:
  source:
    dataset_id: 00000000-0000-0000-0000-000000000000  # Non-existent dataset
```

❌ **Runtime Error**: Dataset not found

```yaml
error:
  code: APPEND_001
  message: "Dataset not found"
  details:
    dataset_id: "00000000-0000-0000-0000-000000000000"
```

---

### Column Validation (Source Selector)

```yaml
type: append
parameters:
  source:
    dataset_id: f6a7b8c9-d0e1-2345-f123-456789012345
  source_selector: "nonexistent_column = 'value'"
```

❌ **Runtime Error**: Column not found in source dataset

```yaml
error:
  code: APPEND_006
  message: "Column referenced in expression not found in source dataset"
  details:
    column: "nonexistent_column"
    context: "source_selector"
```

---

### Column Validation (Aggregation)

```yaml
type: append
parameters:
  source:
    dataset_id: f6a7b8c9-d0e1-2345-f123-456789012345
  aggregation:
    group_by:
      - account_code
    aggregations:
      - column: nonexistent_output_column  # Not in working dataset
        expression: "SUM(amount)"
```

❌ **Runtime Error**: Output column not in working dataset

```yaml
error:
  code: APPEND_003
  message: "Appended rows contain columns not in working dataset"
  details:
    extra_columns: ["nonexistent_output_column"]
```

---

## Complete Pipeline Example

### Scenario: Budget vs Actual Analysis

```yaml
operations:
  # Load actual transactions (working dataset)
  - type: load
    order: 1
    parameters:
      dataset_id: "actuals-2026-01"
  
  # Append original budget (simple)
  - type: append
    order: 2
    alias: append_budgets
    parameters:
      source:
        dataset_id: "budgets-2026"
      source_selector: "budget_type = 'original'"
  
  # Append aggregated forecast by account
  - type: append
    order: 3
    alias: append_forecast_summary
    parameters:
      source:
        dataset_id: "forecast-2026"
        dataset_version: 1
      aggregation:
        group_by:
          - account_code
        aggregations:
          - column: forecast_total
            expression: "SUM(amount)"
          - column: forecast_count
            expression: "COUNT(*)"
  
  # Output combined dataset
  - type: output
    order: 4
    parameters:
      format: parquet
      path: "budget_vs_actual_2026_01.parquet"
```

**Expected Result**:
- Working dataset: 10,000 actual transaction rows
- After operation 2: +4 budget rows → 10,004 rows
- After operation 3: +12 forecast summary rows → 10,016 rows
- Output: 10,016 rows with actuals, budgets, and forecast summaries

---

**Schema version**: 1.0.0  
**Last updated**: 2026-02-22  
**Specification**: [spec.md](../spec.md)
