# Tasks: Update Operation

**Input**: Design documents from `/workspace/specs/005-update-operation/`
**Prerequisites**: plan.md, spec.md, research.md, data-model.md, contracts/rust-api.md, quickstart.md

**Tests**: Per constitutional principle I (TDD), all tasks include tests written BEFORE implementation. Tests are MANDATORY and NON-NEGOTIABLE.

**Organization**: Tasks are grouped by functional user story to enable independent implementation and testing of each capability.

## Format: `- [X] [ID] [P?] [Story] Description`

- **[P]**: Can run in parallel (different files, no dependencies)
- **[Story]**: Which user story this task belongs to (e.g., US1, US2, US3)
- Include exact file paths in descriptions

## Path Conventions

- Repository root: `/workspace/`
- Core crate: `/workspace/crates/core/`
- Source files: `/workspace/crates/core/src/`
- Unit tests: `/workspace/crates/core/tests/unit/`
- Integration tests: `/workspace/crates/core/tests/integration/scenarios/`

---

## Phase 1: Setup (Shared Infrastructure)

**Purpose**: Create module structure for update operation implementation

- [X] T001 Create ops module directory at /workspace/crates/core/src/engine/ops/
- [X] T002 Create /workspace/crates/core/src/engine/ops/mod.rs with update module export
- [X] T003 Update /workspace/crates/core/src/engine/mod.rs to expose ops module
- [X] T004 Create unit test directory at /workspace/crates/core/tests/unit/
- [X] T005 Create /workspace/crates/core/tests/unit/mod.rs to include update_operation_test module

---

## Phase 2: Foundational (Blocking Prerequisites)

**Purpose**: Core data structures and helper functions that ALL user stories depend on

**⚠️ CRITICAL**: No user story work can begin until this phase is complete

- [X] T006 Define UpdateOperation struct in /workspace/crates/core/src/engine/ops/update.rs with selector and assignments fields
- [X] T007 [P] Define Assignment struct in /workspace/crates/core/src/engine/ops/update.rs with column and expression fields
- [X] T008 [P] Define UpdateExecutionContext struct in /workspace/crates/core/src/engine/ops/update.rs with working_dataset, selectors, and run_timestamp fields
- [X] T009 Add serde derives (Serialize, Deserialize) to UpdateOperation and Assignment structs in /workspace/crates/core/src/engine/ops/update.rs
- [X] T010 Add validation for UpdateOperation (non-empty assignments) in /workspace/crates/core/src/engine/ops/update.rs
- [X] T011 Implement resolve_selector helper function signature in /workspace/crates/core/src/engine/ops/update.rs
- [X] T012 [P] Implement compile_selector helper function signature in /workspace/crates/core/src/engine/ops/update.rs
- [X] T013 [P] Implement compile_assignments helper function signature in /workspace/crates/core/src/engine/ops/update.rs
- [X] T014 Implement execute_update function signature in /workspace/crates/core/src/engine/ops/update.rs

**Checkpoint**: Foundation ready - user story implementation can now begin in parallel

---

## Phase 3: User Story 1 - Basic Assignment Without Selector (Priority: P1) 🎯 MVP

**Goal**: Apply a single assignment to all rows in the dataset without any filtering. This is the simplest update operation scenario.

**Independent Test**: Create a DataFrame with 3 rows, apply update with no selector and single assignment, verify all 3 rows are modified and _updated_at is set.

### Tests for User Story 1 (MANDATORY - TDD Principle I) ⚠️

> **NOTE: Write these tests FIRST, ensure they FAIL before implementation**

- [X] T015 [US1] Create /workspace/crates/core/tests/unit/update_operation_test.rs with test_update_single_assignment_no_selector test
- [X] T016 [US1] Write test for empty assignments error case in /workspace/crates/core/tests/unit/update_operation_test.rs
- [X] T017 [US1] Write test for _updated_at system column update in /workspace/crates/core/tests/unit/update_operation_test.rs

### Implementation for User Story 1

- [X] T018 [US1] Implement execute_update function basic flow in /workspace/crates/core/src/engine/ops/update.rs (validate assignments, handle None selector as all rows)
- [X] T019 [US1] Implement compile_assignments function in /workspace/crates/core/src/engine/ops/update.rs (parse expression strings to Polars Expr with alias)
- [X] T020 [US1] Add simple expression parser for string literals in /workspace/crates/core/src/engine/ops/update.rs (placeholder until S01 integration)
- [X] T021 [US1] Apply assignments using LazyFrame.with_columns() in execute_update function in /workspace/crates/core/src/engine/ops/update.rs
- [X] T022 [US1] Add _updated_at column update with run_timestamp in execute_update function in /workspace/crates/core/src/engine/ops/update.rs
- [X] T023 [US1] Verify User Story 1 tests pass from /workspace/crates/core/tests/unit/update_operation_test.rs

**Checkpoint**: At this point, User Story 1 should be fully functional - can update all rows with simple assignments

---

## Phase 4: User Story 2 - Named Selector Resolution (Priority: P1)

**Goal**: Support {{NAME}} selector interpolation from Project selectors map. This enables reusable filter expressions.

**Independent Test**: Create DataFrame with active and inactive rows, use {{active_rows}} selector from selectors map, verify only active rows are updated.

### Tests for User Story 2 (MANDATORY - TDD Principle I) ⚠️

- [X] T024 [US2] Write test_update_with_named_selector test in /workspace/crates/core/tests/unit/update_operation_test.rs
- [X] T025 [US2] Write test for undefined selector name error in /workspace/crates/core/tests/unit/update_operation_test.rs
- [X] T026 [US2] Write test for selector without interpolation (plain expression) in /workspace/crates/core/tests/unit/update_operation_test.rs

### Implementation for User Story 2

- [X] T027 [US2] Implement resolve_selector function in /workspace/crates/core/src/engine/ops/update.rs (detect {{NAME}} pattern, lookup in selectors map)
- [X] T028 [US2] Add error handling for undefined selector names in resolve_selector in /workspace/crates/core/src/engine/ops/update.rs
- [X] T029 [US2] Integrate resolve_selector into execute_update flow before compile_selector in /workspace/crates/core/src/engine/ops/update.rs
- [X] T030 [US2] Verify User Story 2 tests pass from /workspace/crates/core/tests/unit/update_operation_test.rs

**Checkpoint**: At this point, User Stories 1 AND 2 should both work - can use named selectors

---

## Phase 5: User Story 3 - Selector-Based Row Filtering (Priority: P1)

**Goal**: Apply selector expression to filter which rows get updated. Non-matching rows pass through unchanged.

**Independent Test**: Create DataFrame with 5 rows, use selector "status = 'active'", verify only matching rows are updated and non-matching rows retain original values.

### Tests for User Story 3 (MANDATORY - TDD Principle I) ⚠️

- [X] T031 [US3] Write test_update_with_selector_filters_rows test in /workspace/crates/core/tests/unit/update_operation_test.rs
- [X] T032 [US3] Write test for non_matching_rows_unchanged in /workspace/crates/core/tests/unit/update_operation_test.rs
- [X] T033 [US3] Write test for invalid selector expression error in /workspace/crates/core/tests/unit/update_operation_test.rs

### Implementation for User Story 3

- [X] T034 [US3] Implement compile_selector function in /workspace/crates/core/src/engine/ops/update.rs (parse selector to Polars Expr)
- [X] T035 [US3] Extend expression parser to support comparison operators (=, >, <) in /workspace/crates/core/src/engine/ops/update.rs
- [X] T036 [US3] Apply selector filtering using LazyFrame.filter() in execute_update in /workspace/crates/core/src/engine/ops/update.rs
- [X] T037 [US3] Implement row merge logic to union updated rows with non-matching rows in execute_update in /workspace/crates/core/src/engine/ops/update.rs
- [X] T038 [US3] Verify User Story 3 tests pass from /workspace/crates/core/tests/unit/update_operation_test.rs

**Checkpoint**: At this point, User Stories 1-3 should all work - basic filtering is functional

---

## Phase 6: User Story 4 - Multiple Assignments (Priority: P2)

**Goal**: Support multiple column assignments in a single update operation for efficiency.

**Independent Test**: Create DataFrame, apply update with 3 different assignments, verify all 3 columns are updated correctly in a single operation.

### Tests for User Story 4 (MANDATORY - TDD Principle I) ⚠️

- [X] T039 [US4] Write test_update_multiple_assignments test in /workspace/crates/core/tests/unit/update_operation_test.rs
- [X] T040 [US4] Write test for assignment with arithmetic expressions in /workspace/crates/core/tests/unit/update_operation_test.rs

### Implementation for User Story 4

- [X] T041 [US4] Extend expression parser to support arithmetic operators (+, -, *, /) in /workspace/crates/core/src/engine/ops/update.rs
- [X] T042 [US4] Extend expression parser to support column references in /workspace/crates/core/src/engine/ops/update.rs
- [X] T043 [US4] Verify compile_assignments handles multiple expressions correctly in /workspace/crates/core/src/engine/ops/update.rs
- [X] T044 [US4] Verify User Story 4 tests pass from /workspace/crates/core/tests/unit/update_operation_test.rs

**Checkpoint**: At this point, all core user stories work - multiple assignments supported

---

## Phase 7: User Story 5 - New Column Creation (Priority: P2)

**Goal**: Allow assignments to target non-existent columns, which are created with NULL for non-matching rows.

**Independent Test**: Create DataFrame without column "discount", apply update that adds "discount" column, verify new column exists and has NULL for non-matching rows.

### Tests for User Story 5 (MANDATORY - TDD Principle I) ⚠️

- [X] T045 [US5] Write test_update_creates_new_column test in /workspace/crates/core/tests/unit/update_operation_test.rs
- [X] T046 [US5] Write test for new column has NULL in non-matching rows in /workspace/crates/core/tests/unit/update_operation_test.rs

### Implementation for User Story 5

- [X] T047 [US5] Verify LazyFrame.with_columns() behavior for new columns in /workspace/crates/core/src/engine/ops/update.rs (may require no code changes)
- [X] T048 [US5] Add test to confirm schema evolution propagates to output LazyFrame in /workspace/crates/core/tests/unit/update_operation_test.rs
- [X] T049 [US5] Verify User Story 5 tests pass from /workspace/crates/core/tests/unit/update_operation_test.rs

**Checkpoint**: All user stories now support new column creation

---

## Phase 8: User Story 6 - Integration Test Scenarios (Priority: P2)

**Goal**: Validate update operation via test harness with realistic scenarios from sample datasets.

**Independent Test**: Run TS-03 (FX conversion) and TS-08 (named selector) scenarios through test harness and verify expected outputs.

### Tests for User Story 6 (MANDATORY - TDD Principle I) ⚠️

- [X] T050 [US6] Create /workspace/crates/core/tests/integration/scenarios/ts03_fx_conversion.yaml with FX conversion scenario
- [X] T051 [US6] Create /workspace/crates/core/tests/integration/scenarios/ts08_named_selector.yaml with named selector interpolation scenario

### Implementation for User Story 6

- [X] T052 [US6] Integrate execute_update function with test harness operation executor in /workspace/crates/core/src/engine/ops/update.rs (if not already integrated)
- [X] T053 [US6] Run TS-03 scenario from /workspace/crates/core/tests/integration/scenarios/ts03_fx_conversion.yaml and verify FX conversion
- [X] T054 [US6] Run TS-08 scenario from /workspace/crates/core/tests/integration/scenarios/ts08_named_selector.yaml and verify selector interpolation
- [X] T055 [US6] Verify both integration tests pass for files in /workspace/crates/core/tests/integration/scenarios/

**Checkpoint**: Integration tests validate end-to-end functionality

---

## Phase 9: User Story 7 - Error Handling & Validation (Priority: P3)

**Goal**: Comprehensive error handling with meaningful error messages for all failure scenarios.

**Independent Test**: Trigger each error condition (undefined column, type mismatch, invalid expression) and verify appropriate error is returned with context.

### Tests for User Story 7 (MANDATORY - TDD Principle I) ⚠️

- [X] T056 [US7] Write test_undefined_column_in_expression_error test in /workspace/crates/core/tests/unit/update_operation_test.rs
- [X] T057 [US7] Write test_type_mismatch_in_assignment_error test in /workspace/crates/core/tests/unit/update_operation_test.rs
- [X] T058 [US7] Write test_invalid_column_name_validation test in /workspace/crates/core/tests/unit/update_operation_test.rs

### Implementation for User Story 7

- [X] T059 [US7] Add column name validation (regex check) in /workspace/crates/core/src/engine/ops/update.rs
- [X] T060 [US7] Add error context using .context() for all error propagation paths in execute_update in /workspace/crates/core/src/engine/ops/update.rs
- [X] T061 [US7] Verify error messages match contract specification in /workspace/specs/005-update-operation/contracts/rust-api.md
- [X] T062 [US7] Verify User Story 7 tests pass from /workspace/crates/core/tests/unit/update_operation_test.rs

**Checkpoint**: Error handling is comprehensive and user-friendly

---

## Phase 10: Polish & Cross-Cutting Concerns

**Purpose**: Improvements that affect multiple user stories and code quality

- [X] T063 [P] Add documentation comments to all public functions in /workspace/crates/core/src/engine/ops/update.rs
- [X] T064 [P] Add module-level documentation in /workspace/crates/core/src/engine/ops/update.rs
- [X] T065 Run cargo clippy and fix all warnings in /workspace/crates/core/src/engine/ops/update.rs
- [X] T066 Run cargo fmt on /workspace/crates/core/src/engine/ops/update.rs
- [X] T067 Run full test suite with cargo test --all from /workspace/
- [X] T068 Verify quickstart workflow in /workspace/specs/005-update-operation/quickstart.md by following steps manually
- [X] T069 [P] Add performance tracing events for expression compilation in /workspace/crates/core/src/engine/ops/update.rs
- [X] T070 Add TODO comments for S01 integration points in /workspace/crates/core/src/engine/ops/update.rs

---

## Dependencies & Execution Order

### Phase Dependencies

- **Setup (Phase 1)**: No dependencies - can start immediately
- **Foundational (Phase 2)**: Depends on Setup completion - BLOCKS all user stories
- **User Stories (Phase 3-9)**: All depend on Foundational phase completion
  - US1 (Basic Assignment): Can start after Foundational - No dependencies on other stories
  - US2 (Named Selector): Depends on US1 completion (uses execute_update base implementation)
  - US3 (Row Filtering): Depends on US1 completion (extends execute_update)
  - US4 (Multiple Assignments): Depends on US1, US3 (uses filtering + assignments)
  - US5 (New Column): Depends on US1, US4 (extends assignment logic)
  - US6 (Integration Tests): Depends on US1-5 (validates all features)
  - US7 (Error Handling): Can run in parallel with US1-5 (different aspect)
- **Polish (Phase 10)**: Depends on all user stories being complete

### User Story Dependencies

```
Foundational (Phase 2)
    ├── US1: Basic Assignment (Phase 3) - No dependencies
    ├── US2: Named Selector (Phase 4) - Depends on US1
    ├── US3: Row Filtering (Phase 5) - Depends on US1
    ├── US4: Multiple Assignments (Phase 6) - Depends on US1, US3
    ├── US5: New Column Creation (Phase 7) - Depends on US1, US4
    ├── US6: Integration Tests (Phase 8) - Depends on US1-5
    └── US7: Error Handling (Phase 9) - Can run in parallel with US1-5
```

### Within Each User Story

- Tests MUST be written and FAIL before implementation (Red-Green-Refactor)
- Test scaffolding before test implementation
- Helper functions before main execute_update modifications
- Core implementation before integration
- Story complete and tested before moving to next priority

### Parallel Opportunities

- **Setup (Phase 1)**: T001-T005 can run in parallel (different files)
- **Foundational (Phase 2)**: T007, T008 can run in parallel (different structs); T011, T012, T013 can run in parallel (independent helper signatures)
- **After Foundational**: US7 (Error Handling) can run in parallel with US1-5 (focuses on different aspect)
- **Within US1**: T015-T017 (tests) can run in parallel
- **Within US2**: T024-T026 (tests) can run in parallel
- **Within US3**: T031-T033 (tests) can run in parallel
- **Within US4**: T039-T040 (tests) can run in parallel
- **Within US5**: T045-T046 (tests) can run in parallel
- **Within US6**: T050-T051 (scenario files) can run in parallel
- **Within US7**: T056-T058 (tests) can run in parallel
- **Polish (Phase 10)**: T063-T064, T069-T070 can run in parallel (documentation tasks)

---

## Parallel Example: User Story 1

```bash
# Launch all tests for User Story 1 together (MANDATORY per TDD principle):
Task T015 [US1]: Create test file with test_update_single_assignment_no_selector in /workspace/crates/core/tests/unit/update_operation_test.rs
Task T016 [US1]: Write test for empty assignments error in /workspace/crates/core/tests/unit/update_operation_test.rs
Task T017 [US1]: Write test for _updated_at system column update in /workspace/crates/core/tests/unit/update_operation_test.rs

# After tests fail, implement sequentially:
Task T018 [US1]: Implement execute_update basic flow in /workspace/crates/core/src/engine/ops/update.rs
Task T019 [US1]: Implement compile_assignments in /workspace/crates/core/src/engine/ops/update.rs
Task T020 [US1]: Add simple expression parser in /workspace/crates/core/src/engine/ops/update.rs
Task T021 [US1]: Apply assignments with with_columns() in /workspace/crates/core/src/engine/ops/update.rs
Task T022 [US1]: Add _updated_at column update in /workspace/crates/core/src/engine/ops/update.rs
Task T023 [US1]: Verify tests pass from /workspace/crates/core/tests/unit/update_operation_test.rs
```

---

## Parallel Example: User Story 2

```bash
# Launch all tests for User Story 2 together:
Task T024 [US2]: Write test_update_with_named_selector in /workspace/crates/core/tests/unit/update_operation_test.rs
Task T025 [US2]: Write test for undefined selector error in /workspace/crates/core/tests/unit/update_operation_test.rs
Task T026 [US2]: Write test for plain expression selector in /workspace/crates/core/tests/unit/update_operation_test.rs

# After tests fail, implement sequentially:
Task T027 [US2]: Implement resolve_selector function in /workspace/crates/core/src/engine/ops/update.rs
Task T028 [US2]: Add error handling for undefined names in /workspace/crates/core/src/engine/ops/update.rs
Task T029 [US2]: Integrate into execute_update flow in /workspace/crates/core/src/engine/ops/update.rs
Task T030 [US2]: Verify tests pass from /workspace/crates/core/tests/unit/update_operation_test.rs
```

---

## Implementation Strategy

### MVP First (User Story 1 Only)

1. Complete Phase 1: Setup → Module structure ready
2. Complete Phase 2: Foundational → CRITICAL - all data structures defined
3. Complete Phase 3: User Story 1 → Basic assignment without selector works
4. **STOP and VALIDATE**: Run T015-T017 tests, verify all pass
5. Minimal viable update operation ready for demo/validation

**MVP Scope**: Can update all rows in a dataset with simple assignments (string/numeric literals)

### Incremental Delivery

1. Complete Setup + Foundational → Foundation ready
2. Add User Story 1 → Test independently → Basic update works (MVP!)
3. Add User Story 2 → Test independently → Named selectors work
4. Add User Story 3 → Test independently → Row filtering works
5. Add User Story 4 → Test independently → Multiple assignments work
6. Add User Story 5 → Test independently → New column creation works
7. Add User Story 6 → Test independently → Integration scenarios pass
8. Add User Story 7 → Test independently → Error handling comprehensive
9. Each story adds capability without breaking previous stories

### Parallel Team Strategy

With multiple developers (if applicable):

1. Team completes Setup + Foundational together
2. Once Foundational is done:
   - Developer A: User Story 1 + User Story 2 (sequential - US2 depends on US1)
   - Developer B: User Story 7 (Error Handling - can run in parallel)
   - Developer C: User Story 6 (Integration Tests - after US1-5 complete)
3. After US1 complete:
   - Developer A: User Story 3
   - Developer C: User Story 4 (if US1, US3 done)
4. Stories complete and integrate independently

### TDD Workflow (Red-Green-Refactor)

For each user story:

1. **Red Phase**: Write all tests (marked with story label), run cargo test, verify they FAIL
2. **Green Phase**: Implement minimal code to make tests pass
3. **Refactor Phase**: Improve code quality while keeping tests green
4. **Verify**: Run story-specific tests + full test suite
5. **Checkpoint**: Story is complete, move to next priority

---

## Notes

- **[P] tasks**: Different files, no dependencies, can run in parallel
- **[Story] label**: Maps task to specific user story for traceability (US1-US7)
- Each user story should be independently completable and testable
- **Tests are MANDATORY**: Constitutional Principle I requires TDD approach
- Verify tests FAIL before implementing (Red phase)
- Commit after each task or logical group
- Stop at any checkpoint to validate story independently
- **Avoid**: Vague tasks, same file conflicts, cross-story dependencies that break independence
- **S01 Integration**: Placeholder expression parser used initially; replace with S01 once available

---

## Total Task Count

**Total Tasks**: 70

### Tasks by User Story

- **Setup (Phase 1)**: 5 tasks
- **Foundational (Phase 2)**: 9 tasks
- **User Story 1** (Basic Assignment): 9 tasks (3 tests + 6 implementation)
- **User Story 2** (Named Selector): 7 tasks (3 tests + 4 implementation)
- **User Story 3** (Row Filtering): 8 tasks (3 tests + 5 implementation)
- **User Story 4** (Multiple Assignments): 6 tasks (2 tests + 4 implementation)
- **User Story 5** (New Column Creation): 5 tasks (2 tests + 3 implementation)
- **User Story 6** (Integration Tests): 6 tasks (2 tests + 4 implementation)
- **User Story 7** (Error Handling): 7 tasks (3 tests + 4 implementation)
- **Polish (Phase 10)**: 8 tasks

### Test Task Count

- **Total Test Tasks**: 21 (30% of total tasks)
- **Unit Tests**: 16 tasks
- **Integration Tests**: 2 tasks (scenario files)
- **Test Infrastructure**: 3 tasks

### Parallel Opportunities Identified

- **Phase 1**: 5 tasks can run in parallel (different files)
- **Phase 2**: 5 tasks can run in parallel (different structs/functions)
- **User Story Tests**: All tests within a story can be written in parallel (3 per story average)
- **Cross-Story Parallelism**: US7 (Error Handling) can run in parallel with US1-5
- **Polish Phase**: 4 documentation tasks can run in parallel

### MVP Scope (Phase 1 + 2 + 3)

**Tasks**: T001-T023 (23 tasks, 33% of total)
**Capability**: Basic update operation with single assignment, no selector, all rows updated
**Validation**: 3 unit tests pass (T015-T017)
**Time Estimate**: ~2-3 hours for experienced Rust developer (per quickstart.md)
