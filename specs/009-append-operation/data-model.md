# Data Model: Append Operation

**Feature**: 009-append-operation  
**Date**: 2026-02-22  
**Purpose**: Define entities, relationships, and state transitions for append operation

---

## Core Entities

### 1. AppendOperation

**Purpose**: Configuration for an append operation that loads and appends rows from a source dataset.

**Fields**:
| Field | Type | Required | Description | Validation Rules |
|-------|------|----------|-------------|------------------|
| `source` | DatasetRef | Yes | Reference to source dataset to append from | Must reference existing dataset |
| `source_selector` | Expression | No | Filter expression for source rows | Evaluated against source dataset columns |
| `aggregation` | AppendAggregation | No | Aggregation to apply before appending | group_by and aggregations must reference source columns |

**Relationships**:
- References → Dataset (via `source.dataset_id`)
- Contains → Expression (via `source_selector`)
- Contains → AppendAggregation (via `aggregation`)

**State Transitions**: N/A (immutable configuration)

**Example (YAML)**:
```yaml
type: append
parameters:
  source:
    dataset_id: "550e8400-e29b-41d4-a716-446655440000"
    dataset_version: 3  # optional pinning
  source_selector: "budget_type = 'original'"
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

**Example (JSON)**:
```json
{
  "type": "append",
  "parameters": {
    "source": {
      "dataset_id": "550e8400-e29b-41d4-a716-446655440000"
    },
    "source_selector": "amount > 10000"
  }
}
```

---

### 2. DatasetRef

**Purpose**: Reference to a source dataset with optional version pinning.

**Fields**:
| Field | Type | Required | Description | Validation Rules |
|-------|------|----------|-------------|------------------|
| `dataset_id` | UUID | Yes | Unique identifier of the dataset | Must exist in MetadataStore |
| `dataset_version` | i32 | No | Specific version to use (if pinned) | Must match existing dataset version |

**Relationships**:
- References → Dataset (via `dataset_id`)

**Validation Rules**:
- Dataset with `dataset_id` MUST exist
- If `dataset_version` specified, dataset MUST have that exact version
- If `dataset_version` omitted, use latest active version (status = Active)

**Example**:
```yaml
source:
  dataset_id: "550e8400-e29b-41d4-a716-446655440000"
  dataset_version: 2  # optional
```

---

### 3. AppendAggregation

**Purpose**: Configuration for aggregating source rows before appending (grouped summarization).

**Fields**:
| Field | Type | Required | Description | Validation Rules |
|-------|------|----------|-------------|------------------|
| `group_by` | Vec\<String\> | Yes | Columns to group by | All columns must exist in source dataset |
| `aggregations` | Vec\<Aggregation\> | Yes | Aggregate computations to perform | At least one aggregation required |

**Relationships**:
- Contains → List of Aggregation

**Validation Rules**:
- `group_by` columns MUST exist in source dataset schema
- At least one `Aggregation` MUST be specified
- All aggregation output columns MUST exist in working dataset schema

**Example**:
```yaml
aggregation:
  group_by:
    - account_code
    - cost_center_code
  aggregations:
    - column: total_budget
      expression: "SUM(amount)"
    - column: line_count
      expression: "COUNT(*)"
    - column: avg_amount
      expression: "AVG(amount)"
```

**State Transitions**: N/A (immutable configuration)

---

### 4. Aggregation

**Purpose**: Single aggregate computation with output column name and aggregate expression.

**Fields**:
| Field | Type | Required | Description | Validation Rules |
|-------|------|----------|-------------|------------------|
| `column` | String | Yes | Output column name for aggregated value | Must exist in working dataset schema |
| `expression` | String | Yes | Aggregate function expression | Must be valid aggregate function |

**Validation Rules**:
- `expression` MUST match pattern: `FUNCTION(column_name)`
- Supported functions: `SUM`, `COUNT`, `AVG`, `MIN_AGG`, `MAX_AGG`
- Column in expression MUST exist in source dataset schema
- Output `column` MUST exist in working dataset schema

**Example**:
```yaml
- column: total_budget
  expression: "SUM(amount)"
- column: budget_count
  expression: "COUNT(budget_id)"
- column: avg_budget
  expression: "AVG(amount)"
```

**Expression Parsing**:
```
Expression: "SUM(amount_local)"
  ├─ Function: SUM
  └─ Column: amount_local

Validation:
  1. Parse: Extract "SUM" and "amount_local"
  2. Verify: "amount_local" exists in source dataset
  3. Verify: Output column exists in working dataset
  4. Transform: col("amount_local").sum().alias(output_column)
```

---

### 5. Expression (existing entity, extended usage)

**Purpose**: Filter expression for source_selector (evaluates to boolean).

**Fields** (from existing model):
| Field | Type | Required | Description |
|-------|------|----------|-------------|
| `source` | String | Yes | Expression source code |

**Validation Rules for source_selector**:
- Expression MUST reference only source dataset columns
- Expression MUST evaluate to boolean (filter condition)
- Supported operators: `=`, `>`, `<`, `>=`, `<=`, `!=`, `AND`, `OR`, `NOT`
- Examples: `"budget_type = 'original'"`, `"amount > 10000"`, `"status = 'active' AND amount > 5000"`

**Example**:
```yaml
source_selector: "budget_type = 'original' AND fiscal_year = 2026"
```

---

### 6. System Columns (appended rows metadata)

**Purpose**: Metadata columns automatically added to all appended rows.

**Fields**:
| Column | Type | Description | Generation Rule |
|--------|------|-------------|-----------------|
| `_row_id` | String (UUID v7) | Unique row identifier | Generate new UUID v7 for each appended row |
| `_source_dataset` | String (UUID) | Source dataset ID | Set to `source.dataset_id` |
| `_operation_seq` | u32 | Operation sequence number | Set to current operation's sequence |
| `_deleted` | Boolean | Deletion flag | Always set to `false` for appended rows |

**Generation Logic**:
```rust
for each appended_row in source_df {
    appended_row._row_id = Uuid::now_v7().to_string();
    appended_row._source_dataset = source.dataset_id.to_string();
    appended_row._operation_seq = current_operation_seq;
    appended_row._deleted = false;
}
```

**Validation**:
- `_row_id` MUST be unique across all rows in result dataset
- `_source_dataset` MUST match the source DatasetRef.dataset_id
- `_operation_seq` MUST be monotonically increasing within run

---

## Entity Relationships Diagram

```
┌─────────────────────┐
│  OperationInstance  │
│  (type: append)     │
└──────────┬──────────┘
           │ contains parameters
           ▼
┌─────────────────────┐
│  AppendOperation    │
├─────────────────────┤
│ + source            │────────┐
│ + source_selector   │        │
│ + aggregation       │        │
└─────────────────────┘        │
           │                   │ references
           │                   ▼
           │            ┌──────────────┐
           │            │  Dataset     │
           │            │  (source)    │
           │            └──────────────┘
           │
           ├─ contains ──▶ ┌──────────────────┐
           │               │  Expression      │
           │               │ (source_selector)│
           │               └──────────────────┘
           │
           └─ contains ──▶ ┌─────────────────────┐
                           │ AppendAggregation   │
                           ├─────────────────────┤
                           │ + group_by: Vec     │
                           │ + aggregations: Vec │
                           └──────────┬──────────┘
                                      │
                                      │ contains
                                      ▼
                              ┌──────────────┐
                              │ Aggregation  │
                              ├──────────────┤
                              │ + column     │
                              │ + expression │
                              └──────────────┘
```

---

## Schema Alignment Rules

### Rule 1: Source Columns ⊂ Working Columns

**Validation**: All columns in appended rows (after aggregation) MUST exist in working dataset.

**Process**:
1. Load source dataset → columns: `{budget_id, account_code, amount}`
2. Apply aggregation → columns: `{account_code, total_budget, budget_count}`
3. Working dataset schema: `{journal_id, account_code, total_budget, budget_count, description}`
4. Validate: `{account_code, total_budget, budget_count}` ⊂ `{journal_id, account_code, total_budget, budget_count, description}` ✓

**Error Case**:
- Source columns: `{account_code, budget_type, amount}`
- Working columns: `{account_code, amount, description}`
- Extra column: `budget_type` → **ERROR**: Column 'budget_type' not in working dataset

---

### Rule 2: Missing Columns → NULL

**Process**: Columns in working dataset but NOT in appended rows are filled with NULL.

**Example**:
- Working schema: `{journal_id, account_code, amount, description}`
- Appended schema: `{account_code, amount}`
- Result: Add `journal_id = NULL`, `description = NULL` to all appended rows

**Implementation**:
```rust
for column in working_schema {
    if !appended_schema.contains(column) {
        appended_df.add_column(column, NULL);
    }
}
```

---

### Rule 3: Column Order Matches Working Dataset

**Process**: Reorder appended DataFrame columns to match working dataset column order.

**Example**:
- Appended columns (unordered): `{amount, account_code, _row_id}`
- Working columns (ordered): `{_row_id, account_code, amount, description}`
- Result: Reorder to `{_row_id, account_code, amount, description}`

---

## Temporal Filtering Rules

### Rule 1: Period Mode

**Trigger**: Source dataset has `temporal_mode = "period"`

**Filter**:
```sql
WHERE _period = run_period.identifier
```

**Example**:
- Run period: `"2026-01"`
- Source rows: `[{_period: "2025-12"}, {_period: "2026-01"}, {_period: "2026-02"}]`
- Filtered: `[{_period: "2026-01"}]`

---

### Rule 2: Bitemporal Mode

**Trigger**: Source dataset has `temporal_mode = "bitemporal"`

**Filter**:
```sql
WHERE valid_from <= run.asOf_date 
  AND (valid_to > run.asOf_date OR valid_to IS NULL)
```

**Example**:
- Run asOf: `2026-01-15`
- Source rows:
  - Row 1: `valid_from = 2026-01-01, valid_to = 2026-01-10` → Excluded
  - Row 2: `valid_from = 2026-01-01, valid_to = NULL` → Included
  - Row 3: `valid_from = 2026-01-20, valid_to = NULL` → Excluded

---

### Rule 3: Snapshot Mode (or NULL temporal_mode)

**Trigger**: Source dataset has `temporal_mode = NULL` or unspecified

**Filter**: None (append all rows)

---

## Operation Execution Flow

### Phase 1: Planning & Validation

```
1. Validate dataset reference
   ├─ Check dataset_id exists in MetadataStore
   ├─ If dataset_version specified, verify version matches
   └─ Load dataset metadata (schema, temporal_mode, resolver_id)

2. Validate source_selector (if present)
   ├─ Parse expression syntax
   └─ Verify all referenced columns exist in source schema

3. Validate aggregation (if present)
   ├─ Verify group_by columns exist in source schema
   ├─ Parse aggregation expressions
   ├─ Verify aggregation input columns exist in source schema
   └─ Verify aggregation output columns exist in working schema

4. Determine resolver
   ├─ Check project resolver_overrides for dataset_id
   ├─ Fallback to dataset.resolver_id
   └─ Fallback to system default resolver
```

### Phase 2: Data Loading

```
1. Load source dataset via DataLoader
   ├─ Use resolved resolver_id
   └─ Returns Polars DataFrame

2. Apply temporal filtering
   ├─ If temporal_mode = "period": filter _period = run_period
   ├─ If temporal_mode = "bitemporal": filter valid_from/valid_to
   └─ If temporal_mode = NULL: no filtering

3. Apply source_selector filtering (if present)
   └─ Filter rows matching expression

4. Check row count
   └─ If zero rows: return success with zero appended
```

### Phase 3: Aggregation (if configured)

```
1. Build group_by columns from AppendAggregation.group_by

2. Build aggregate expressions
   ├─ Parse each Aggregation.expression
   ├─ Transform to Polars Expr (e.g., SUM(x) → col(x).sum())
   └─ Apply alias to output column

3. Execute aggregation
   └─ df.group_by(group_cols).agg(agg_exprs)
```

### Phase 4: Schema Alignment

```
1. Extract schemas
   ├─ Appended schema: columns from aggregated/filtered source
   └─ Working schema: columns from working dataset

2. Validate: appended columns ⊂ working columns
   ├─ If extra columns exist: ERROR
   └─ Otherwise: proceed

3. Fill missing columns with NULL
   └─ For each working column not in appended: add NULL column

4. Reorder columns to match working schema
```

### Phase 5: System Columns & Append

```
1. Generate system columns
   ├─ _row_id: UUID v7 for each row
   ├─ _source_dataset: source.dataset_id
   ├─ _operation_seq: current operation sequence
   └─ _deleted: false

2. Concatenate DataFrames
   └─ working_df.vstack(appended_df)

3. Return result
   └─ AppendResult { rows_appended, result_df }
```

---

## Error Cases & Handling

| Error Type | Trigger | Error Code | Example |
|------------|---------|------------|---------|
| DatasetNotFound | dataset_id doesn't exist | APPEND_001 | source.dataset_id = "unknown-uuid" |
| DatasetVersionNotFound | Pinned version doesn't match | APPEND_002 | dataset_version = 5, actual version = 3 |
| ColumnMismatch | Extra columns in appended rows | APPEND_003 | Appended has "budget_type", working doesn't |
| ExpressionParseError | Invalid expression syntax | APPEND_004 | source_selector = "invalid syntax here" |
| InvalidAggregation | Unknown aggregate function | APPEND_005 | expression = "MEDIAN(amount)" (unsupported) |
| ColumnNotFound | Column in expression not in dataset | APPEND_006 | group_by: ["nonexistent_column"] |
| ResolverNotFound | No resolver configured | APPEND_007 | dataset has no resolver_id, no project override |
| DataLoadError | Data loading fails | APPEND_008 | Resolver returns error |

---

## Performance Characteristics

### Expected Latencies (from research.md)

| Scenario | Row Count | Operations | Target Latency |
|----------|-----------|------------|----------------|
| Simple append | 10,000 | Load + Concat | <10ms |
| Simple append | 100,000 | Load + Concat | <10ms |
| Filtered append | 10,000 | Load + Filter + Concat | <15ms |
| Filtered append | 100,000 | Load + Filter + Concat | <20ms |
| Aggregated append | 10,000 | Load + Filter + GroupBy + Agg + Concat | <30ms |
| Aggregated append | 100,000 | Load + Filter + GroupBy + Agg + Concat | <50ms |

### Optimization Strategies

1. **Lazy Evaluation**: Use Polars LazyFrame until final collect()
2. **Filter Early**: Apply source_selector before aggregation
3. **Batch System Columns**: Generate all UUIDs in one pass
4. **Schema Caching**: Cache working schema to avoid repeated lookups

---

## Data Model Change Summary

### New Entities
- `AppendOperation` (new operation parameters struct)
- `DatasetRef` (new dataset reference struct)
- `AppendAggregation` (new aggregation config struct)
- `Aggregation` (new single aggregation struct)

### Modified Entities
- `Expression` (extended usage for source_selector, no schema change)

### Existing Entities (unchanged)
- `OperationInstance` (already has `OperationKind::Append`)
- `Dataset` (used for source dataset lookup)
- `TemporalMode` (used for temporal filtering)

---

**Data model completed**: 2026-02-22  
**Next**: contracts/ (operation schemas), quickstart.md (usage guide)
