# S02: Test Harness

## Feature
Build the data-driven test harness that loads YAML test scenario files, provisions input data with injected system metadata, executes a Project pipeline via `core::engine`, compares actual output to expected output, and produces structured diff reports.

## Context
- Read: `docs/capabilities/execute-test-scenario.md` (full capability definition — TestScenario schema, TestConfig, TestResult, DataMismatch, TraceMismatch, execution flow, built-in test Resolver)
- Read: `docs/architecture/sample-datasets.md` (sample data and test scenario catalogue TS-01 through TS-11)
- Read: `docs/entities/dataset.md` (system columns, `temporal_mode`, `ColumnDef`)
- Read: `docs/architecture/system-architecture.md` (`cli` and `test-resolver` crate responsibilities)

## Scope

### In Scope
- `test-resolver` crate: `InMemoryDataLoader` implementing `DataLoader` trait — serves data from `DataBlock` (inline rows or file reference) as Polars `LazyFrame`
- YAML scenario parser: deserialize `TestScenario` from YAML using serde
- Metadata injection: for each table's data rows, auto-generate `_row_id` (UUID v7), `_deleted` (false), `_created_at`, `_updated_at`, `_source_dataset_id`, `_source_table`, and temporal columns (`_period` or `_period_from`/`_period_to`) based on `temporal_mode`
- Output comparison engine:
  - Strip system columns from comparison (unless `validate_metadata: true`)
  - `match_mode: exact` — row-for-row match (order-insensitive by default; order-sensitive if `order_sensitive: true`)
  - `match_mode: subset` — expected rows must exist in actual; extra rows tolerated
  - Collect all `DataMismatch` entries: `missing_row`, `extra_row`, `value_mismatch` (with differing column list)
- Trace comparison: match `TraceAssertion` entries against actual `TraceEvent`s when `validate_traceability: true`
- `TestResult` assembly: `pass`/`fail`/`error` with `warnings`, `data_mismatches`, `trace_mismatches`
- `actual_snapshot` persistence on failure when `snapshot_on_failure: true`
- `InMemoryMetadataStore` and `InMemoryTraceWriter` for test isolation
- CLI integration point: `dobo test <scenario.yaml>` and `dobo test --suite <dir>` (command parsing only — actual CLI binary is S20)

### Out of Scope
- Pipeline execution logic (stubbed — calls `core::engine` which is built in S10; for now use a passthrough or mock)
- Production IO adapters (S16)
- Trace event generation (S12) — the harness compares trace events but doesn't generate them
- Suite-level aggregation (deferred)

## Dependencies
- **S00** (Workspace Scaffold): entity model structs, IO traits

## Parallel Opportunities
This spec can run in parallel with **S01** (DSL Parser), **S16** (DataSource Adapters), **S17** (Metadata Store).

## Key Design Decisions
- One scenario per YAML file
- Data rows can be inline (list of maps) or file references (CSV/Parquet)
- System columns are injected by the harness — users provide only business columns
- Built-in test Resolver is auto-injected; no user config needed
- Comparison collects ALL mismatches (does not fail on first)
- ProjectRef version drift produces a warning, not a failure
- Suite discovery: `tests/scenarios/**/*.yaml` convention, overridable via CLI args

## Sample Test Scenario (for harness self-testing)

```yaml
name: "Harness self-test — passthrough"
periods:
  - { identifier: "2026-01", level: "month", start_date: "2026-01-01", end_date: "2026-01-31" }
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

## Success Criteria
- YAML scenario file parses correctly into `TestScenario` struct
- Metadata injection adds all required system columns with correct types
- `InMemoryDataLoader` serves test data as `LazyFrame` with correct schema
- Exact match mode detects missing rows, extra rows, and value mismatches
- Subset match mode allows extra rows
- `TestResult` correctly reports pass/fail/error
- Diff report lists specific columns that differ per mismatched row
- Passthrough scenario (above) passes end-to-end once pipeline executor (S10) is available
