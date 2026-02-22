# Capability: Execute Test Scenario

**Status**: Draft  
**Created**: 2026-02-22  
**Domain**: Quality Assurance / Development Infrastructure

## Definition

Execute Test Scenario is the capability that loads a self-contained, data-driven test definition (a single YAML file), provisions the input data with injected system metadata, executes a Project against it using a built-in test Resolver, and compares the actual output to the expected output — collecting all mismatches into a structured diff report. The harness serves two audiences: the development team (using it to define what operations should produce, driving the engine's own development) and end-users (using it to create contract test scenarios that must pass before a new version of the system can be deployed to production).

## Purpose & Role

Without this capability, there is no automated way to verify that the computation engine produces correct results for a given input. By making test scenarios declarative, data-driven, and self-contained, the system enables:

- **Engine development**: developers define expected operation outputs before implementing them, using the test harness as the primary feedback loop.
- **End-user contract testing**: users author test scenarios that encode business expectations; these become deployment gates — a new system version cannot ship unless all contract scenarios pass.
- **Regression detection**: changes to the engine, operations, or Resolver logic are validated against the full suite of scenarios.

---

## Inputs

| Input | Type | Required | Constraints | Description |
|---|---|---|---|---|
| `scenario_file` | `FilePath` | Yes | Must be a valid YAML file conforming to the TestScenario schema | Path to the test scenario definition |

### TestScenario (YAML structure)

| Field | Type | Required | Constraints | Description |
|---|---|---|---|---|
| `name` | `String` | Yes | Non-empty | Human-readable scenario name |
| `description` | `String` | No | — | Narrative description of what is being tested |
| `periods` | `List<PeriodDef>` | Yes | At least one | The Period(s) the Run will execute against |
| `input` | `TestInput` | Yes | — | Input Dataset definition with sample data |
| `project` | `ProjectDef \| ProjectRef` | Yes | Inline definition or reference to existing Project by `id + version` | The Project to execute |
| `expected_output` | `TestOutput` | Yes | — | Expected result data for comparison |
| `expected_trace` | `List<TraceAssertion>` | No | — | Expected trace events to compare against actual trace records |
| `config` | `TestConfig` | Yes | — | Controls comparison behaviour |

#### PeriodDef

| Field | Type | Required | Description |
|---|---|---|---|
| `identifier` | `String` | Yes | Period identifier (e.g., `"2026-01"`) |
| `level` | `String` | Yes | Calendar level name (e.g., `"month"`) |
| `start_date` | `Date` | Yes | Period start date |
| `end_date` | `Date` | Yes | Period end date |

#### TestInput

| Field | Type | Required | Description |
|---|---|---|---|
| `dataset` | `DatasetSchema` | Yes | Dataset schema definition (must conform to Dataset entity structure — `main_table`, `lookups`, `temporal_mode` on each TableRef) |
| `data` | `Map<String, DataBlock>` | Yes | Keyed by table logical name; each value is a `DataBlock` |

#### DataBlock

| Field | Type | Required | Description |
|---|---|---|---|
| `rows` | `List<Map<String, Any>>` | No | Inline data rows (each map is column→value). Use for small datasets |
| `file` | `FilePath` | No | Path to external data file (CSV, Parquet). Use for large datasets |

> Exactly one of `rows` or `file` must be present.

#### TestOutput

| Field | Type | Required | Description |
|---|---|---|---|
| `data` | `DataBlock` | Yes | Expected output rows — inline or file reference |

#### TraceAssertion

| Field | Type | Required | Description |
|---|---|---|---|
| `operation_order` | `Integer` | Yes | The operation that should have produced this trace event |
| `change_type` | `Enum` | Yes | `created \| updated \| deleted` |
| `row_match` | `Map<String, Any>` | Yes | Column values that identify the row (subset match against natural key or `_row_id`) |
| `expected_diff` | `Map<String, Any>` | No | Expected column changes (for `updated` events — old/new values) |

#### TestConfig

| Field | Type | Required | Default | Description |
|---|---|---|---|---|
| `match_mode` | `Enum` | No | `exact` | `exact`: all rows must match, no extras. `subset`: expected rows must exist, extra rows tolerated |
| `validate_metadata` | `Boolean` | No | `false` | When true, system columns (`_row_id`, `_created_at`, etc.) are included in comparison |
| `validate_traceability` | `Boolean` | No | `false` | When true, `expected_trace` assertions are evaluated |
| `snapshot_on_failure` | `Boolean` | No | `true` | When true, the actual output is saved for inspection on test failure |
| `order_sensitive` | `Boolean` | No | `false` | When true, row order matters in comparison |

---

## Outputs

| Output | Type | Description |
|---|---|---|
| `result` | `TestResult` | Pass/Fail verdict with structured diff on failure |
| `actual_snapshot` | `DataBlock` | The actual output data; produced only when `snapshot_on_failure: true` and the test fails |

### TestResult

| Field | Type | Description |
|---|---|---|
| `scenario_name` | `String` | Name of the scenario that was executed |
| `status` | `Enum` | `pass \| fail \| error` |
| `warnings` | `List<String>` | Non-fatal warnings (e.g., Project version drift on `ProjectRef` contracts) |
| `data_mismatches` | `List<DataMismatch>` | Row-level mismatches between expected and actual output; empty on pass |
| `trace_mismatches` | `List<TraceMismatch>` | Trace assertion mismatches; empty when traceability validation is off or all assertions pass |
| `error` | `ErrorDetail` | Present only when `status: error`; describes execution failure (not a comparison failure) |

### DataMismatch

| Field | Type | Description |
|---|---|---|
| `type` | `Enum` | `missing_row \| extra_row \| value_mismatch` |
| `expected` | `Map<String, Any>` | Expected row values (for `missing_row` and `value_mismatch`) |
| `actual` | `Map<String, Any>` | Actual row values (for `extra_row` and `value_mismatch`) |
| `columns` | `List<String>` | Columns that differ (for `value_mismatch` only) |

### TraceMismatch

| Field | Type | Description |
|---|---|---|
| `operation_order` | `Integer` | The operation where the trace assertion failed |
| `type` | `Enum` | `missing_event \| extra_event \| diff_mismatch` |
| `expected` | `TraceAssertion` | The expected trace assertion |
| `actual` | `Map` | The actual trace event (if found) |

---

## Trigger

Invoked explicitly by a user or CI/CD pipeline. Two usage patterns:

1. **Developer mode**: run a single scenario file during development as a feedback loop.
2. **Contract suite mode**: run all scenario files discovered by convention (`tests/scenarios/**/*.yaml`) or specified via CLI arguments (explicit file paths or directories). Each scenario produces an independent `TestResult`.

---

## Preconditions

- **PRE-001**: The scenario YAML file must exist and parse without errors.
- **PRE-002**: When using `ProjectRef`, the referenced Project must exist at the specified version.
- **PRE-003**: Data files referenced in `DataBlock.file` must exist and be readable.
- **PRE-004**: The `input.dataset` schema must be valid (conform to Dataset entity rules — at least one column per TableRef, no `_` prefix on user columns, etc.).

---

## Postconditions

- **POST-001**: A `TestResult` is produced for every executed scenario.
- **POST-002**: No production data is written — all output goes to the built-in test Resolver's ephemeral storage.
- **POST-003**: If `snapshot_on_failure: true` and the test fails, the actual output is persisted for inspection.
- **POST-004**: The system state is unchanged — no Datasets, Projects, or Runs are created in the production registry as a side effect of test execution.

---

## Execution Flow

1. **Parse** the scenario YAML and validate its structure.
2. **Provision input data**: for each table in `input.data`, inject system metadata columns (`_row_id`, `_period` or `_period_from/_period_to` based on the table's `temporal_mode`, `_deleted: false`, `_created_at`, etc.). Users provide only business columns.
3. **Register with test Resolver**: the built-in test Resolver is configured to serve the provisioned data when the engine requests it by table name and Period.
4. **Assemble the Project**: if inline, use as-is. If a `ProjectRef`, load the referenced Project. Override the Resolver to use the built-in test Resolver for all Datasets in the scenario.
5. **Create a synthetic Run** for the specified Period(s) and execute the Project pipeline.
6. **Capture output**: collect the `output` operation results.
7. **Compare**: strip system columns from both expected and actual output (unless `validate_metadata: true`). Apply `match_mode` (exact or subset). Collect all mismatches.
8. **Compare trace** (if `validate_traceability: true`): match `expected_trace` assertions against actual trace events from the Run.
9. **Produce result**: assemble `TestResult`. If failed and `snapshot_on_failure: true`, persist actual output as `actual_snapshot`.

---

## Built-in Test Resolver

The harness provides a dedicated Resolver that:

- Serves data exclusively from the scenario's `input.data` block (inline rows or referenced files).
- Maps table logical names + Period identifiers to the provisioned test data.
- Is automatically injected as the Resolver for every Dataset in the scenario — no user configuration needed.
- Can be overridden per-table in the scenario if the user needs custom resolution behaviour (e.g., testing Resolver rules themselves).
- Does not persist beyond the test execution.

---

## Error Cases

| Error | Trigger Condition | Handling |
|---|---|---|
| `ScenarioParseError` | YAML is malformed or does not conform to the TestScenario schema | `status: error`; parsing errors listed in `error.detail` |
| `SchemaValidationError` | Input Dataset schema violates Dataset entity rules | `status: error`; validation failures listed |
| `ProjectNotFound` | `ProjectRef` references a non-existent Project or version | `status: error` |
| `DataFileNotFound` | A `DataBlock.file` path does not exist or is unreadable | `status: error` |
| `ExecutionFailure` | The Project pipeline fails during execution (expression error, type mismatch, etc.) | `status: error`; the Run's error detail is included |
| `ComparisonFailure` | Output does not match expected data or trace assertions fail | `status: fail`; all mismatches collected in `data_mismatches` and `trace_mismatches` |

---

## Boundaries

- This capability does NOT create persistent entities (Datasets, Projects, Runs) in the production system — everything is ephemeral.
- This capability does NOT validate intermediate operation states — only the final output and optionally trace events.
- This capability does NOT test Resolver configuration — it replaces the Resolver with a built-in test Resolver (unless explicitly overridden in the scenario).
- This capability does NOT manage test suite orchestration (parallel execution, retry, reporting dashboards) — that is the responsibility of the CI/CD pipeline or a future test-runner capability.
- This capability does NOT generate test data — the user provides sample input and expected output.

---

## Dependencies

| Dependency | Type | Description |
|---|---|---|
| Dataset | Entity | Input schema conforms to the Dataset entity model |
| Project | Entity | The recipe being tested — inline or by reference |
| Operation | Entity | Operations within the Project are executed by the engine |
| Run | Entity | A synthetic Run is created to execute the pipeline |
| Resolver | Entity | The built-in test Resolver replaces production Resolvers |
| Execute Project Calculation | Capability | The engine that actually runs the Project pipeline |
| Trace Run Execution | Capability | Produces trace events compared against `expected_trace` |

---

## Open Questions

All initial open questions have been resolved. Decisions are reflected in the sections above.

| # | Question | Resolution |
|---|---|---|
| OQ-001 | Should the harness support parameterised scenarios? | **No.** One scenario file = one execution. Use multiple entries in `periods` for multi-period runs, or create separate scenario files for distinct parameterisations. |
| OQ-002 | How are test scenario files discovered for contract suite mode? | **Convention default + CLI override.** Default convention: `tests/scenarios/**/*.yaml`. CLI may override with explicit file paths or directories. |
| OQ-003 | Should the test result include timing information? | **Deferred.** Not included initially, but the `TestResult` structure should not preclude adding `timing` fields later. |
| OQ-004 | For contract tests with `ProjectRef`, should the harness enforce version pinning? | **Warning only.** The harness detects when the referenced Project version differs from the current version and emits a warning in the `TestResult`, but does not fail the test. |

---

## Serialization (YAML DSL)

### Annotated Example

```yaml
# test-scenario: Verify regional discount calculation
name: "Regional Discount Calculation"
description: "Validates that EMEA customers with gold tier get 10% discount on order amount"

periods:
  - identifier: "2026-01"
    level: "month"
    start_date: "2026-01-01"
    end_date: "2026-01-31"

input:
  dataset:
    main_table:
      name: orders
      temporal_mode: period
      columns:
        - name: order_number
          type: string
          nullable: false
        - name: customer_id
          type: string
          nullable: false
        - name: amount
          type: decimal
        - name: region
          type: string
    lookups: []
  data:
    orders:
      rows:
        - { order_number: "ORD-001", customer_id: "C1", amount: 100.00, region: "EMEA", _period: "2026-01" }
        - { order_number: "ORD-002", customer_id: "C2", amount: 200.00, region: "APAC", _period: "2026-01" }
        - { order_number: "ORD-003", customer_id: "C3", amount: 150.00, region: "EMEA", _period: "2026-01" }

# Inline project — used during engine development
project:
  name: "Discount Calculator"
  materialization: eager
  selectors:
    EMEA_ONLY: "region = \"EMEA\""
  operations:
    - order: 1
      type: update
      alias: apply_discount
      parameters:
        selector: "{{EMEA_ONLY}}"
        joins:
          - alias: customers
            source:
              dataset_id: "ds-customers-test"
            on: "orders.customer_id = customers.id"
        assignments:
          - column: amount
            expression: "IF(customers.tier = \"gold\", orders.amount * 0.9, orders.amount)"
    - order: 2
      type: output
      parameters:
        destination: default

expected_output:
  data:
    rows:
      - { order_number: "ORD-001", customer_id: "C1", amount: 90.00, region: "EMEA" }   # gold tier → 10% off
      - { order_number: "ORD-002", customer_id: "C2", amount: 200.00, region: "APAC" }  # not EMEA → unchanged
      - { order_number: "ORD-003", customer_id: "C3", amount: 150.00, region: "EMEA" }  # EMEA but silver → unchanged

config:
  match_mode: exact
  validate_metadata: false
  validate_traceability: false
  snapshot_on_failure: true
  order_sensitive: false
```
