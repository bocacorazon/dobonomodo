# Feature Specification: Test Harness

**Feature Branch**: `003-test-harness`  
**Created**: 2026-02-22  
**Status**: Draft  
**Input**: User description: "Build the data-driven test harness from docs/specs/S02-test-harness/prompt.md"

## User Scenarios & Testing *(mandatory)*

### User Story 1 - Execute Single Test Scenario (Priority: P1)

As a platform developer, I need to execute a single YAML test scenario file that provisions test data, runs a Project pipeline, and validates output so I can verify pipeline correctness with automated testing.

**Why this priority**: This is the core testing capability. Without it, no pipeline validation can occur, blocking all development verification workflows.

**Independent Test**: Can be fully tested by creating a passthrough scenario YAML file, running `dobo test <scenario.yaml>`, and confirming the test passes with correct metadata injection and output comparison.

**Acceptance Scenarios**:

1. **Given** a valid YAML test scenario file, **When** I run `dobo test <scenario.yaml>`, **Then** the system parses the scenario, injects system metadata, executes the pipeline, and reports pass/fail status.
2. **Given** a test scenario with expected output, **When** the actual output matches exactly, **Then** the test reports "PASS" with no mismatches.
3. **Given** a test scenario with mismatched output, **When** the actual output differs from expected, **Then** the test reports "FAIL" with specific data mismatches listed (missing rows, extra rows, value differences).

---

### User Story 2 - Validate with Multiple Match Modes (Priority: P2)

As a test author, I need to choose between exact and subset match modes so I can validate strict equality or allow flexible output validation depending on test intent.

**Why this priority**: Different testing scenarios require different validation strategies. Exact matching validates complete correctness, while subset matching allows for flexible validation of core requirements.

**Independent Test**: Can be tested by creating two scenarios (one with `match_mode: exact`, one with `match_mode: subset`) and verifying behavior: exact mode fails on extra rows, subset mode allows them.

**Acceptance Scenarios**:

1. **Given** a test scenario with `match_mode: exact`, **When** actual output contains extra rows beyond expected, **Then** the test fails with "extra_row" mismatches.
2. **Given** a test scenario with `match_mode: subset`, **When** actual output contains extra rows beyond expected, **Then** the test passes as long as all expected rows are present.
3. **Given** either match mode, **When** expected rows are missing from actual output, **Then** the test fails with "missing_row" mismatches.

---

### User Story 3 - Validate Trace Events (Priority: P3)

As a platform developer, I need to validate that pipeline execution produces expected trace events so I can verify traceability and lineage tracking work correctly.

**Why this priority**: Trace validation is important for system observability but secondary to basic functional correctness. It can be validated once core execution is proven.

**Independent Test**: Can be tested by creating a scenario with `validate_traceability: true` and `trace_assertions`, running the test, and confirming trace events match expectations.

**Acceptance Scenarios**:

1. **Given** a test scenario with `validate_traceability: true`, **When** the pipeline produces trace events matching all assertions, **Then** the test passes with no trace mismatches.
2. **Given** a test scenario with trace assertions, **When** expected trace events are missing or differ, **Then** the test fails with specific trace mismatches listed.

---

### User Story 4 - Execute Test Suite (Priority: P3)

As a platform developer, I need to execute all test scenarios in a directory so I can run comprehensive regression testing with a single command.

**Why this priority**: Suite execution is a quality-of-life improvement that builds on single-scenario execution. It's valuable but not required for initial testing capability.

**Independent Test**: Can be tested by creating a directory with multiple YAML scenario files, running `dobo test --suite <dir>`, and verifying all scenarios execute with aggregated pass/fail reporting.

**Acceptance Scenarios**:

1. **Given** a directory containing multiple valid test scenario files, **When** I run `dobo test --suite <dir>`, **Then** all scenarios execute and report individual pass/fail status.
2. **Given** a test suite with mixed pass/fail scenarios, **When** execution completes, **Then** the system reports total scenarios, passed count, and failed count.

### Edge Cases

- What happens when a YAML file is malformed? → Parser must report clear error with file location and syntax issue.
- What happens when temporal_mode requires period columns but data rows omit them? → Metadata injection must fail with clear error.
- What happens when expected output schema doesn't match actual output schema? → Comparison must report schema mismatch as test error.
- What happens when system metadata validation is enabled but metadata is incorrect? → Test must fail with specific metadata mismatches.
- What happens when ProjectRef version drifts from current? → System must emit warning but not fail the test.
- What happens when pipeline execution throws an error? → Test must report "ERROR" status with exception details.

## Requirements *(mandatory)*

### Functional Requirements

- **FR-001**: System MUST parse YAML test scenario files into `TestScenario` structs using serde deserialization.
- **FR-002**: System MUST inject system metadata columns (`_row_id`, `_deleted`, `_created_at`, `_updated_at`, `_source_dataset_id`, `_source_table`) into all test data rows.
- **FR-003**: System MUST inject temporal columns (`_period` or `_period_from`/`_period_to`) based on table's `temporal_mode` configuration.
- **FR-004**: System MUST generate UUIDs v7 for `_row_id` values during metadata injection.
- **FR-005**: System MUST provide `InMemoryDataLoader` implementing `DataLoader` trait to serve test data as Polars `LazyFrame`.
- **FR-006**: System MUST support inline data rows (YAML list of maps) and file reference data blocks (CSV/Parquet).
- **FR-007**: System MUST execute Project pipelines via `core::engine` integration point.
- **FR-008**: System MUST compare actual output to expected output using configured match mode (exact or subset).
- **FR-009**: System MUST strip system columns from comparison by default unless `validate_metadata: true`.
- **FR-010**: System MUST support order-insensitive row matching by default and order-sensitive matching when `order_sensitive: true`.
- **FR-011**: System MUST collect ALL data mismatches (missing rows, extra rows, value mismatches) without stopping on first failure.
- **FR-012**: System MUST report specific differing columns for each value mismatch.
- **FR-013**: System MUST validate trace events against `TraceAssertion` entries when `validate_traceability: true`.
- **FR-014**: System MUST assemble `TestResult` with status (pass/fail/error), warnings, data_mismatches, and trace_mismatches.
- **FR-015**: System MUST persist actual output snapshot when test fails and `snapshot_on_failure: true`.
- **FR-016**: System MUST provide `InMemoryMetadataStore` and `InMemoryTraceWriter` for test isolation.
- **FR-017**: System MUST support CLI command `dobo test <scenario.yaml>` to execute single scenario.
- **FR-018**: System MUST support CLI command `dobo test --suite <dir>` to execute all scenarios in directory.
- **FR-019**: System MUST emit warning (not failure) when ProjectRef version drifts from current version.
- **FR-020**: System MUST discover test scenarios using `tests/scenarios/**/*.yaml` convention by default.

### Key Entities *(include if feature involves data)*

- **TestScenario**: Complete test definition including periods, input dataset/data, project configuration, expected output, trace assertions, and test config.
- **TestConfig**: Test behavior configuration including match_mode, validate_metadata, validate_traceability, order_sensitive, snapshot_on_failure.
- **TestResult**: Test execution outcome with status, warnings, data_mismatches, trace_mismatches, and optional actual_snapshot.
- **DataMismatch**: Specific data validation failure (missing_row, extra_row, value_mismatch with differing columns).
- **TraceMismatch**: Specific trace validation failure (missing trace event or incorrect trace data).
- **InMemoryDataLoader**: Test-specific DataLoader implementation serving data from DataBlock (inline or file reference).
- **InMemoryMetadataStore**: Test-specific MetadataStore implementation for test isolation.
- **InMemoryTraceWriter**: Test-specific TraceWriter implementation for test isolation.

## Assumptions

- This feature depends on **S00** (Workspace Scaffold) for entity model structs and IO traits.
- Pipeline execution logic (S10) is stubbed for now; test harness will use passthrough or mock until S10 is available.
- Production IO adapters (S16), trace event generation (S12), and metadata stores (S17) are out of scope; test harness provides in-memory implementations.
- Suite-level aggregation (total scenarios, pass/fail counts) is implemented as basic CLI output, not persistent reporting.
- YAML is the only supported test scenario format; JSON scenarios are not required.

## Success Criteria *(mandatory)*

### Measurable Outcomes

- **SC-001**: 100% of valid YAML test scenario files parse successfully into `TestScenario` structs.
- **SC-002**: 100% of test data rows receive all required system metadata columns with correct types and values.
- **SC-003**: `InMemoryDataLoader` serves test data as valid `LazyFrame` with schema matching table definition.
- **SC-004**: Exact match mode correctly detects and reports missing rows, extra rows, and value mismatches.
- **SC-005**: Subset match mode allows extra rows while still detecting missing rows and value mismatches.
- **SC-006**: `TestResult` accurately reports pass/fail/error status for all test scenarios.
- **SC-007**: Diff reports list specific columns that differ for each mismatched row.
- **SC-008**: The passthrough scenario from the spec executes end-to-end and passes once pipeline executor (S10) is available.
- **SC-009**: Test suite execution reports individual and aggregated pass/fail counts correctly.
- **SC-010**: Malformed YAML scenarios produce clear error messages identifying the issue.
