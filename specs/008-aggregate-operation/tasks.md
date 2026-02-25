# Tasks: Aggregate Operation

**Input**: Design documents from `/specs/001-aggregate-operation/` (reference artifacts)
**Prerequisites**: plan.md ✓ (template only), spec.md (using 001 spec as reference), research.md (using 001 research), data-model.md (using 001 data-model), contracts/ (using 001 contracts)

**Note**: This feature (008-aggregate-operation) shares the same specification as 001-aggregate-operation. Design artifacts from 001 are used as reference for task generation.

**Tests**: Per constitutional principle I (TDD), all tasks include tests written BEFORE implementation. Tests are MANDATORY and NON-NEGOTIABLE.

**Organization**: Tasks are grouped by user story to enable independent implementation and testing of each story.

## Format: `- [ ] [ID] [P?] [Story] Description`

- **[P]**: Can run in parallel (different files, no dependencies)
- **[US#]**: Which user story this task belongs to (e.g., US1, US2, US3)
- Include exact file paths in descriptions

## Path Conventions

Per existing Cargo workspace structure:
- Core implementation: `crates/core/src/engine/ops/` (new directory)
- Model definitions: `crates/core/src/model/operation.rs` (exists)
- Unit tests: `crates/core/tests/unit/`
- Integration tests: `crates/core/tests/integration/`
- Contract tests: `crates/core/tests/contracts/`

---

## Phase 1: Setup (Shared Infrastructure)

**Purpose**: Project initialization and basic structure verification

- [X] T001 Verify Cargo workspace structure exists at /workspace with members: core, api-server, engine-worker, cli, test-resolver
- [X] T002 Verify Polars 0.46 dependency in workspace Cargo.toml
- [X] T003 [P] Verify uuid v7 feature enabled in workspace Cargo.toml
- [X] T004 [P] Verify chrono dependency with serde and clock features in workspace Cargo.toml
- [X] T005 [P] Verify serde, serde_json dependencies in workspace Cargo.toml

---

## Phase 2: Foundational (Blocking Prerequisites)

**Purpose**: Core infrastructure that MUST be complete before ANY user story can be implemented

**⚠️ CRITICAL**: No user story work can begin until this phase is complete

- [X] T006 Verify OperationKind::Aggregate enum variant exists in crates/core/src/model/operation.rs
- [X] T007 Create ops module directory at crates/core/src/engine/ops/
- [X] T008 Create mod.rs for ops module at crates/core/src/engine/ops/mod.rs
- [X] T009 Create aggregate module file at crates/core/src/engine/ops/aggregate.rs
- [X] T010 Export aggregate module from crates/core/src/engine/ops/mod.rs
- [X] T011 Define AggregateOperation struct in crates/core/src/engine/ops/aggregate.rs
- [X] T012 Define Aggregation struct in crates/core/src/engine/ops/aggregate.rs
- [X] T013 Define AggregateError enum with variants (EmptyGroupBy, EmptyAggregations, DuplicateGroupBy, UnknownColumn, SystemColumnConflict, DuplicateAggregationColumn, ExecutionError) in crates/core/src/engine/ops/aggregate.rs
- [X] T014 Implement std::error::Error and Display for AggregateError in crates/core/src/engine/ops/aggregate.rs
- [X] T015 Create test directory structure at crates/core/tests/unit/, crates/core/tests/integration/, crates/core/tests/contracts/

**Checkpoint**: Foundation ready - user story implementation can now begin in parallel

---

## Phase 3: User Story 1 - Append Grouped Summary Rows (Priority: P1) 🎯 MVP

**Goal**: Group rows by specified columns, compute aggregate values, and append summary rows to working dataset without modifying existing rows

**Independent Test**: Execute a pipeline containing one aggregate operation and verify summary rows are added while existing rows remain unchanged

**Acceptance Criteria**:
1. One summary row appended per distinct group
2. All original rows remain present and unmodified
3. Aggregate functions (SUM, COUNT) work correctly

### Tests for User Story 1 (MANDATORY - TDD Principle I) ⚠️

> **NOTE: Write these tests FIRST, ensure they FAIL before implementation**

- [X] T016 [P] [US1] Contract test: deserialize AggregateOperation from JSON with single group-by column in crates/core/tests/contracts/us1_aggregate_contract.rs
- [X] T017 [P] [US1] Contract test: deserialize AggregateOperation from JSON with multiple group-by columns in crates/core/tests/contracts/us1_aggregate_contract.rs
- [X] T018 [P] [US1] Contract test: serialize AggregateOperation to JSON in crates/core/tests/contracts/us1_aggregate_contract.rs
- [X] T019 [P] [US1] Unit test: group by single column produces correct number of groups in crates/core/tests/unit/aggregate_basic_test.rs
- [X] T020 [P] [US1] Unit test: group by multiple columns produces correct combinations in crates/core/tests/unit/aggregate_basic_test.rs
- [X] T021 [P] [US1] Integration test: SUM aggregate function computes correct totals in crates/core/tests/integration/aggregate_execution_test.rs
- [X] T022 [P] [US1] Integration test: COUNT aggregate function computes correct row counts in crates/core/tests/integration/aggregate_execution_test.rs
- [X] T023 [P] [US1] Integration test: original rows remain unchanged after aggregation in crates/core/tests/integration/aggregate_execution_test.rs
- [X] T024 [P] [US1] Integration test: monthly totals by account type scenario (TS-05 from quickstart) in crates/core/tests/integration/aggregate_execution_test.rs

### Implementation for User Story 1

- [X] T025 [P] [US1] Implement serde Serialize for AggregateOperation in crates/core/src/engine/ops/aggregate.rs
- [X] T026 [P] [US1] Implement serde Deserialize for AggregateOperation in crates/core/src/engine/ops/aggregate.rs
- [X] T027 [P] [US1] Implement serde Serialize for Aggregation in crates/core/src/engine/ops/aggregate.rs
- [X] T028 [P] [US1] Implement serde Deserialize for Aggregation in crates/core/src/engine/ops/aggregate.rs
- [X] T029 [US1] Implement validate_aggregate_spec function (validate non-empty group_by and aggregations) in crates/core/src/engine/ops/aggregate.rs
- [X] T030 [US1] Implement helper: convert_group_by_to_polars_exprs function in crates/core/src/engine/ops/aggregate.rs
- [X] T031 [US1] Implement helper: convert_aggregations_to_polars_exprs function for SUM in crates/core/src/engine/ops/aggregate.rs
- [X] T032 [US1] Implement helper: convert_aggregations_to_polars_exprs function for COUNT in crates/core/src/engine/ops/aggregate.rs
- [X] T033 [US1] Implement execute_aggregate function skeleton with signature in crates/core/src/engine/ops/aggregate.rs
- [X] T034 [US1] Implement selector filtering logic in execute_aggregate in crates/core/src/engine/ops/aggregate.rs
- [X] T035 [US1] Implement Polars group_by operation in execute_aggregate in crates/core/src/engine/ops/aggregate.rs
- [X] T036 [US1] Implement Polars agg operation for aggregations in execute_aggregate in crates/core/src/engine/ops/aggregate.rs
- [X] T037 [US1] Implement summary row concatenation (append to original dataset) in execute_aggregate in crates/core/src/engine/ops/aggregate.rs
- [X] T038 [US1] Verify all US1 tests pass (T016-T024)

**Checkpoint**: At this point, User Story 1 should be fully functional and testable independently - basic aggregation with row preservation works

---

## Phase 4: User Story 2 - Produce Consistent Summary Row Shape (Priority: P2)

**Goal**: Ensure appended summary rows follow the working dataset schema with grouped values, aggregated values, system metadata, and nulls for non-produced columns

**Independent Test**: Run an aggregate operation and verify summary rows contain grouped and aggregated values, while all non-produced columns are present with null values

**Acceptance Criteria**:
1. Non-grouped, non-aggregated columns are null on summary rows
2. Summary rows include all required system metadata fields (_row_id, _created_at, _updated_at, _source_dataset_id, _source_table, _deleted, _period)
3. Summary rows marked as not deleted (_deleted = false)
4. All aggregate functions (AVG, MIN_AGG, MAX_AGG) work correctly

### Tests for User Story 2 (MANDATORY - TDD Principle I) ⚠️

> **NOTE: Write these tests FIRST, ensure they FAIL before implementation**

- [X] T039 [P] [US2] Unit test: verify non-aggregated columns are null on summary rows in crates/core/tests/unit/aggregate_schema_test.rs
- [X] T040 [P] [US2] Unit test: verify _row_id is unique UUID v7 on summary rows in crates/core/tests/unit/aggregate_schema_test.rs
- [X] T041 [P] [US2] Unit test: verify _created_at and _updated_at are execution timestamp in crates/core/tests/unit/aggregate_schema_test.rs
- [X] T042 [P] [US2] Unit test: verify _deleted is false on summary rows in crates/core/tests/unit/aggregate_schema_test.rs
- [X] T043 [P] [US2] Unit test: verify _source_dataset_id and _source_table from context in crates/core/tests/unit/aggregate_schema_test.rs
- [X] T044 [P] [US2] Integration test: AVG aggregate function computes correct averages in crates/core/tests/integration/aggregate_execution_test.rs
- [X] T045 [P] [US2] Integration test: MIN_AGG aggregate function computes correct minimums in crates/core/tests/integration/aggregate_execution_test.rs
- [X] T046 [P] [US2] Integration test: MAX_AGG aggregate function computes correct maximums in crates/core/tests/integration/aggregate_execution_test.rs
- [X] T047 [P] [US2] Integration test: complete summary row schema consistency (all columns present) in crates/core/tests/integration/aggregate_execution_test.rs

### Implementation for User Story 2

- [X] T048 [US2] Implement identify_non_aggregated_columns helper function in crates/core/src/engine/ops/aggregate.rs
- [X] T049 [US2] Implement add_null_columns_for_non_aggregated function in crates/core/src/engine/ops/aggregate.rs
- [X] T050 [US2] Implement generate_row_ids function (UUID v7 generation) in crates/core/src/engine/ops/aggregate.rs
- [X] T051 [US2] Implement add_system_metadata function skeleton in crates/core/src/engine/ops/aggregate.rs
- [X] T052 [US2] Implement _row_id population in add_system_metadata in crates/core/src/engine/ops/aggregate.rs
- [X] T053 [US2] Implement _created_at and _updated_at population in add_system_metadata in crates/core/src/engine/ops/aggregate.rs
- [X] T054 [US2] Implement _source_dataset_id and _source_table population in add_system_metadata in crates/core/src/engine/ops/aggregate.rs
- [X] T055 [US2] Implement _deleted = false population in add_system_metadata in crates/core/src/engine/ops/aggregate.rs
- [X] T056 [US2] Implement _period column handling (from group-by if present, else null) in add_system_metadata in crates/core/src/engine/ops/aggregate.rs
- [X] T057 [US2] Implement AVG aggregate function in convert_aggregations_to_polars_exprs in crates/core/src/engine/ops/aggregate.rs
- [X] T058 [US2] Implement MIN_AGG aggregate function in convert_aggregations_to_polars_exprs in crates/core/src/engine/ops/aggregate.rs
- [X] T059 [US2] Implement MAX_AGG aggregate function in convert_aggregations_to_polars_exprs in crates/core/src/engine/ops/aggregate.rs
- [X] T060 [US2] Integrate null columns and system metadata into execute_aggregate in crates/core/src/engine/ops/aggregate.rs
- [X] T061 [US2] Verify all US2 tests pass (T039-T047)

**Checkpoint**: At this point, User Stories 1 AND 2 should both work independently - summary rows have correct schema with metadata and nulls

---

## Phase 5: User Story 3 - Handle Invalid Aggregate Definitions Early (Priority: P3)

**Goal**: Validate aggregate configurations before execution and fail with explicit feedback for invalid definitions

**Independent Test**: Submit invalid aggregate definitions (unknown columns, invalid expressions, empty lists) and confirm execution is blocked with explicit validation feedback

**Acceptance Criteria**:
1. Unknown group-by columns fail validation with clear error (UnknownColumn)
2. Empty group_by list fails validation (EmptyGroupBy)
3. Empty aggregations list fails validation (EmptyAggregations)
4. Duplicate group-by columns rejected (DuplicateGroupBy)
5. System column conflicts rejected (SystemColumnConflict)
6. Duplicate aggregation output columns rejected (DuplicateAggregationColumn)

### Tests for User Story 3 (MANDATORY - TDD Principle I) ⚠️

> **NOTE: Write these tests FIRST, ensure they FAIL before implementation**

- [X] T062 [P] [US3] Unit test: empty group_by returns EmptyGroupBy error in crates/core/tests/unit/aggregate_validation_test.rs
- [X] T063 [P] [US3] Unit test: empty aggregations returns EmptyAggregations error in crates/core/tests/unit/aggregate_validation_test.rs
- [X] T064 [P] [US3] Unit test: duplicate group_by column returns DuplicateGroupBy error in crates/core/tests/unit/aggregate_validation_test.rs
- [X] T065 [P] [US3] Unit test: unknown group_by column returns UnknownColumn error in crates/core/tests/unit/aggregate_validation_test.rs
- [X] T066 [P] [US3] Unit test: system column conflict (_row_id) returns SystemColumnConflict error in crates/core/tests/unit/aggregate_validation_test.rs
- [X] T067 [P] [US3] Unit test: system column conflict (_deleted) returns SystemColumnConflict error in crates/core/tests/unit/aggregate_validation_test.rs
- [X] T068 [P] [US3] Unit test: duplicate aggregation output column returns DuplicateAggregationColumn error in crates/core/tests/unit/aggregate_validation_test.rs
- [X] T069 [P] [US3] Integration test: invalid aggregate blocks execution before processing in crates/core/tests/integration/aggregate_execution_test.rs

### Implementation for User Story 3

- [X] T070 [P] [US3] Implement check_empty_group_by validation in validate_aggregate_spec in crates/core/src/engine/ops/aggregate.rs
- [X] T071 [P] [US3] Implement check_empty_aggregations validation in validate_aggregate_spec in crates/core/src/engine/ops/aggregate.rs
- [X] T072 [P] [US3] Implement check_duplicate_group_by validation in validate_aggregate_spec in crates/core/src/engine/ops/aggregate.rs
- [X] T073 [US3] Implement validate_aggregate_compile function for schema validation in crates/core/src/engine/ops/aggregate.rs
- [X] T074 [US3] Implement check_unknown_group_by_columns in validate_aggregate_compile in crates/core/src/engine/ops/aggregate.rs
- [X] T075 [US3] Implement check_system_column_conflicts for aggregation output columns in validate_aggregate_compile in crates/core/src/engine/ops/aggregate.rs
- [X] T076 [US3] Implement check_duplicate_aggregation_columns in validate_aggregate_compile in crates/core/src/engine/ops/aggregate.rs
- [X] T077 [US3] Integrate parse-time validation (validate_aggregate_spec) into execute_aggregate entry point in crates/core/src/engine/ops/aggregate.rs
- [X] T078 [US3] Integrate compile-time validation (validate_aggregate_compile) into execute_aggregate entry point in crates/core/src/engine/ops/aggregate.rs
- [X] T079 [US3] Verify all US3 tests pass (T062-T069)

**Checkpoint**: At this point, all three user stories should work independently - validation prevents invalid configurations

---

## Phase 6: Polish & Cross-Cutting Concerns

**Purpose**: Final integration, documentation, and quality improvements that span multiple user stories

- [X] T080 [P] Add comprehensive documentation to AggregateOperation struct in crates/core/src/engine/ops/aggregate.rs
- [X] T081 [P] Add comprehensive documentation to Aggregation struct in crates/core/src/engine/ops/aggregate.rs
- [X] T082 [P] Add comprehensive documentation to execute_aggregate function in crates/core/src/engine/ops/aggregate.rs
- [X] T083 [P] Add error context messages to all AggregateError variants in crates/core/src/engine/ops/aggregate.rs
- [X] T084 Run cargo fmt on all modified files in crates/core/
- [X] T085 Run cargo clippy on crates/core and fix all warnings
- [X] T086 Run full test suite (cargo test) and verify 100% pass rate
- [X] T087 [P] Add integration test for edge case: selector filters all rows (zero summary rows) in crates/core/tests/integration/aggregate_edge_cases_test.rs
- [X] T088 [P] Add integration test for edge case: null values in group-by columns in crates/core/tests/integration/aggregate_edge_cases_test.rs
- [X] T089 [P] Add integration test for edge case: null values in aggregated columns in crates/core/tests/integration/aggregate_edge_cases_test.rs
- [X] T090 Update crates/core/src/engine/mod.rs to export ops module
- [X] T091 Verify no compiler warnings (cargo build --all-features)
- [X] T092 Final verification: all tests pass (cargo test --all-features)

---

## Dependencies & Execution Strategy

### User Story Completion Order

```
Setup (Phase 1) → Foundational (Phase 2)
                        ↓
        ┌───────────────┼───────────────┐
        ↓               ↓               ↓
      US1 (P1)        US2 (P2)        US3 (P3)
    (T016-T038)     (T039-T061)     (T062-T079)
        └───────────────┼───────────────┘
                        ↓
                   Polish (Phase 6)
                    (T080-T092)
```

**Dependencies**:
- Phase 1 (Setup): No dependencies
- Phase 2 (Foundational): Requires Phase 1 complete
- Phase 3 (US1): Requires Phase 2 complete
- Phase 4 (US2): Requires Phase 2 complete (independent of US1)
- Phase 5 (US3): Requires Phase 2 complete (independent of US1 and US2)
- Phase 6 (Polish): Requires US1, US2, US3 complete

**Parallel Opportunities**:
- Within Phase 1: T003, T004, T005 can run in parallel
- Within Phase 2: T015 can run in parallel with T006-T014
- Within Phase 3 (US1 tests): T016-T024 are all parallelizable (different test files or test cases)
- Within Phase 3 (US1 impl): T025-T028 can run in parallel (serde implementations)
- Within Phase 4 (US2 tests): T039-T047 are all parallelizable
- Within Phase 5 (US3 tests): T062-T069 are all parallelizable
- Within Phase 5 (US3 impl): T070-T072 can run in parallel (independent validation checks)
- Within Phase 6: T080-T083, T087-T089 can run in parallel
- **Cross-story parallelism**: After Phase 2 completes, US1, US2, and US3 can be worked on in parallel by different developers

### MVP Recommendation

**Minimum Viable Product**: User Story 1 (Phase 3) only
- Delivers core business value: aggregation with row preservation
- Supports SUM and COUNT functions
- 23 tasks (T016-T038)
- Estimated effort: 3-5 days for experienced Rust developer

**Incremental Delivery Path**:
1. **MVP (US1)**: Basic aggregation functionality
2. **MVP + Schema Consistency (US1 + US2)**: Production-ready summary rows
3. **Full Feature (US1 + US2 + US3)**: Robust validation and all aggregate functions

---

## Implementation Notes

### Test-Driven Development (TDD) Process

1. **Red**: Write failing test first (T016-T024 for US1, etc.)
2. **Green**: Implement minimum code to make test pass (T025-T037 for US1)
3. **Refactor**: Clean up implementation while keeping tests green
4. **Repeat**: Move to next test

### File Creation Order

1. Core structures first: `crates/core/src/engine/ops/aggregate.rs` with basic types
2. Test files: Create test files and write failing tests
3. Implementation: Fill in logic to make tests pass
4. Documentation: Add comprehensive docs after tests pass

### Validation Strategy

- **Parse-time validation**: Structural checks (non-empty, no duplicates) - runs before execution
- **Compile-time validation**: Schema checks (column existence, conflicts) - runs after schema resolution
- **Runtime validation**: Type checks and expression evaluation - runs during execution

### Error Handling

All validation errors must:
- Use AggregateError enum variants
- Include context (column names, operation details)
- Fail fast (don't attempt execution after validation failure)
- Return Result<T, AggregateError> from all fallible functions

---

## Summary

- **Total Tasks**: 92
- **Setup Tasks**: 5 (T001-T005)
- **Foundational Tasks**: 10 (T006-T015)
- **User Story 1 Tasks**: 23 (T016-T038) - MVP
- **User Story 2 Tasks**: 23 (T039-T061)
- **User Story 3 Tasks**: 18 (T062-T079)
- **Polish Tasks**: 13 (T080-T092)

**Parallel Opportunities**: 45+ tasks marked with [P] can run in parallel with other tasks

**Independent Test Criteria**:
- US1: Run integration test T024 (monthly totals scenario) - should produce correct summary rows
- US2: Run integration test T047 (schema consistency) - should have all metadata fields
- US3: Run integration test T069 (validation blocks execution) - should fail before processing

**Suggested MVP Scope**: User Story 1 only (T001-T038) - delivers core aggregation functionality

**Format Validation**: ✅ All tasks follow checklist format with checkbox, ID, labels, file paths
