---
description: "Task list for output operation feature implementation"
---

# Tasks: Output Operation

**Input**: Design documents from `/workspace/specs/010-output-operation/`
**Prerequisites**: plan.md, spec.md, research.md, data-model.md, contracts/api.md

**Tests**: Per constitutional principle I (TDD), all tasks include tests written BEFORE implementation. Tests are MANDATORY and NON-NEGOTIABLE.

**Organization**: Tasks are grouped by user story to enable independent implementation and testing of each story.

## Format: `- [ ] [ID] [P?] [Story?] Description`

- **[P]**: Can run in parallel (different files, no dependencies)
- **[Story]**: Which user story this task belongs to (e.g., US1, US2, US3)
- Include exact file paths in descriptions

## Path Conventions

- **Workspace**: Rust workspace with multiple crates
- **Implementation crate**: `crates/core/`
- **Test location**: `crates/core/tests/`
- All paths are from repository root

---

## Phase 1: Setup (Shared Infrastructure)

**Purpose**: Project initialization and basic structure

- [X] T001 Create module structure in crates/core/src/engine/ops/mod.rs
- [X] T002 Create output operation module file at crates/core/src/engine/ops/output.rs
- [X] T003 [P] Create unit test directory structure at crates/core/tests/unit/
- [X] T004 [P] Create integration test directory structure at crates/core/tests/integration/
- [X] T005 [P] Create contract test directory structure at crates/core/tests/contract/

---

## Phase 2: Foundational (Blocking Prerequisites)

**Purpose**: Core data structures and error types that ALL user stories depend on

**⚠️ CRITICAL**: No user story work can begin until this phase is complete

- [X] T006 Define OutputError enum with all error variants in crates/core/src/engine/ops/output.rs
- [X] T007 [P] Define OutputOperation struct in crates/core/src/engine/ops/output.rs
- [X] T008 [P] Define OutputDestination enum in crates/core/src/engine/ops/output.rs
- [X] T009 [P] Define OutputFormat enum in crates/core/src/engine/ops/output.rs
- [X] T010 [P] Define OutputResult struct in crates/core/src/engine/ops/output.rs
- [X] T011 [P] Define OutputSchema struct and ColumnDef in crates/core/src/engine/ops/output.rs
- [X] T012 Implement extract_schema helper function in crates/core/src/engine/ops/output.rs
- [X] T013 Add module exports to crates/core/src/engine/ops/mod.rs

**Checkpoint**: Foundation ready - user story implementation can now begin in parallel

---

## Phase 3: User Story 1 - Basic Output (All Rows, All Columns) (Priority: P1) 🎯 MVP

**Goal**: Output the entire working dataset to a destination without filtering or projection

**Independent Test**: Given a 1000-row, 10-column dataset, verify all rows and columns are written to destination and working dataset remains unchanged

### Tests for User Story 1 (MANDATORY - TDD Principle I) ⚠️

> **NOTE: Write these tests FIRST, ensure they FAIL before implementation**

- [X] T014 [P] [US1] Write unit test for basic output (no selector, no projection) in crates/core/tests/unit/output_op_test.rs
- [X] T015 [P] [US1] Write integration test for end-to-end basic output in crates/core/tests/integration/output_integration_test.rs
- [X] T016 [P] [US1] Write test to verify working dataset immutability in crates/core/tests/integration/output_integration_test.rs

### Implementation for User Story 1

- [X] T017 [US1] Implement execute_output function skeleton with working_dataset cloning in crates/core/src/engine/ops/output.rs
- [X] T018 [US1] Implement LazyFrame to DataFrame collection in execute_output in crates/core/src/engine/ops/output.rs
- [X] T019 [US1] Implement write operation via OutputWriter trait in execute_output in crates/core/src/engine/ops/output.rs
- [X] T020 [US1] Implement OutputResult construction and return in execute_output in crates/core/src/engine/ops/output.rs
- [X] T021 [US1] Add error handling for write failures in execute_output in crates/core/src/engine/ops/output.rs
- [X] T022 [US1] Run unit tests to verify basic output functionality

**Checkpoint**: At this point, User Story 1 should be fully functional - basic output works, tests pass

---

## Phase 4: User Story 2 - Column Projection (TS-07) (Priority: P1)

**Goal**: Output only specific columns from the working dataset to reduce output size

**Independent Test**: Given a 10-column dataset, output only 4 specified columns and verify output contains exactly those columns in the specified order

### Tests for User Story 2 (MANDATORY - TDD Principle I) ⚠️

- [X] T023 [P] [US2] Write contract test for TS-07 column projection in crates/core/tests/contract/ts07_column_projection.rs
- [X] T024 [P] [US2] Write unit test for column projection logic in crates/core/tests/unit/output_op_test.rs
- [X] T025 [P] [US2] Write unit test for invalid column error handling in crates/core/tests/unit/output_op_test.rs

### Implementation for User Story 2

- [X] T026 [US2] Implement validate_columns function to check column existence in crates/core/src/engine/ops/output.rs
- [X] T027 [US2] Implement column projection logic (LazyFrame select) in execute_output in crates/core/src/engine/ops/output.rs
- [X] T028 [US2] Add ColumnProjectionError handling with missing columns list in crates/core/src/engine/ops/output.rs
- [X] T029 [US2] Update OutputResult to include columns_written field in crates/core/src/engine/ops/output.rs
- [X] T030 [US2] Run contract test TS-07 to verify column projection works correctly

**Checkpoint**: At this point, User Stories 1 AND 2 should both work independently

---

## Phase 5: User Story 3 - Exclude Deleted Rows (Default Behavior) (Priority: P2)

**Goal**: Exclude soft-deleted rows from output by default to prevent downstream consumers from processing deleted data

**Independent Test**: Given a 100-row dataset with 10 deleted rows, verify output contains only 90 non-deleted rows

### Tests for User Story 3 (MANDATORY - TDD Principle I) ⚠️

- [X] T031 [P] [US3] Write unit test for deleted row exclusion (include_deleted=false) in crates/core/tests/unit/output_op_test.rs
- [X] T032 [P] [US3] Write integration test verifying no deleted rows in output in crates/core/tests/integration/output_integration_test.rs

### Implementation for User Story 3

- [X] T033 [US3] Implement deleted row filter logic in execute_output in crates/core/src/engine/ops/output.rs
- [X] T034 [US3] Add filter condition for _deleted != true when include_deleted=false in crates/core/src/engine/ops/output.rs
- [X] T035 [US3] Ensure filter is applied AFTER selector but BEFORE projection in crates/core/src/engine/ops/output.rs
- [X] T036 [US3] Run unit and integration tests to verify deleted rows excluded

**Checkpoint**: All user stories (US1, US2, US3) should work independently

---

## Phase 6: User Story 4 - Include Deleted Rows (Priority: P2)

**Goal**: Include soft-deleted rows in output when explicitly requested for audit trails

**Independent Test**: Given a 100-row dataset with 10 deleted rows, verify output contains all 100 rows when include_deleted=true

### Tests for User Story 4 (MANDATORY - TDD Principle I) ⚠️

- [X] T037 [P] [US4] Write unit test for deleted row inclusion (include_deleted=true) in crates/core/tests/unit/output_op_test.rs
- [X] T038 [P] [US4] Write test verifying deleted rows have _deleted=true in output in crates/core/tests/unit/output_op_test.rs

### Implementation for User Story 4

- [X] T039 [US4] Update execute_output to skip deleted filter when include_deleted=true in crates/core/src/engine/ops/output.rs
- [X] T040 [US4] Add conditional logic for include_deleted flag in execute_output in crates/core/src/engine/ops/output.rs
- [X] T041 [US4] Run unit tests to verify include_deleted=true works correctly

**Checkpoint**: Deleted row handling complete (both include and exclude scenarios)

---

## Phase 7: User Story 5 - Filtered Output (Selector) (Priority: P2)

**Goal**: Output only rows matching a condition to create targeted datasets

**Independent Test**: Given a 1000-row dataset, apply selector "amount > 10000 AND region = 'EMEA'" and verify only matching rows are output

### Tests for User Story 5 (MANDATORY - TDD Principle I) ⚠️

- [X] T042 [P] [US5] Write unit test for selector filtering in crates/core/tests/unit/output_op_test.rs
- [X] T043 [P] [US5] Write unit test for selector evaluation errors in crates/core/tests/unit/output_op_test.rs
- [X] T044 [P] [US5] Write integration test for complex selector expressions in crates/core/tests/integration/output_integration_test.rs

### Implementation for User Story 5

- [X] T045 [US5] Implement validate_selector function to check selector is boolean expression in crates/core/src/engine/ops/output.rs
- [X] T046 [US5] Implement selector filter application in execute_output in crates/core/src/engine/ops/output.rs
- [X] T047 [US5] Add SelectorError handling for invalid or non-boolean selectors in crates/core/src/engine/ops/output.rs
- [X] T048 [US5] Ensure selector is evaluated BEFORE deleted flag filter in crates/core/src/engine/ops/output.rs
- [X] T049 [US5] Run unit and integration tests to verify selector filtering works

**Checkpoint**: Selector filtering complete and independently testable

---

## Phase 8: User Story 6 - Register Output as Dataset (Priority: P2)

**Goal**: Register the output as a named Dataset so it can be reused as input to other Projects

**Independent Test**: Given a successful output write, verify Dataset entity is created with correct name, version, schema, and can be queried via MetadataStore

### Tests for User Story 6 (MANDATORY - TDD Principle I) ⚠️

- [X] T050 [P] [US6] Write unit test for dataset registration logic in crates/core/tests/unit/output_op_test.rs
- [X] T051 [P] [US6] Write integration test for dataset registration via MetadataStore in crates/core/tests/integration/output_integration_test.rs
- [X] T052 [P] [US6] Write test for MissingMetadataStore error when registration requested but store is None in crates/core/tests/unit/output_op_test.rs
- [X] T053 [P] [US6] Write test for dataset version increment on re-registration in crates/core/tests/integration/output_integration_test.rs

### Implementation for User Story 6

- [X] T054 [US6] Implement register_dataset helper function in crates/core/src/engine/ops/output.rs
- [X] T055 [US6] Implement schema extraction from output DataFrame for registration in crates/core/src/engine/ops/output.rs
- [X] T056 [US6] Add Dataset entity creation with name, version, schema in register_dataset in crates/core/src/engine/ops/output.rs
- [X] T057 [US6] Implement version increment logic for existing datasets in register_dataset in crates/core/src/engine/ops/output.rs
- [X] T058 [US6] Add MissingMetadataStore validation in execute_output in crates/core/src/engine/ops/output.rs
- [X] T059 [US6] Integrate register_dataset into execute_output after successful write in crates/core/src/engine/ops/output.rs
- [X] T060 [US6] Add dataset_id to OutputResult when registration succeeds in crates/core/src/engine/ops/output.rs
- [X] T061 [US6] Add logging for registration failures (non-fatal) in crates/core/src/engine/ops/output.rs
- [X] T062 [US6] Run unit and integration tests to verify dataset registration

**Checkpoint**: Dataset registration complete and independently testable

---

## Phase 9: User Story 7 - Mid-Pipeline Output (Checkpoint) (Priority: P3)

**Goal**: Output intermediate results mid-pipeline without modifying the working dataset for debugging or auditing

**Independent Test**: Given a pipeline with [update, output (checkpoint), aggregate, output (final)], verify working dataset unchanged after checkpoint output

### Tests for User Story 7 (MANDATORY - TDD Principle I) ⚠️

- [X] T063 [P] [US7] Write integration test for mid-pipeline output (multiple outputs in sequence) in crates/core/tests/integration/output_integration_test.rs
- [X] T064 [P] [US7] Write test verifying working dataset unchanged after output operation in crates/core/tests/integration/output_integration_test.rs

### Implementation for User Story 7

- [X] T065 [US7] Verify execute_output uses immutable reference to working_dataset in crates/core/src/engine/ops/output.rs
- [X] T066 [US7] Add documentation explaining mid-pipeline usage in crates/core/src/engine/ops/output.rs
- [X] T067 [US7] Run integration tests to verify mid-pipeline output preserves working dataset

**Checkpoint**: Mid-pipeline output verified (immutability confirmed)

---

## Phase 10: User Story 8 - Write Failure Handling (Priority: P3)

**Goal**: Receive clear error messages when output write fails to diagnose and fix issues

**Independent Test**: Given an inaccessible destination, verify operation fails with WriteFailed error and descriptive message

### Tests for User Story 8 (MANDATORY - TDD Principle I) ⚠️

- [X] T068 [P] [US8] Write unit test for write failure error handling in crates/core/tests/unit/output_op_test.rs
- [X] T069 [P] [US8] Write test verifying no Dataset registered when write fails in crates/core/tests/integration/output_integration_test.rs

### Implementation for User Story 8

- [X] T070 [US8] Implement error context propagation for WriteFailed in execute_output in crates/core/src/engine/ops/output.rs
- [X] T071 [US8] Add detailed error message construction with root cause in crates/core/src/engine/ops/output.rs
- [X] T072 [US8] Ensure registration is skipped when write fails in execute_output in crates/core/src/engine/ops/output.rs
- [X] T073 [US8] Run unit and integration tests to verify write failure handling

**Checkpoint**: Error handling complete for write failures

---

## Phase 11: User Story 9 - Column Projection Validation (Priority: P3)

**Goal**: Be notified if invalid column names are specified to correct configuration before execution

**Independent Test**: Given a dataset with columns [id, name, amount], specify columns [id, price, quantity] and verify error lists missing columns and available columns

### Tests for User Story 9 (MANDATORY - TDD Principle I) ⚠️

- [X] T074 [P] [US9] Write unit test for column validation with missing columns in crates/core/tests/unit/output_op_test.rs
- [X] T075 [P] [US9] Write test verifying error message includes available columns list in crates/core/tests/unit/output_op_test.rs

### Implementation for User Story 9

- [X] T076 [US9] Implement pre-execution column validation in execute_output in crates/core/src/engine/ops/output.rs
- [X] T077 [US9] Build ColumnProjectionError with missing columns list in crates/core/src/engine/ops/output.rs
- [X] T078 [US9] Add available columns to error message for debugging in crates/core/src/engine/ops/output.rs
- [X] T079 [US9] Run unit tests to verify column projection validation

**Checkpoint**: All user stories complete and independently testable

---

## Phase 12: Polish & Cross-Cutting Concerns

**Purpose**: Improvements that affect multiple user stories

- [X] T080 [P] Add comprehensive documentation comments to all public functions in crates/core/src/engine/ops/output.rs
- [X] T081 [P] Add performance benchmarks for large dataset processing in crates/core/tests/benchmarks/output_benchmarks.rs
- [X] T082 [P] Add logging for all major operations (filter, project, write, register) in crates/core/src/engine/ops/output.rs
- [X] T083 [P] Create quickstart examples demonstrating all use cases in specs/010-output-operation/quickstart.md
- [X] T084 [P] Run cargo clippy and fix all warnings in crates/core/src/engine/ops/output.rs
- [X] T085 [P] Run cargo fmt to ensure consistent code formatting
- [-] T086 Verify test coverage meets 80% minimum (100% for critical paths) using cargo tarpaulin
- [X] T087 Run all contract tests to verify TS-07 passes
- [X] T088 Validate against quickstart.md scenarios
- [X] T089 Update project documentation with output operation usage examples

---

## Dependencies & Execution Order

### Phase Dependencies

- **Setup (Phase 1)**: No dependencies - can start immediately
- **Foundational (Phase 2)**: Depends on Setup completion - BLOCKS all user stories
- **User Stories (Phase 3-11)**: All depend on Foundational phase completion
  - User stories can then proceed in parallel (if staffed)
  - Or sequentially in priority order (P1 → P2 → P3)
- **Polish (Phase 12)**: Depends on all desired user stories being complete

### User Story Dependencies

- **US1 (Basic Output)**: Can start after Foundational - No dependencies on other stories
- **US2 (Column Projection)**: Can start after Foundational - Builds on US1 but independently testable
- **US3 (Exclude Deleted)**: Can start after Foundational - Independent of US1/US2
- **US4 (Include Deleted)**: Can start after Foundational - Related to US3 but independent
- **US5 (Selector)**: Can start after Foundational - Independent of other stories
- **US6 (Register Dataset)**: Can start after Foundational - Independent of other stories
- **US7 (Mid-Pipeline)**: Can start after Foundational - Validates immutability from US1
- **US8 (Write Failure)**: Can start after Foundational - Error handling for US1
- **US9 (Validation)**: Can start after Foundational - Error handling for US2

### Within Each User Story

- Tests MUST be written and FAIL before implementation (TDD Principle I)
- Validation functions before main logic
- Main execute_output updates after validation
- Error handling integrated with implementation
- Tests run to verify story complete before moving to next priority

### Parallel Opportunities

- All Setup tasks marked [P] can run in parallel
- All Foundational tasks marked [P] can run in parallel (within Phase 2)
- Once Foundational phase completes, all user stories can start in parallel (if team capacity allows)
- All tests for a user story marked [P] can run in parallel
- All Polish tasks marked [P] can run in parallel

---

## Parallel Example: User Story 2 (Column Projection)

```bash
# Launch all tests for User Story 2 together (MANDATORY per TDD principle):
Task T023: "Contract test for TS-07 column projection in crates/core/tests/contract/ts07_column_projection.rs"
Task T024: "Unit test for column projection logic in crates/core/tests/unit/output_op_test.rs"
Task T025: "Unit test for invalid column error handling in crates/core/tests/unit/output_op_test.rs"

# Then implement sequentially after tests fail:
Task T026: "Implement validate_columns function"
Task T027: "Implement column projection logic"
# ... etc
```

---

## Implementation Strategy

### MVP First (User Stories 1-2 Only)

1. Complete Phase 1: Setup
2. Complete Phase 2: Foundational (CRITICAL - blocks all stories)
3. Complete Phase 3: User Story 1 (Basic Output)
4. Complete Phase 4: User Story 2 (Column Projection with TS-07)
5. **STOP and VALIDATE**: Test US1 and US2 independently
6. Deploy/demo if ready

**Rationale**: US1 + US2 provide core output functionality with column projection (covers TS-07 contract test)

### Incremental Delivery

1. Complete Setup + Foundational → Foundation ready
2. Add User Story 1 (Basic Output) → Test independently → Core output works
3. Add User Story 2 (Column Projection) → Test independently → TS-07 passes
4. Add User Story 3 (Exclude Deleted) → Test independently → Default behavior complete
5. Add User Story 4 (Include Deleted) → Test independently → Audit support added
6. Add User Story 5 (Selector) → Test independently → Filtering works
7. Add User Story 6 (Register Dataset) → Test independently → Reusability enabled
8. Add User Story 7 (Mid-Pipeline) → Test independently → Checkpointing validated
9. Add User Story 8 (Write Failure) → Test independently → Error handling robust
10. Add User Story 9 (Validation) → Test independently → All error cases covered
11. Each story adds value without breaking previous stories

### Parallel Team Strategy

With multiple developers:

1. Team completes Setup + Foundational together
2. Once Foundational is done:
   - Developer A: User Stories 1-2 (Basic + Projection)
   - Developer B: User Stories 3-4 (Deleted row handling)
   - Developer C: User Stories 5-6 (Selector + Registration)
   - Developer D: User Stories 7-9 (Mid-pipeline + Error handling)
3. Stories complete and integrate independently

---

## Summary

- **Total Tasks**: 89 tasks
- **Task Distribution**:
  - Setup: 5 tasks
  - Foundational: 8 tasks
  - User Story 1 (Basic Output): 9 tasks
  - User Story 2 (Column Projection): 8 tasks
  - User Story 3 (Exclude Deleted): 6 tasks
  - User Story 4 (Include Deleted): 5 tasks
  - User Story 5 (Selector): 8 tasks
  - User Story 6 (Register Dataset): 13 tasks
  - User Story 7 (Mid-Pipeline): 5 tasks
  - User Story 8 (Write Failure): 6 tasks
  - User Story 9 (Validation): 6 tasks
  - Polish: 10 tasks

- **Parallel Opportunities**: 42 tasks marked [P] can run in parallel within their phase
- **Independent Test Criteria**:
  - US1: Basic output writes all rows/columns, working dataset unchanged
  - US2: Column projection outputs only specified columns in correct order
  - US3: Deleted rows excluded by default
  - US4: Deleted rows included when opted in
  - US5: Selector filters rows correctly
  - US6: Dataset registration creates valid entity
  - US7: Mid-pipeline output preserves working dataset
  - US8: Write failures produce clear error messages
  - US9: Column validation catches missing columns before execution

- **Suggested MVP Scope**: User Stories 1-2 (Basic Output + Column Projection)
  - Covers core output functionality
  - Implements contract test TS-07
  - Provides immediate value
  - ~22 tasks total (Setup + Foundational + US1 + US2)

- **Format Validation**: ✅ ALL tasks follow strict checklist format:
  - `- [ ] [TaskID] [P?] [Story?] Description with file path`
  - Sequential Task IDs (T001-T089)
  - [P] marker only on parallelizable tasks
  - [Story] label (US1-US9) on all user story tasks
  - Explicit file paths in every description

---

## Notes

- All tests MANDATORY per TDD Principle I
- Tests must be written FIRST and FAIL before implementation
- [P] tasks = different files, no dependencies within phase
- [Story] label maps task to specific user story for traceability
- Each user story should be independently completable and testable
- Verify tests fail before implementing
- Commit after each task or logical group
- Stop at any checkpoint to validate story independently
- Memory efficiency: LazyFrame with late materialization (research.md decision R1)
- Error handling: thiserror for structured errors (research.md decision R3)
