# Tasks: Aggregate Operation

**Input**: Design documents from `/specs/001-aggregate-operation/`
**Prerequisites**: plan.md ✓, spec.md ✓, research.md ✓, data-model.md ✓, contracts/ ✓

**Tests**: Per constitutional principle I (TDD), all tasks include tests written BEFORE implementation. Tests are MANDATORY and NON-NEGOTIABLE.

**Organization**: Tasks are grouped by user story to enable independent implementation and testing of each story.

## Format: `- [ ] [ID] [P?] [Story] Description`

- **[P]**: Can run in parallel (different files, no dependencies)
- **[US#]**: Which user story this task belongs to (e.g., US1, US2, US3)
- Include exact file paths in descriptions

## Path Conventions

Per plan.md, this is a Cargo workspace monorepo:
- Core implementation: `crates/core/src/engine/ops/aggregate.rs`
- Unit tests: `crates/core/tests/unit/aggregate_validation_test.rs`
- Integration tests: `crates/core/tests/integration/aggregate_execution_test.rs`
- Contract tests: `crates/core/tests/contracts/aggregate_contract_test.rs`

---

## Phase 1: Setup (Shared Infrastructure)

**Purpose**: Project initialization and basic structure

- [ ] T001 Verify Cargo workspace structure per plan.md in /workspace
- [ ] T002 Verify Polars 0.46 dependency in crates/core/Cargo.toml
- [ ] T003 [P] Verify uuid v7 feature enabled in workspace Cargo.toml
- [ ] T004 [P] Verify chrono dependency for timestamp generation in crates/core/Cargo.toml

---

## Phase 2: Foundational (Blocking Prerequisites)

**Purpose**: Core infrastructure that MUST be complete before ANY user story can be implemented

**⚠️ CRITICAL**: No user story work can begin until this phase is complete

- [ ] T005 Verify OperationKind::Aggregate exists in crates/core/src/model/operation.rs
- [ ] T006 Verify Expression wrapper exists in crates/core/src/model/expression.rs
- [ ] T007 Verify SchemaRef and dataset types exist in crates/core/src/engine/types.rs
- [ ] T008 Create module structure in crates/core/src/engine/ops/aggregate.rs
- [ ] T009 Define AggregateOperation struct in crates/core/src/engine/ops/aggregate.rs
- [ ] T010 Define Aggregation struct in crates/core/src/engine/ops/aggregate.rs
- [ ] T011 Define AggregateError enum with all error variants in crates/core/src/engine/ops/aggregate.rs

**Checkpoint**: Foundation ready - user story implementation can now begin in parallel

---

## Phase 3: User Story 1 - Append Grouped Summary Rows (Priority: P1) 🎯 MVP

**Goal**: Group rows by specified columns, compute aggregate values, and append summary rows to working dataset without modifying existing rows

**Independent Test**: Execute a pipeline containing one aggregate operation and verify summary rows are added while existing rows remain unchanged

**Acceptance Criteria**:
1. One summary row appended per distinct group
2. All original rows remain present and unmodified

### Tests for User Story 1 (MANDATORY - TDD Principle I) ⚠️

> **NOTE: Write these tests FIRST, ensure they FAIL before implementation**

- [ ] T012 [P] [US1] Unit test for basic aggregation execution (single group-by column) in crates/core/tests/unit/aggregate_validation_test.rs
- [ ] T013 [P] [US1] Unit test for multi-column group-by in crates/core/tests/unit/aggregate_validation_test.rs
- [ ] T014 [P] [US1] Integration test for SUM aggregate function in crates/core/tests/integration/aggregate_execution_test.rs
- [ ] T015 [P] [US1] Integration test for COUNT aggregate function in crates/core/tests/integration/aggregate_execution_test.rs
- [ ] T016 [P] [US1] Integration test verifying row preservation (original rows unchanged) in crates/core/tests/integration/aggregate_execution_test.rs
- [ ] T017 [P] [US1] Integration test for TS-05 scenario (monthly totals by account type) in crates/core/tests/integration/aggregate_execution_test.rs

### Implementation for User Story 1

- [ ] T018 [US1] Implement parse-time validation: validate_aggregate_spec function in crates/core/src/engine/ops/aggregate.rs
- [ ] T019 [US1] Implement compile-time validation: validate_aggregate_compile function in crates/core/src/engine/ops/aggregate.rs
- [ ] T020 [US1] Implement helper function to convert group_by columns to Polars expressions in crates/core/src/engine/ops/aggregate.rs
- [ ] T021 [US1] Implement helper function to convert aggregations to Polars agg expressions in crates/core/src/engine/ops/aggregate.rs
- [ ] T022 [US1] Implement core execute_aggregate function skeleton in crates/core/src/engine/ops/aggregate.rs
- [ ] T023 [US1] Implement selector filtering logic within execute_aggregate in crates/core/src/engine/ops/aggregate.rs
- [ ] T024 [US1] Implement Polars group_by and agg execution in crates/core/src/engine/ops/aggregate.rs
- [ ] T025 [US1] Implement summary row appending via concat in crates/core/src/engine/ops/aggregate.rs
- [ ] T026 [US1] Verify all US1 tests pass (T012-T017)

**Checkpoint**: At this point, User Story 1 should be fully functional and testable independently - basic aggregation with row preservation works

---

## Phase 4: User Story 2 - Produce Consistent Summary Row Shape (Priority: P2)

**Goal**: Ensure appended summary rows follow the working dataset schema with grouped values, aggregated values, system metadata, and nulls for non-produced columns

**Independent Test**: Run an aggregate operation and verify summary rows contain grouped and aggregated values, while all non-produced columns are present with null values

**Acceptance Criteria**:
1. Non-grouped, non-aggregated columns are null on summary rows
2. Summary rows include all required system metadata fields
3. Summary rows marked as not deleted (_deleted = false)

### Tests for User Story 2 (MANDATORY - TDD Principle I) ⚠️

- [ ] T027 [P] [US2] Unit test verifying non-aggregated columns are null on summary rows in crates/core/tests/unit/aggregate_validation_test.rs
- [ ] T028 [P] [US2] Unit test verifying system metadata fields on summary rows in crates/core/tests/unit/aggregate_validation_test.rs
- [ ] T029 [P] [US2] Integration test for AVG aggregate function in crates/core/tests/integration/aggregate_execution_test.rs
- [ ] T030 [P] [US2] Integration test for MIN_AGG aggregate function in crates/core/tests/integration/aggregate_execution_test.rs
- [ ] T031 [P] [US2] Integration test for MAX_AGG aggregate function in crates/core/tests/integration/aggregate_execution_test.rs
- [ ] T032 [P] [US2] Integration test verifying complete summary row schema consistency in crates/core/tests/integration/aggregate_execution_test.rs

### Implementation for User Story 2

- [ ] T033 [US2] Implement function to identify non-aggregated columns in crates/core/src/engine/ops/aggregate.rs
- [ ] T034 [US2] Implement function to add null columns for non-aggregated business columns in crates/core/src/engine/ops/aggregate.rs
- [ ] T035 [US2] Implement system metadata population (_row_id with UUID v7) in crates/core/src/engine/ops/aggregate.rs
- [ ] T036 [US2] Implement system metadata population (_created_at, _updated_at with execution timestamp) in crates/core/src/engine/ops/aggregate.rs
- [ ] T037 [US2] Implement system metadata population (_source_dataset_id, _source_table from context) in crates/core/src/engine/ops/aggregate.rs
- [ ] T038 [US2] Implement system metadata population (_deleted = false) in crates/core/src/engine/ops/aggregate.rs
- [ ] T039 [US2] Handle _period column (from group-by if present, else null) in crates/core/src/engine/ops/aggregate.rs
- [ ] T040 [US2] Integrate metadata and null column logic into execute_aggregate in crates/core/src/engine/ops/aggregate.rs
- [ ] T041 [US2] Verify all US2 tests pass (T027-T032)

**Checkpoint**: At this point, User Stories 1 AND 2 should both work independently - summary rows have correct schema with metadata and nulls

---

## Phase 5: User Story 3 - Handle Invalid Aggregate Definitions Early (Priority: P3)

**Goal**: Validate aggregate configurations before execution and fail with explicit feedback for invalid definitions

**Independent Test**: Submit invalid aggregate definitions (unknown columns, invalid expressions, empty lists) and confirm execution is blocked with explicit validation feedback

**Acceptance Criteria**:
1. Unknown group-by columns fail validation with clear error
2. Empty aggregation lists fail validation before execution
3. Duplicate group-by columns rejected
4. System column conflicts rejected

### Tests for User Story 3 (MANDATORY - TDD Principle I) ⚠️

- [ ] T042 [P] [US3] Unit test for empty group_by validation (EmptyGroupBy error) in crates/core/tests/unit/aggregate_validation_test.rs
- [ ] T043 [P] [US3] Unit test for empty aggregations validation (EmptyAggregations error) in crates/core/tests/unit/aggregate_validation_test.rs
- [ ] T044 [P] [US3] Unit test for duplicate group_by column validation in crates/core/tests/unit/aggregate_validation_test.rs
- [ ] T045 [P] [US3] Unit test for unknown group_by column validation in crates/core/tests/unit/aggregate_validation_test.rs
- [ ] T046 [P] [US3] Unit test for system column conflict validation in crates/core/tests/unit/aggregate_validation_test.rs
- [ ] T047 [P] [US3] Unit test for duplicate aggregation output column validation in crates/core/tests/unit/aggregate_validation_test.rs

### Implementation for User Story 3

- [ ] T048 [US3] Implement empty group_by validation in validate_aggregate_spec in crates/core/src/engine/ops/aggregate.rs
- [ ] T049 [US3] Implement empty aggregations validation in validate_aggregate_spec in crates/core/src/engine/ops/aggregate.rs
- [ ] T050 [US3] Implement duplicate group_by column check in validate_aggregate_spec in crates/core/src/engine/ops/aggregate.rs
- [ ] T051 [US3] Implement duplicate aggregation output column check in validate_aggregate_spec in crates/core/src/engine/ops/aggregate.rs
- [ ] T052 [US3] Implement system column conflict check in validate_aggregate_spec in crates/core/src/engine/ops/aggregate.rs
- [ ] T053 [US3] Implement unknown group_by column check in validate_aggregate_compile in crates/core/src/engine/ops/aggregate.rs
- [ ] T054 [US3] Implement unknown aggregation column check in validate_aggregate_compile in crates/core/src/engine/ops/aggregate.rs
- [ ] T055 [US3] Add validation calls at entry to execute_aggregate in crates/core/src/engine/ops/aggregate.rs
- [ ] T056 [US3] Verify all US3 tests pass (T042-T047)

**Checkpoint**: All user stories should now be independently functional - validation prevents invalid configurations

---

## Phase 6: Edge Cases & Contract Tests

**Purpose**: Handle edge cases and verify serialization contracts

### Edge Case Tests

- [ ] T057 [P] Integration test for edge case: zero input rows (selector filters all) in crates/core/tests/integration/aggregate_execution_test.rs
- [ ] T058 [P] Integration test for edge case: single group (all rows same group key) in crates/core/tests/integration/aggregate_execution_test.rs
- [ ] T059 [P] Integration test for edge case: null values in group-by columns in crates/core/tests/integration/aggregate_execution_test.rs
- [ ] T060 [P] Integration test for edge case: null values in aggregated columns in crates/core/tests/integration/aggregate_execution_test.rs
- [ ] T061 [P] Integration test for edge case: all nulls in aggregated column in crates/core/tests/integration/aggregate_execution_test.rs

### Contract Tests (Serialization/Deserialization)

- [ ] T062 [P] Contract test for AggregateOperation deserialization from YAML in crates/core/tests/contracts/aggregate_contract_test.rs
- [ ] T063 [P] Contract test for AggregateOperation deserialization from JSON in crates/core/tests/contracts/aggregate_contract_test.rs
- [ ] T064 [P] Contract test for Aggregation struct serialization in crates/core/tests/contracts/aggregate_contract_test.rs
- [ ] T065 [P] Contract test for OperationInstance with OperationKind::Aggregate in crates/core/tests/contracts/aggregate_contract_test.rs
- [ ] T066 [P] Contract test for error message format consistency in crates/core/tests/contracts/aggregate_contract_test.rs

### Edge Case Implementation

- [ ] T067 Implement zero-row edge case handling (return unchanged dataset) in crates/core/src/engine/ops/aggregate.rs
- [ ] T068 Implement null group key handling (treat null as distinct group) in crates/core/src/engine/ops/aggregate.rs
- [ ] T069 Implement null aggregate value handling (follow SQL semantics) in crates/core/src/engine/ops/aggregate.rs
- [ ] T070 Verify all edge case tests pass (T057-T061)
- [ ] T071 Verify all contract tests pass (T062-T066)

---

## Phase 7: Polish & Cross-Cutting Concerns

**Purpose**: Improvements that affect multiple user stories

- [ ] T072 [P] Add comprehensive documentation comments to all public functions in crates/core/src/engine/ops/aggregate.rs
- [ ] T073 [P] Add module-level documentation with examples in crates/core/src/engine/ops/aggregate.rs
- [ ] T074 [P] Export public API in crates/core/src/engine/ops/mod.rs
- [ ] T075 [P] Add error context with anyhow for better debugging in crates/core/src/engine/ops/aggregate.rs
- [ ] T076 Run cargo clippy on aggregate module and fix all warnings
- [ ] T077 Run cargo fmt on aggregate module
- [ ] T078 Run full test suite (cargo test) and verify 100% pass
- [ ] T079 Validate against quickstart.md examples in /workspace/specs/001-aggregate-operation/quickstart.md
- [ ] T080 Run performance profiling with 100k+ rows per plan.md performance goals
- [ ] T081 Code review and refactoring for clarity and maintainability

---

## Dependencies & Execution Order

### Phase Dependencies

- **Setup (Phase 1)**: No dependencies - can start immediately
- **Foundational (Phase 2)**: Depends on Setup completion - BLOCKS all user stories
- **User Stories (Phase 3-5)**: All depend on Foundational phase completion
  - User stories can then proceed in parallel (if staffed)
  - Or sequentially in priority order (P1 → P2 → P3)
- **Edge Cases (Phase 6)**: Depends on all user stories being complete
- **Polish (Phase 7)**: Depends on all desired functionality being complete

### User Story Dependencies

- **User Story 1 (P1)**: Can start after Foundational (Phase 2) - No dependencies on other stories
  - Provides core aggregate execution: grouping, aggregation, row appending
  - Independent test: Basic aggregation with row preservation
  
- **User Story 2 (P2)**: Can start after Foundational (Phase 2) - May build on US1 implementation but independently testable
  - Extends US1 with schema consistency: metadata population, null handling
  - Independent test: Verify summary row schema matches working dataset
  
- **User Story 3 (P3)**: Can start after Foundational (Phase 2) - Adds validation to US1 execution path
  - Adds early validation to prevent invalid configurations
  - Independent test: Submit invalid specs and verify rejection with clear errors

### Within Each User Story

- Tests MUST be written and FAIL before implementation (Red-Green-Refactor)
- Validation functions before execution functions
- Helper functions before main execute_aggregate integration
- Core implementation before edge case handling
- Story complete (all tests pass) before moving to next priority

### Parallel Opportunities

- All Setup tasks marked [P] can run in parallel (T003, T004)
- All tests for a user story marked [P] can run in parallel (write tests together)
- Implementation tasks within same story can sometimes overlap if different sections of code
- Different user stories can be worked on in parallel by different team members after Phase 2
- All contract tests (T062-T066) can run in parallel
- All edge case tests (T057-T061) can run in parallel
- All polish documentation tasks (T072-T075) can run in parallel

---

## Parallel Example: User Story 1

```bash
# Phase: Write ALL tests for User Story 1 FIRST (TDD - Red phase)
Parallel Tasks:
  - T012: Unit test for basic aggregation execution
  - T013: Unit test for multi-column group-by
  - T014: Integration test for SUM aggregate
  - T015: Integration test for COUNT aggregate
  - T016: Integration test for row preservation
  - T017: Integration test for TS-05 scenario
  
# Verify all tests FAIL (no implementation yet)

# Phase: Implement User Story 1 (Green phase)
Sequential Tasks:
  - T018: Implement validate_aggregate_spec
  - T019: Implement validate_aggregate_compile
  - T020: Helper for group_by expressions
  - T021: Helper for aggregation expressions
  - T022: execute_aggregate skeleton
  - T023: Selector filtering logic
  - T024: Polars group_by and agg
  - T025: Summary row appending
  - T026: Verify all US1 tests pass

# Phase: Refactor while keeping tests green
```

---

## Parallel Example: User Story 2

```bash
# Phase: Write ALL tests for User Story 2 FIRST
Parallel Tasks:
  - T027: Unit test for null columns
  - T028: Unit test for system metadata
  - T029: Integration test for AVG
  - T030: Integration test for MIN_AGG
  - T031: Integration test for MAX_AGG
  - T032: Integration test for schema consistency

# Phase: Implement User Story 2
Sequential Tasks:
  - T033: Identify non-aggregated columns
  - T034: Add null columns function
  - T035: _row_id with UUID v7
  - T036: Timestamps
  - T037: Source lineage fields
  - T038: _deleted flag
  - T039: _period handling
  - T040: Integrate into execute_aggregate
  - T041: Verify all US2 tests pass
```

---

## Implementation Strategy

### MVP First (User Story 1 Only)

1. Complete Phase 1: Setup (T001-T004)
2. Complete Phase 2: Foundational (T005-T011) - CRITICAL, blocks all stories
3. Complete Phase 3: User Story 1 (T012-T026)
4. **STOP and VALIDATE**: Test User Story 1 independently
   - Can group rows by single/multiple columns
   - Can compute SUM, COUNT aggregates
   - Preserves all original rows
   - Appends summary rows correctly
5. **MVP READY**: Core aggregate functionality works

### Incremental Delivery

1. MVP (US1) → Demonstrates core grouping and aggregation value
2. Add US2 → Adds schema consistency and metadata (professional quality)
3. Add US3 → Adds validation and error handling (production ready)
4. Add Phase 6 → Handles all edge cases (robust)
5. Add Phase 7 → Polish and documentation (maintainable)

Each increment adds value without breaking previous functionality.

### Parallel Team Strategy

With multiple developers:

1. **All together**: Complete Setup + Foundational (Phase 1-2)
2. **Once Phase 2 done, split work**:
   - Developer A: User Story 1 (T012-T026)
   - Developer B: User Story 2 (T027-T041)
   - Developer C: User Story 3 (T042-T056)
3. Stories integrate cleanly (US2 extends US1, US3 adds validation)
4. **Merge in priority order**: US1 → US2 → US3

### TDD Workflow (Per Constitutional Principle I)

**For each user story:**

1. **RED**: Write all tests first (T012-T017 for US1)
   - Run tests, verify they FAIL
   - Commit failing tests
2. **GREEN**: Implement minimum code to pass (T018-T025 for US1)
   - Run tests frequently
   - Stop when all tests pass
   - Commit passing implementation
3. **REFACTOR**: Improve code quality
   - Keep tests passing
   - Extract functions, improve naming
   - Commit refactorings

**Never skip writing tests first. Never commit without passing tests.**

---

## Success Metrics

### Per User Story

- **US1 Success**:
  - ✅ All 6 US1 tests pass (T012-T017)
  - ✅ Can group by 1+ columns
  - ✅ Can compute SUM, COUNT
  - ✅ Original rows unchanged
  - ✅ Summary rows appended correctly

- **US2 Success**:
  - ✅ All 6 US2 tests pass (T027-T032)
  - ✅ Non-aggregated columns are null
  - ✅ All system metadata populated
  - ✅ Can compute AVG, MIN_AGG, MAX_AGG
  - ✅ Schema matches working dataset exactly

- **US3 Success**:
  - ✅ All 6 US3 tests pass (T042-T047)
  - ✅ Empty lists rejected
  - ✅ Unknown columns rejected
  - ✅ Duplicates rejected
  - ✅ System column conflicts rejected
  - ✅ Clear error messages for all cases

### Overall Feature Success (All 81 tasks complete)

- ✅ 100% of acceptance scenarios from spec.md pass
- ✅ 100% of success criteria (SC-001 through SC-005) met
- ✅ All 9 integration test scenarios pass
- ✅ All 6 validation unit tests pass
- ✅ All 5 edge case tests pass
- ✅ All 5 contract tests pass
- ✅ cargo test shows 0 failures
- ✅ cargo clippy shows 0 warnings
- ✅ Quickstart examples all work
- ✅ Performance goals met (100k+ rows, <1s latency)

---

## Notes

- **[P] tasks**: Different files, no dependencies - can run in parallel
- **[US#] label**: Maps task to specific user story for traceability
- **Each user story independently completable and testable**
- **TDD workflow mandatory**: Tests first (RED), implementation (GREEN), refactor
- **Commit strategy**: After each task or logical group, with passing tests
- **Stop at any checkpoint**: Validate story independently before proceeding
- **Avoid**:
  - Vague tasks without file paths
  - Same file conflicts (coordinate edits to aggregate.rs)
  - Cross-story dependencies that break independence
  - Implementing before tests are written
  - Committing with failing tests

**Total Task Count**: 81 tasks
- Setup: 4 tasks
- Foundational: 7 tasks
- User Story 1 (P1): 15 tasks (6 tests + 9 implementation)
- User Story 2 (P2): 15 tasks (6 tests + 9 implementation)
- User Story 3 (P3): 15 tasks (6 tests + 9 implementation)
- Edge Cases & Contracts: 15 tasks (11 tests + 4 implementation)
- Polish: 10 tasks

**Tests**: 44 test tasks (54% of total - demonstrates TDD commitment)
**Implementation**: 37 implementation tasks

**Suggested MVP Scope**: Phase 1 + Phase 2 + Phase 3 (User Story 1 only) = 26 tasks
This delivers core aggregate functionality with grouping, basic aggregates (SUM, COUNT), and row preservation.
