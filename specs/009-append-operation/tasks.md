# Tasks: Append Operation

**Input**: Design documents from `/workspace/specs/009-append-operation/`
**Prerequisites**: plan.md ✓, spec.md ✓, research.md ✓, data-model.md ✓, contracts/ ✓, quickstart.md ✓

**Tests**: Per constitutional principle I (TDD), all tasks MUST include tests written BEFORE implementation. Tests are MANDATORY and NON-NEGOTIABLE.

**Organization**: Tasks are grouped by user story to enable independent implementation and testing of each story.

## Format: `- [X] T### [P?] [US#?] Description with file path`

- **[P]**: Can run in parallel (different files, no dependencies)
- **[US#]**: Which user story this task belongs to (US1, US2, US3, US4)
- Include exact file paths in descriptions

## Path Conventions

Based on plan.md, this is a Rust workspace:
- Core library: `crates/core/src/`
- Tests: `crates/core/tests/`
- Contract tests: `crates/core/tests/contracts/`
- Integration tests: `crates/core/tests/integration/`

---

## Phase 1: Setup (Shared Infrastructure)

**Purpose**: Project initialization and basic structure for append operation

- [X] T001 Review existing operation execution framework in crates/core/src/engine/
- [X] T002 Review existing resolver pattern in crates/core/src/resolver/
- [X] T003 [P] Review existing Expression model in crates/core/src/model/expression.rs
- [X] T004 [P] Review existing Dataset model in crates/core/src/model/dataset.rs for TemporalMode usage

---

## Phase 2: Foundational (Blocking Prerequisites)

**Purpose**: Core data structures and parsing that ALL user stories depend on

**⚠️ CRITICAL**: No user story work can begin until this phase is complete

- [X] T005 Define AppendOperation struct in crates/core/src/model/operation.rs
- [X] T006 [P] Define DatasetRef struct in crates/core/src/model/operation.rs
- [X] T007 [P] Define AppendAggregation struct in crates/core/src/model/operation.rs
- [X] T008 [P] Define Aggregation struct in crates/core/src/model/operation.rs
- [X] T009 Implement Serde deserialization for AppendOperation from YAML/JSON in crates/core/src/model/operation.rs
- [X] T010 Implement parse_aggregation() function for SUM/COUNT/AVG/MIN_AGG/MAX_AGG in crates/core/src/dsl/aggregation.rs
- [X] T011 Implement parse_source_selector() function for filter expressions in crates/core/src/dsl/expression.rs
- [X] T012 Create AppendError enum in crates/core/src/engine/error.rs with variants: DatasetNotFound, DatasetVersionNotFound, ColumnMismatch, ExpressionParseError, AggregationError, ColumnNotFound, ResolverNotFound, DataLoadError

**Checkpoint**: Foundation ready - user story implementation can now begin in parallel

---

## Phase 3: User Story 1 - Basic Budget vs Actual Comparison (Priority: P1) 🎯 MVP

**Goal**: Append budget rows into the working dataset alongside actual transactions for side-by-side analysis. This is the fundamental capability to merge datasets.

**Independent Test**: Load a transactions dataset, append a budget dataset with matching columns, verify both transaction and budget rows exist in the output.

### Tests for User Story 1 (MANDATORY - TDD Principle I) ⚠️

> **NOTE: Write these tests FIRST, ensure they FAIL before implementation**

- [X] T013 [P] [US1] Contract test: deserialize simple append operation from YAML in crates/core/tests/contracts/us009_append_operation_contract_test.rs
- [X] T014 [P] [US1] Contract test: deserialize simple append operation from JSON in crates/core/tests/contracts/us009_append_operation_contract_test.rs
- [X] T015 [P] [US1] Contract test: validate AppendOperation parameters in crates/core/tests/contracts/us009_append_operation_contract_test.rs
- [X] T016 [P] [US1] Integration test TS-01: append 4 budget rows to 10 transaction rows yielding 14 total rows in crates/core/tests/integration/us009_append_scenarios_test.rs
- [X] T017 [P] [US1] Integration test TS-02: verify budget rows with subset columns are appended successfully in crates/core/tests/integration/us009_append_scenarios_test.rs
- [X] T018 [P] [US1] Integration test TS-03: verify missing columns filled with NULL in appended budget rows in crates/core/tests/integration/us009_append_scenarios_test.rs

### Implementation for User Story 1

- [X] T019 [US1] Implement resolve_and_load_source() with resolver precedence (Project overrides → Dataset resolver → system default) in crates/core/src/engine/append.rs
- [X] T020 [US1] Implement validate_append_operation() for dataset existence and version validation in crates/core/src/engine/append.rs
- [X] T021 [US1] Implement align_appended_schema() with two-phase validation (fail on extra columns, fill missing with NULL) in crates/core/src/engine/append.rs
- [X] T022 [US1] Implement add_system_columns() to generate _row_id (UUID v7), _source_dataset, _operation_seq, _deleted in crates/core/src/engine/append.rs
- [X] T023 [US1] Implement execute_append() core logic for simple append (no filter, no aggregation) in crates/core/src/engine/append.rs
- [X] T024 [US1] Wire AppendOperation into operation execution pipeline in crates/core/src/engine/executor.rs
- [X] T025 [US1] Add error handling for DatasetNotFound and ColumnMismatch in crates/core/src/engine/append.rs
- [X] T026 [US1] Add logging for append operation (source dataset, row counts) in crates/core/src/engine/append.rs

**Checkpoint**: At this point, User Story 1 should be fully functional - simple append with column alignment works independently

---

## Phase 4: User Story 2 - Filtered Source Data Append (Priority: P2)

**Goal**: Append only specific budget rows (e.g., only "original" budget type) from the budget dataset for selective data integration.

**Independent Test**: Append a budget dataset with a source_selector that filters to specific budget types, verify only matching budget rows are appended.

### Tests for User Story 2 (MANDATORY - TDD Principle I) ⚠️

- [X] T027 [P] [US2] Contract test: deserialize append operation with source_selector from YAML in crates/core/tests/contracts/us009_append_operation_contract_test.rs
- [X] T028 [P] [US2] Integration test TS-06: append only 4 "original" budget rows from 12 total using source_selector "budget_type = 'original'" in crates/core/tests/integration/us009_append_scenarios_test.rs
- [X] T029 [P] [US2] Integration test TS-07: filter by numeric comparison "amount > 10000" in crates/core/tests/integration/us009_append_scenarios_test.rs
- [X] T030 [P] [US2] Integration test TS-08: highly selective filter matching 5 of 100 rows in crates/core/tests/integration/us009_append_scenarios_test.rs
- [X] T031 [P] [US2] Unit test: parse_source_selector() for various expression formats in crates/core/tests/unit/dsl_expression_test.rs
- [X] T032 [P] [US2] Edge case test: source_selector matches zero rows yields success with 0 appended in crates/core/tests/integration/us009_append_scenarios_test.rs

### Implementation for User Story 2

- [X] T033 [US2] Implement apply_source_selector() to filter DataFrame using parsed expression in crates/core/src/engine/append.rs
- [X] T034 [US2] Integrate source_selector filtering into execute_append() before aggregation in crates/core/src/engine/append.rs
- [X] T035 [US2] Implement zero-row append handling (success with zero appended) in crates/core/src/engine/append.rs
- [X] T036 [US2] Add validation for source_selector column existence in source dataset in crates/core/src/engine/append.rs
- [X] T037 [US2] Add error handling for ExpressionParseError in crates/core/src/engine/append.rs
- [X] T038 [US2] Add logging for filtered row counts (before/after filter) in crates/core/src/engine/append.rs

**Checkpoint**: At this point, User Stories 1 AND 2 should both work independently - filtering source data works

---

## Phase 5: User Story 4 - Period-Filtered Source Data (Priority: P2)

**Goal**: Source dataset rows are automatically filtered by the run period according to the source dataset's temporal_mode for temporally consistent data.

**Independent Test**: Create a run for period "2026-01", append a source dataset with temporal_mode: period containing rows for multiple periods, verify only "2026-01" rows are appended.

**Note**: This is US4 but prioritized before US3 based on spec.md priority order (both P2, but US4 addresses critical temporal consistency)

### Tests for User Story 4 (MANDATORY - TDD Principle I) ⚠️

- [X] T039 [P] [US4] Integration test TS-16: temporal_mode=period filters to _period = "2026-01" only in crates/core/tests/integration/us009_temporal_filtering_test.rs
- [X] T040 [P] [US4] Integration test TS-17: temporal_mode=bitemporal filters by asOf date (2026-01-01) in crates/core/tests/integration/us009_temporal_filtering_test.rs
- [X] T041 [P] [US4] Integration test TS-18: temporal_mode=snapshot appends all rows without filtering in crates/core/tests/integration/us009_temporal_filtering_test.rs
- [X] T042 [P] [US4] Unit test: apply_temporal_filter() for period mode in crates/core/tests/unit/engine_temporal_test.rs
- [X] T043 [P] [US4] Unit test: apply_temporal_filter() for bitemporal mode in crates/core/tests/unit/engine_temporal_test.rs

### Implementation for User Story 4

- [X] T044 [US4] Implement apply_temporal_filter() for period mode (_period = run_period) in crates/core/src/engine/temporal.rs
- [X] T045 [US4] Implement apply_temporal_filter() for bitemporal mode (valid_from/valid_to asOf query) in crates/core/src/engine/temporal.rs
- [X] T046 [US4] Implement apply_temporal_filter() for snapshot mode (no filtering) in crates/core/src/engine/temporal.rs
- [X] T047 [US4] Integrate temporal filtering into resolve_and_load_source() after data load in crates/core/src/engine/append.rs
- [X] T048 [US4] Add validation for bitemporal mode requiring asOf date in crates/core/src/engine/append.rs
- [X] T049 [US4] Add logging for temporal filtering (mode, row counts) in crates/core/src/engine/append.rs

**Checkpoint**: At this point, User Stories 1, 2, and 4 should all work independently - temporal filtering ensures period consistency

---

## Phase 6: User Story 3 - Aggregated Data Append (Priority: P3)

**Goal**: Append pre-aggregated summary rows from a source dataset (e.g., monthly totals by account) for hierarchical reporting combining detail and summary data.

**Independent Test**: Append a source dataset with aggregation configured (group_by + aggregations), verify the appended rows contain aggregated values rather than raw source rows.

### Tests for User Story 3 (MANDATORY - TDD Principle I) ⚠️

- [X] T050 [P] [US3] Contract test: deserialize append operation with aggregation from YAML in crates/core/tests/contracts/us009_append_operation_contract_test.rs
- [X] T051 [P] [US3] Contract test: validate aggregation expressions match pattern in crates/core/tests/contracts/us009_append_operation_contract_test.rs
- [X] T052 [P] [US3] Integration test TS-13: aggregate 12 budget rows by account_code yielding one row per account with SUM(amount) in crates/core/tests/integration/us009_append_scenarios_test.rs
- [X] T053 [P] [US3] Integration test TS-14: aggregate 100 rows across 5 accounts with SUM and COUNT in crates/core/tests/integration/us009_append_scenarios_test.rs
- [X] T054 [P] [US3] Integration test TS-15: source_selector filters 50 of 100 rows BEFORE aggregation in crates/core/tests/integration/us009_append_scenarios_test.rs
- [X] T055 [P] [US3] Unit test: parse_aggregation() for SUM/COUNT/AVG/MIN_AGG/MAX_AGG in crates/core/tests/unit/dsl_aggregation_test.rs
- [X] T056 [P] [US3] Unit test: build_agg_expressions() from AppendAggregation config in crates/core/tests/unit/engine_aggregation_test.rs

### Implementation for User Story 3

- [X] T057 [US3] Implement build_agg_expressions() to convert Aggregation list to Polars Expr in crates/core/src/engine/aggregation.rs
- [X] T058 [US3] Implement apply_aggregation() to execute group_by + aggregations on DataFrame in crates/core/src/engine/aggregation.rs
- [X] T059 [US3] Integrate aggregation into execute_append() AFTER source_selector filtering in crates/core/src/engine/append.rs
- [X] T060 [US3] Add validation for group_by columns existing in source dataset in crates/core/src/engine/append.rs
- [X] T061 [US3] Add validation for aggregation input columns existing in source dataset in crates/core/src/engine/append.rs
- [X] T062 [US3] Add validation for aggregation output columns existing in working dataset in crates/core/src/engine/append.rs
- [X] T063 [US3] Add error handling for AggregationError (invalid function, column not found) in crates/core/src/engine/append.rs
- [X] T064 [US3] Add logging for aggregation operations (group_by columns, agg count, output row count) in crates/core/src/engine/append.rs

**Checkpoint**: All user stories should now be independently functional - aggregation enables summary data alongside details

---

## Phase 7: Polish & Cross-Cutting Concerns

**Purpose**: Improvements that affect multiple user stories and final validation

- [X] T065 [P] Add comprehensive error messages with context to all AppendError variants in crates/core/src/engine/error.rs
- [X] T066 [P] Add performance benchmarks for 10k rows simple append (<10ms target) in crates/core/benches/append_benchmarks.rs
- [X] T067 [P] Add performance benchmarks for 100k rows aggregated append (<50ms target) in crates/core/benches/append_benchmarks.rs
- [X] T068 [P] Document AppendOperation API in crates/core/src/model/operation.rs with rustdoc
- [X] T069 [P] Document append execution functions in crates/core/src/engine/append.rs with rustdoc
- [X] T070 Run cargo fmt on workspace crates referenced by /workspace/Cargo.toml
- [X] T071 Run cargo clippy for workspace crates referenced by /workspace/Cargo.toml and fix warnings
- [X] T072 Run cargo test for workspace crates referenced by /workspace/Cargo.toml
- [X] T073 Validate quickstart scenarios in /workspace/specs/009-append-operation/quickstart.md end-to-end
- [X] T074 Update append operation documentation summary in /workspace/README.md
- [X] T075 [P] Add edge case test: non-existent dataset_id returns DatasetNotFound error in crates/core/tests/integration/us009_edge_cases_test.rs
- [X] T076 [P] Add edge case test: extra columns in source return ColumnMismatch error in crates/core/tests/integration/us009_edge_cases_test.rs
- [X] T077 [P] Add edge case test: invalid aggregate function returns AggregationError in crates/core/tests/integration/us009_edge_cases_test.rs
- [X] T078 [P] Add edge case test: group_by references non-existent column returns ColumnNotFound in crates/core/tests/integration/us009_edge_cases_test.rs

---

## Dependencies & Execution Order

### Phase Dependencies

- **Setup (Phase 1)**: No dependencies - can start immediately
- **Foundational (Phase 2)**: Depends on Setup completion - BLOCKS all user stories
- **User Stories (Phase 3-6)**: All depend on Foundational phase completion
  - User stories can then proceed in parallel (if staffed)
  - Or sequentially in priority order (US1 P1 → US2 P2 → US4 P2 → US3 P3)
- **Polish (Phase 7)**: Depends on all desired user stories being complete

### User Story Dependencies

- **User Story 1 (P1)**: Can start after Foundational (Phase 2) - No dependencies on other stories - FOUNDATIONAL for append capability
- **User Story 2 (P2)**: Can start after Foundational (Phase 2) - Independent of US1 but builds on execute_append() - Tests filtering logic
- **User Story 4 (P2)**: Can start after Foundational (Phase 2) - Independent of US1/US2 - Tests temporal filtering (critical for period consistency)
- **User Story 3 (P3)**: Can start after Foundational (Phase 2) - Independent of other stories but uses source_selector from US2 conceptually - Tests aggregation

**Recommended Order**: US1 (core append) → US2 (filtering) → US4 (temporal) → US3 (aggregation)

### Within Each User Story

- Tests MUST be written and FAIL before implementation (TDD Principle I)
- Core execution functions before integration
- Validation before error handling
- Logging after core implementation
- Story complete before moving to next priority

### Parallel Opportunities

- **Phase 1 (Setup)**: All review tasks (T001-T004) can run in parallel
- **Phase 2 (Foundational)**: Struct definitions (T006-T008) can run in parallel after T005; parsing functions (T010-T011) can run in parallel
- **Within User Stories**: All test tasks marked [P] can be written in parallel; contract tests can be written in parallel with integration tests
- **Across User Stories**: Once Foundational phase completes, US1, US2, US3, US4 can all start in parallel (if team capacity allows)
- **Phase 7 (Polish)**: Documentation tasks (T066-T069), edge case tests (T075-T078) can run in parallel

---

## Parallel Example: User Story 1

```bash
# Launch all tests for User Story 1 together (MANDATORY per TDD principle):
# Contract tests (parallel):
Task T013: "Contract test: deserialize simple append operation from YAML"
Task T014: "Contract test: deserialize simple append operation from JSON"
Task T015: "Contract test: validate AppendOperation parameters"

# Integration tests (parallel):
Task T016: "Integration test TS-01: append 4 budget rows to 10 transaction rows"
Task T017: "Integration test TS-02: verify budget rows with subset columns"
Task T018: "Integration test TS-03: verify missing columns filled with NULL"

# After tests fail, implementation can proceed sequentially:
# T019 (resolve and load) → T020 (validate) → T021 (align schema) → 
# T022 (system columns) → T023 (execute core) → T024 (wire into pipeline) →
# T025 (error handling) → T026 (logging)
```

---

## Parallel Example: User Story 3

```bash
# Launch all tests for User Story 3 together:
# Contract tests (parallel):
Task T050: "Contract test: deserialize append operation with aggregation from YAML"
Task T051: "Contract test: validate aggregation expressions match pattern"

# Integration tests (parallel):
Task T052: "Integration test TS-13: aggregate 12 budget rows by account_code"
Task T053: "Integration test TS-14: aggregate 100 rows across 5 accounts"
Task T054: "Integration test TS-15: source_selector filters BEFORE aggregation"

# Unit tests (parallel):
Task T055: "Unit test: parse_aggregation() for all aggregate functions"
Task T056: "Unit test: build_agg_expressions() from AppendAggregation config"

# After tests fail, implementation:
Task T057: "build_agg_expressions()"
Task T058: "apply_aggregation()"
# Then remaining tasks sequentially
```

---

## Implementation Strategy

### MVP First (User Story 1 Only)

1. Complete Phase 1: Setup (T001-T004)
2. Complete Phase 2: Foundational (T005-T012) - CRITICAL - blocks all stories
3. Complete Phase 3: User Story 1 (T013-T026)
4. **STOP and VALIDATE**: Test User Story 1 independently
   - Load transactions dataset (10 rows)
   - Append budget dataset (4 rows)
   - Verify output has 14 rows with correct column alignment
5. Deploy/demo if ready - **This is the MVP!**

### Incremental Delivery

1. Complete Setup + Foundational → Foundation ready (T001-T012)
2. Add User Story 1 (T013-T026) → Test independently → Deploy/Demo (MVP - basic append works!)
3. Add User Story 2 (T027-T038) → Test independently → Deploy/Demo (filtering works!)
4. Add User Story 4 (T039-T049) → Test independently → Deploy/Demo (temporal filtering works!)
5. Add User Story 3 (T050-T064) → Test independently → Deploy/Demo (aggregation works!)
6. Add Polish (T065-T078) → Final validation → Release

Each story adds value without breaking previous stories.

### Parallel Team Strategy

With multiple developers:

1. Team completes Setup + Foundational together (T001-T012)
2. Once Foundational is done:
   - **Developer A**: User Story 1 (T013-T026) - Core append capability
   - **Developer B**: User Story 2 (T027-T038) - Filtering
   - **Developer C**: User Story 4 (T039-T049) - Temporal filtering
   - **Developer D**: User Story 3 (T050-T064) - Aggregation
3. Stories complete and integrate independently
4. Team collaborates on Polish (T065-T078)

---

## Success Metrics

### Total Task Count: 78 tasks

### Tasks per User Story:
- **Phase 1 (Setup)**: 4 tasks
- **Phase 2 (Foundational)**: 8 tasks (BLOCKS all user stories)
- **Phase 3 (User Story 1 - P1)**: 14 tasks (6 tests + 8 implementation)
- **Phase 4 (User Story 2 - P2)**: 12 tasks (6 tests + 6 implementation)
- **Phase 5 (User Story 4 - P2)**: 11 tasks (5 tests + 6 implementation)
- **Phase 6 (User Story 3 - P3)**: 15 tasks (7 tests + 8 implementation)
- **Phase 7 (Polish)**: 14 tasks

### Parallel Opportunities Identified:
- **Phase 1**: 2 parallel groups (4 tasks total can run in parallel)
- **Phase 2**: 3 parallel groups after initial struct definitions
- **User Story Tests**: Within each story, all test tasks marked [P] can run in parallel
- **Across User Stories**: All 4 user stories can start in parallel after Foundational phase (52 tasks can be distributed)
- **Phase 7**: 8 tasks can run in parallel

### Independent Test Criteria per Story:

**User Story 1 (P1 - MVP)**:
- ✅ Load 10 transaction rows + append 4 budget rows → 14 total rows
- ✅ Budget rows with subset columns successfully appended
- ✅ Missing columns (journal_id, description) filled with NULL
- ✅ System columns (_row_id, _source_dataset, _operation_seq, _deleted) populated correctly

**User Story 2 (P2)**:
- ✅ source_selector "budget_type = 'original'" filters 12 rows → 4 appended
- ✅ Numeric comparison "amount > 10000" filters correctly
- ✅ Highly selective filter (5 of 100) appends exactly 5 rows
- ✅ Zero-row filter succeeds with 0 appended

**User Story 4 (P2)**:
- ✅ temporal_mode=period filters to run_period "2026-01" only
- ✅ temporal_mode=bitemporal filters by asOf date (2026-01-01)
- ✅ temporal_mode=snapshot appends all rows (no filtering)

**User Story 3 (P3)**:
- ✅ Aggregate 12 rows by account_code → one row per account with SUM(amount)
- ✅ Aggregate 100 rows across 5 accounts → 5 summary rows with SUM + COUNT
- ✅ source_selector filters BEFORE aggregation (50 of 100 filtered → aggregate only 50)
- ✅ All aggregate functions work: SUM, COUNT, AVG, MIN_AGG, MAX_AGG

### Suggested MVP Scope:
**User Story 1 ONLY** (Phase 1 + Phase 2 + Phase 3 = 26 tasks)

This delivers the fundamental value: combining data from multiple datasets for side-by-side analysis. Users can append budget rows to transaction rows for budget vs actual comparisons.

**Incremental additions**:
- **MVP + US2**: Add filtering (38 tasks total) - enables selective data integration
- **MVP + US2 + US4**: Add temporal filtering (49 tasks total) - ensures period consistency
- **MVP + US2 + US4 + US3**: Full feature (64 tasks total) - enables hierarchical reporting
- **Full + Polish**: Complete feature (78 tasks) - production ready

---

## Format Validation

✅ **All tasks follow checklist format**:
- Checkbox: `- [X]` present on all tasks
- Task ID: Sequential T001-T078
- [P] marker: Applied only to parallelizable tasks (different files, no dependencies)
- [US#] label: Applied to all user story tasks (US1, US2, US3, US4); omitted for Setup/Foundational/Polish
- Description: Clear action with exact file path
- Examples:
  - ✅ `- [X] T013 [P] [US1] Contract test: deserialize simple append operation from YAML in crates/core/tests/contracts/us009_append_operation_contract_test.rs`
  - ✅ `- [X] T023 [US1] Implement execute_append() core logic for simple append (no filter, no aggregation) in crates/core/src/engine/append.rs`
  - ✅ `- [X] T005 Define AppendOperation struct in crates/core/src/model/operation.rs`

---

## Notes

- [P] tasks = different files, no dependencies
- [US#] label maps task to specific user story for traceability (US1, US2, US3, US4)
- Each user story is independently completable and testable
- Tests MUST fail before implementing (TDD Principle I - Constitutional requirement)
- Commit after each task or logical group
- Stop at any checkpoint to validate story independently
- Avoid: vague tasks, same file conflicts, cross-story dependencies that break independence
- Performance targets: Simple append <10ms, Filtered append <20ms, Aggregated append <50ms (for 100k rows)
