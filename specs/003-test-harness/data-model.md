# Data Model: Test Harness

**Feature**: 003-test-harness  
**Date**: 2026-02-22  
**Phase**: 1 (Design)

## Overview

This document defines the data structures and entities used by the test harness. All entities are defined in the `core/src/model` module for reuse across crates (`test-resolver`, `cli`).

---

## Entity Definitions

### TestScenario

The root entity representing a complete test definition.

| Field | Type | Required | Constraints | Description |
|-------|------|----------|-------------|-------------|
| `name` | `String` | Yes | Non-empty | Human-readable scenario name |
| `description` | `String` | No | — | Narrative description of what is being tested |
| `periods` | `Vec<PeriodDef>` | Yes | At least one | The Period(s) the Run will execute against |
| `input` | `TestInput` | Yes | — | Input Dataset definition with sample data |
| `project` | `ProjectDef` | Yes | — | The Project to execute (inline or reference) |
| `expected_output` | `TestOutput` | Yes | — | Expected result data for comparison |
| `expected_trace` | `Vec<TraceAssertion>` | No | — | Expected trace events (optional) |
| `config` | `TestConfig` | Yes | — | Test behavior configuration (has defaults) |

**Relationships**:
- Contains one `TestInput` (input dataset and data)
- Contains one `ProjectDef` (inline project or reference)
- Contains one `TestOutput` (expected results)
- Contains zero or more `TraceAssertion` (for trace validation)
- Contains one `TestConfig` (comparison behavior)

**Validation Rules**:
- `periods` must have at least one entry
- `input.data` must contain entries for all tables in `input.dataset`
- Each `DataBlock` must have exactly one of `rows` or `file`

---

### PeriodDef

Defines a Period for test execution.

| Field | Type | Required | Description |
|-------|------|----------|-------------|
| `identifier` | `String` | Yes | Period identifier (e.g., "2026-01") |
| `level` | `String` | Yes | Calendar level name (e.g., "month") |
| `start_date` | `NaiveDate` | Yes | Period start date |
| `end_date` | `NaiveDate` | Yes | Period end date |

**Validation Rules**:
- `start_date` must be before or equal to `end_date`
- `identifier` should match calendar naming conventions (not enforced by harness)

---

### TestInput

Defines the input dataset schema and data.

| Field | Type | Required | Description |
|-------|------|----------|-------------|
| `dataset` | `Dataset` | Yes | Dataset schema definition (reuses core entity) |
| `data` | `HashMap<String, DataBlock>` | Yes | Keyed by table logical name |

**Relationships**:
- References `Dataset` entity from core
- Contains `DataBlock` for each table

**Validation Rules**:
- All tables in `dataset` must have corresponding entries in `data`
- Extra entries in `data` (not in `dataset`) are ignored with warning

---

### DataBlock

Defines test data for a single table (inline or file reference).

| Field | Type | Required | Description |
|-------|------|----------|-------------|
| `rows` | `Vec<HashMap<String, Value>>` | No | Inline data rows (each map is column→value) |
| `file` | `String` | No | Path to external data file (CSV, Parquet) |

**Constraints**:
- Exactly one of `rows` or `file` must be present
- `rows`: Each HashMap represents one row; keys are column names (business columns only, no `_` prefix)
- `file`: Path is relative to scenario YAML file location

---

### ProjectDef

Polymorphic type for inline project or reference.

**Variants**:

1. **Inline Project**:
   - Contains full `Project` entity structure
   - Used during engine development to define test projects directly

2. **ProjectRef** (reference):
   - `id`: `Uuid` — Project ID
   - `version`: `i32` — Project version number
   - Used for contract tests against existing deployed projects

**Validation Rules**:
- For `ProjectRef`: Referenced project must exist in metadata store (deferred to execution time)
- Version drift produces warning, not error

---

### TestOutput

Defines expected output data.

| Field | Type | Required | Description |
|-------|------|----------|-------------|
| `data` | `DataBlock` | Yes | Expected output rows (inline or file reference) |

**Validation Rules**:
- Same as `DataBlock` (exactly one of `rows` or `file`)
- Schema is inferred from `project` output operation

---

### TestConfig

Controls test execution and comparison behavior.

| Field | Type | Required | Default | Description |
|-------|------|----------|---------|-------------|
| `match_mode` | `MatchMode` | No | `Exact` | Row matching strategy |
| `validate_metadata` | `bool` | No | `false` | Include system columns in comparison |
| `validate_traceability` | `bool` | No | `false` | Validate trace events |
| `snapshot_on_failure` | `bool` | No | `true` | Save actual output on failure |
| `order_sensitive` | `bool` | No | `false` | Require row order match |

**Enum: MatchMode**:
- `Exact`: All rows must match exactly; no extra rows allowed
- `Subset`: Expected rows must exist in actual; extra actual rows tolerated

---

### TraceAssertion

Defines expected trace event for validation.

| Field | Type | Required | Description |
|-------|------|----------|-------------|
| `operation_order` | `i32` | Yes | The operation that should produce this trace event |
| `change_type` | `ChangeType` | Yes | Type of change (created/updated/deleted) |
| `row_match` | `HashMap<String, Value>` | Yes | Column values identifying the row |
| `expected_diff` | `HashMap<String, Value>` | No | Expected column changes (for `updated` only) |

**Enum: ChangeType**:
- `Created`
- `Updated`
- `Deleted`

---

### TestResult

Output of test execution.

| Field | Type | Required | Description |
|-------|------|----------|-------------|
| `scenario_name` | `String` | Yes | Name from TestScenario |
| `status` | `TestStatus` | Yes | Pass/Fail/Error |
| `warnings` | `Vec<String>` | Yes | Non-fatal warnings (e.g., version drift) |
| `data_mismatches` | `Vec<DataMismatch>` | Yes | Row-level mismatches (empty on pass) |
| `trace_mismatches` | `Vec<TraceMismatch>` | Yes | Trace assertion mismatches |
| `error` | `Option<ErrorDetail>` | No | Present only when status is Error |
| `actual_snapshot` | `Option<DataBlock>` | No | Actual output (when `snapshot_on_failure=true` and test fails) |

**Enum: TestStatus**:
- `Pass`: All assertions passed
- `Fail`: Data or trace mismatches found
- `Error`: Execution failed (parse error, execution exception, etc.)

**Relationships**:
- Contains zero or more `DataMismatch`
- Contains zero or more `TraceMismatch`
- Contains optional `ErrorDetail`

---

### DataMismatch

Represents a single data validation failure.

| Field | Type | Required | Description |
|-------|------|----------|-------------|
| `mismatch_type` | `MismatchType` | Yes | Type of mismatch |
| `expected` | `HashMap<String, Value>` | No | Expected row values (for missing_row, value_mismatch) |
| `actual` | `HashMap<String, Value>` | No | Actual row values (for extra_row, value_mismatch) |
| `differing_columns` | `Vec<String>` | No | Columns that differ (for value_mismatch only) |

**Enum: MismatchType**:
- `MissingRow`: Expected row not found in actual output
- `ExtraRow`: Actual row not in expected output (only reported in Exact mode)
- `ValueMismatch`: Row found but column values differ

---

### TraceMismatch

Represents a trace validation failure.

| Field | Type | Required | Description |
|-------|------|----------|-------------|
| `operation_order` | `i32` | Yes | Operation where assertion failed |
| `mismatch_type` | `TraceMismatchType` | Yes | Type of trace mismatch |
| `expected` | `TraceAssertion` | Yes | The expected trace assertion |
| `actual` | `Option<TraceEvent>` | No | The actual trace event (if found) |

**Enum: TraceMismatchType**:
- `MissingEvent`: Expected trace event not found
- `ExtraEvent`: Unexpected trace event found
- `DiffMismatch`: Trace event found but diff values incorrect

---

### ErrorDetail

Execution error information.

| Field | Type | Required | Description |
|-------|------|----------|-------------|
| `error_type` | `ErrorType` | Yes | Category of error |
| `message` | `String` | Yes | Human-readable error message |
| `details` | `Option<String>` | No | Additional technical details (stack trace, etc.) |

**Enum: ErrorType**:
- `ParseError`: YAML parsing failure
- `SchemaValidationError`: Dataset schema invalid
- `ExecutionError`: Pipeline execution failure
- `FileNotFound`: Data file reference not found
- `ProjectNotFound`: ProjectRef resolution failure

---

## Metadata Injection Schema

System columns injected into each row of test data:

| Column Name | Type | Required | Source | Description |
|-------------|------|----------|--------|-------------|
| `_row_id` | `Uuid` | Yes | Generated (v7) | Unique row identifier |
| `_deleted` | `bool` | Yes | Hardcoded `false` | Deletion flag |
| `_created_at` | `DateTime<Utc>` | Yes | `Utc::now()` | Creation timestamp |
| `_updated_at` | `DateTime<Utc>` | Yes | `Utc::now()` | Update timestamp |
| `_source_dataset_id` | `Uuid` | Yes | From `TestInput.dataset.id` | Source dataset ID |
| `_source_table` | `String` | Yes | Table name | Source table name |
| `_period` | `String` | Conditional | User-provided in row | Period identifier (if `temporal_mode=Period`) |
| `_period_from` | `String` | Conditional | User-provided in row | Period range start (if `temporal_mode=Bitemporal`) |
| `_period_to` | `String` | Conditional | User-provided in row | Period range end (if `temporal_mode=Bitemporal`) |

**Injection Rules**:
- System columns (`_*`) are injected by harness; users provide business columns only
- Temporal columns (`_period`, `_period_from/_period_to`) are provided by user in input rows based on table's `temporal_mode`
- All injected columns are stripped from comparison unless `validate_metadata=true`

---

## Entity Relationships Diagram

```
TestScenario
├── periods: Vec<PeriodDef>
├── input: TestInput
│   ├── dataset: Dataset (from core)
│   └── data: HashMap<String, DataBlock>
│       └── DataBlock (rows XOR file)
├── project: ProjectDef
│   ├── Inline(Project)
│   └── Ref { id, version }
├── expected_output: TestOutput
│   └── data: DataBlock
├── expected_trace: Vec<TraceAssertion>
└── config: TestConfig

TestResult
├── status: TestStatus (Pass/Fail/Error)
├── warnings: Vec<String>
├── data_mismatches: Vec<DataMismatch>
│   └── DataMismatch { type, expected, actual, differing_columns }
├── trace_mismatches: Vec<TraceMismatch>
│   └── TraceMismatch { operation_order, type, expected, actual }
├── error: Option<ErrorDetail>
└── actual_snapshot: Option<DataBlock>
```

---

## Serialization Format (YAML)

All entities support serde serialization to/from YAML:

```yaml
# TestScenario example
name: "Passthrough Test"
periods:
  - identifier: "2026-01"
    level: "month"
    start_date: "2026-01-01"
    end_date: "2026-01-31"

input:
  dataset:
    main_table:
      name: simple
      temporal_mode: period
      columns:
        - { name: id, type: integer, nullable: false }
        - { name: value, type: decimal }
  data:
    simple:
      rows:
        - { id: 1, value: 100.0, _period: "2026-01" }
        - { id: 2, value: 200.0, _period: "2026-01" }

project:
  name: "passthrough"
  materialization: eager
  operations:
    - { order: 1, type: output, parameters: { destination: default } }

expected_output:
  data:
    rows:
      - { id: 1, value: 100.0 }
      - { id: 2, value: 200.0 }

config:
  match_mode: exact
  validate_metadata: false
  validate_traceability: false
  snapshot_on_failure: true
```

---

## State Transitions

**TestStatus State Machine**:

```
[Initial]
   │
   ├──[Parse Success]──► [Execute Pipeline]
   │                        │
   │                        ├──[Execution Success]──► [Compare Output]
   │                        │                            │
   │                        │                            ├──[Match]──► Pass
   │                        │                            └──[Mismatch]──► Fail
   │                        │
   │                        └──[Execution Failure]──► Error
   │
   └──[Parse Failure]──► Error
```

---

## Implementation Notes

1. **Reuse Existing Entities**: `Dataset`, `Project`, `Operation`, `Period`, `TraceEvent` are already defined in `core/src/model` — reuse them directly in TestScenario

2. **New Entities Location**: Place new entities in `core/src/model/test_scenario.rs`:
   - `TestScenario`, `TestInput`, `TestOutput`, `TestConfig`, `TestResult`
   - `DataMismatch`, `TraceMismatch`, `TraceAssertion`
   - `DataBlock`, `PeriodDef`, `ProjectDef`

3. **Serde Attributes**:
   - Use `#[serde(default)]` for optional fields with defaults (e.g., `TestConfig` fields)
   - Use `#[serde(rename_all = "snake_case")]` for enum variants
   - Use `#[serde(tag = "type")]` for `ProjectDef` enum if inline vs ref distinction needed in YAML

4. **Validation**: Implement `validate()` method on `TestScenario` for post-deserialization checks (DataBlock one-of constraint, period non-empty, etc.)
