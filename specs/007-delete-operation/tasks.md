# Tasks: Delete Operation

**Input**: Design documents from `/workspace/specs/007-delete-operation/`
**Prerequisites**: plan.md (required), spec.md (required for user stories), research.md, data-model.md, contracts/

**Tests**: Per constitutional principle I (TDD), all tasks MUST include tests written BEFORE implementation. Tests are MANDATORY and NON-NEGOTIABLE.

**Organization**: Tasks are grouped by user story to enable independent implementation and testing of each story.

## Format: `[ID] [P?] [Story] Description`

- **[P]**: Can run in parallel (different files, no dependencies)
- **[Story]**: Which user story this task belongs to (e.g., US1, US2, US3)
- Include exact file paths in descriptions

## Path Conventions

- **Project Type**: Single Cargo workspace (core, api-server, engine-worker, cli, test-resolver crates)
- **Implementation**: `crates/core/src/` (primary delete operation implementation)
- **Tests**: `crates/core/tests/` (unit & integration), `crates/test-resolver/tests/scenarios/` (contract)

---

## Phase 1: Setup (Shared Infrastructure)

**Purpose**: Project initialization and basic structure

- [X] T001 Verify Cargo workspace structure supports delete operation in crates/core/src/operations/
- [X] T002 Verify test infrastructure exists in crates/core/tests/ and crates/test-resolver/

---

## Phase 2: Foundational (Blocking Prerequisites)

**Purpose**: Core infrastructure that MUST be complete before ANY user story can be implemented

**CRITICAL**: No user story work can begin until this phase is complete

- [X] T003 Create DeleteOperationParams struct in crates/core/src/model/operation.rs with selector field
- [X] T004 [P] Verify OperationKind::Delete enum variant exists in crates/core/src/model/operation.rs
- [X] T005 [P] Verify existing selector evaluation infrastructure in crates/core/src/dsl/ supports delete integration
- [X] T006 [P] Verify working DataFrame metadata columns (_deleted, _modified_at) in /workspace/specs/007-delete-operation/data-model.md
- [X] T007 Create new module file crates/core/src/operations/delete.rs with pub mod export

**Checkpoint**: Foundation ready - user story implementation can now begin in parallel

---

## Phase 3: User Story 1 - Soft-delete matching rows (Priority: P1) MVP

**Goal**: Mark rows that match a business rule as deleted so they no longer affect downstream calculations

**Independent Test**: Execute a pipeline with a delete step using a selector and verify matching rows are marked deleted while non-matching rows remain unchanged

### Tests for User Story 1 (MANDATORY - TDD Principle I)

> **NOTE: Write these tests FIRST, ensure they FAIL before implementation**

- [X] T008 [P] [US1] Create unit test file crates/core/tests/unit/operations/test_delete.rs with module structure
- [X] T009 [P] [US1] Write failing test test_delete_with_selector_marks_matching_rows in crates/core/tests/unit/operations/test_delete.rs
- [X] T010 [P] [US1] Write failing test test_delete_updates_modified_at_timestamp in crates/core/tests/unit/operations/test_delete.rs
- [X] T011 [P] [US1] Write failing test test_delete_with_zero_matches_leaves_unchanged in crates/core/tests/unit/operations/test_delete.rs
- [X] T012 [P] [US1] Write failing test test_delete_already_deleted_rows_no_op in crates/core/tests/unit/operations/test_delete.rs
- [X] T013 [P] [US1] Create contract test scenario crates/test-resolver/tests/scenarios/delete_selective.yaml for selective deletion
- [X] T014 [P] [US1] Create integration test file crates/core/tests/integration/test_pipeline_with_delete.rs with test_deleted_rows_excluded_from_subsequent_operations

### Implementation for User Story 1

- [X] T015 [US1] Implement execute_delete function in crates/core/src/operations/delete.rs with selector parameter parsing
- [X] T016 [US1] Integrate compile_selector function with existing DSL parser in crates/core/src/operations/delete.rs
- [X] T017 [US1] Implement metadata update logic (_deleted and _modified_at) in crates/core/src/operations/delete.rs using Polars with_column
- [X] T018 [US1] Add delete operation execution to pipeline executor in crates/core/src/execution/pipeline.rs with OperationKind::Delete match arm
- [X] T019 [US1] Implement automatic deleted row filtering for non-output operations in crates/core/src/execution/pipeline.rs
- [X] T020 [US1] Verify unit tests in crates/core/tests/unit/operations/test_delete.rs pass after implementation
- [X] T021 [US1] Verify contract test scenario crates/test-resolver/tests/scenarios/delete_selective.yaml passes after implementation
- [X] T022 [US1] Verify integration test crates/core/tests/integration/test_pipeline_with_delete.rs passes after implementation

**Checkpoint**: At this point, User Story 1 should be fully functional and testable independently

---

## Phase 4: User Story 2 - Delete all active rows when no selector is provided (Priority: P2)

**Goal**: Support "delete all active rows" behavior when no selector is provided for reset and purge workflows

**Independent Test**: Execute a pipeline with a delete step that has no selector and verify all currently active rows are marked deleted

### Tests for User Story 2 (MANDATORY - TDD Principle I)

- [X] T023 [P] [US2] Write failing test test_delete_without_selector_marks_all_active_rows in crates/core/tests/unit/operations/test_delete.rs
- [X] T024 [P] [US2] Write failing test test_delete_no_selector_preserves_already_deleted in crates/core/tests/unit/operations/test_delete.rs
- [X] T025 [P] [US2] Create contract test scenario crates/test-resolver/tests/scenarios/delete_all.yaml for delete-all behavior

### Implementation for User Story 2

- [X] T026 [US2] Implement no-selector default behavior (selector = None -> lit(true)) in crates/core/src/operations/delete.rs
- [X] T027 [US2] Add validation to ensure no-selector delete only affects active rows in crates/core/src/operations/delete.rs
- [X] T028 [US2] Verify unit tests in crates/core/tests/unit/operations/test_delete.rs pass after implementation
- [X] T029 [US2] Verify contract test scenario crates/test-resolver/tests/scenarios/delete_all.yaml passes after implementation

**Checkpoint**: At this point, User Stories 1 AND 2 should both work independently

---

## Phase 5: User Story 3 - Respect deleted-row visibility rules in outputs (Priority: P3)

**Goal**: Default outputs exclude deleted rows so published results contain only active records unless explicitly requested otherwise

**Independent Test**: Execute a pipeline with delete followed by output and verify deleted rows are excluded by default and can be included only when explicitly requested by output settings

### Tests for User Story 3 (MANDATORY - TDD Principle I)

- [X] T030 [P] [US3] Write failing test test_output_excludes_deleted_rows_by_default in crates/core/tests/unit/operations/test_output.rs
- [X] T031 [P] [US3] Write failing test test_output_includes_deleted_when_requested in crates/core/tests/unit/operations/test_output.rs
- [X] T032 [P] [US3] Create contract test scenario crates/test-resolver/tests/scenarios/delete_output_visibility.yaml for output visibility control

### Implementation for User Story 3

- [X] T033 [US3] Extend OutputOperationParams with include_deleted field in crates/core/src/model/operation.rs
- [X] T034 [US3] Implement conditional filtering logic in execute_output function in crates/core/src/operations/output.rs
- [X] T035 [US3] Add default behavior (include_deleted = false) to output parameter deserialization in crates/core/src/operations/output.rs
- [X] T036 [US3] Verify unit tests in crates/core/tests/unit/operations/test_output.rs pass after implementation
- [X] T037 [US3] Verify contract test scenario crates/test-resolver/tests/scenarios/delete_output_visibility.yaml passes after implementation

**Checkpoint**: All user stories should now be independently functional

---

## Phase 6: Polish & Cross-Cutting Concerns

**Purpose**: Improvements that affect multiple user stories

- [X] T038 [P] Add comprehensive error handling for invalid selectors in crates/core/src/operations/delete.rs with clear error messages
- [X] T039 [P] Add validation for selector boolean type checking in crates/core/src/validation/
- [X] T040 [P] Add validation for named selector references ({{NAME}}) in crates/core/src/validation/
- [X] T041 [P] Update API schema documentation in docs/ with delete operation examples
- [X] T042 [P] Add delete operation examples to quickstart documentation in docs/quickstart.md
- [X] T043 Run full test suite from /workspace/Cargo.toml with cargo test to verify all tests pass
- [X] T044 Run clippy from /workspace/Cargo.toml with cargo clippy to verify code quality
- [X] T045 [P] Run performance benchmarks for 10k/100k/1M row delete operations
- [X] T046 [P] Add tracing/logging for delete operations in crates/core/src/operations/delete.rs
- [X] T047 Verify scenarios from /workspace/specs/007-delete-operation/quickstart.md execute with crates/test-resolver/tests/scenarios/

---

## Dependencies & Execution Order

### Phase Dependencies

- **Setup (Phase 1)**: No dependencies - can start immediately
- **Foundational (Phase 2)**: Depends on Setup completion - BLOCKS all user stories
- **User Stories (Phase 3-5)**: All depend on Foundational phase completion
  - User stories can then proceed in parallel (if staffed)
  - Or sequentially in priority order (P1 -> P2 -> P3)
- **Polish (Phase 6)**: Depends on all desired user stories being complete

### User Story Dependencies

- **User Story 1 (P1)**: Can start after Foundational (Phase 2) - No dependencies on other stories
- **User Story 2 (P2)**: Can start after Foundational (Phase 2) - Builds on US1 execute_delete function but independently testable
- **User Story 3 (P3)**: Can start after Foundational (Phase 2) - Independent output configuration, no dependency on US1/US2

### Within Each User Story

- Tests MUST be written and FAIL before implementation
- Unit tests -> Integration tests -> Contract tests (can run in parallel)
- Core implementation (execute_delete) before pipeline integration
- Pipeline integration before validation
- Story complete before moving to next priority

### Parallel Opportunities

- Setup verification tasks can run in parallel (T001-T002)
- All Foundational tasks marked [P] can run in parallel (T004-T006)
- Once Foundational phase completes, all user stories can start in parallel (if team capacity allows)
- All tests for a user story marked [P] can run in parallel (T008-T014 for US1)
- Polish tasks marked [P] can run in parallel (T038-T042, T045-T046)

---

## Parallel Example: User Story 1

```bash
# Launch all tests for User Story 1 together (MANDATORY per TDD principle):
Task T008: "Create unit test file crates/core/tests/unit/operations/test_delete.rs"
Task T009: "Write failing test test_delete_with_selector_marks_matching_rows"
Task T010: "Write failing test test_delete_updates_modified_at_timestamp"
Task T011: "Write failing test test_delete_with_zero_matches_leaves_unchanged"
Task T012: "Write failing test test_delete_already_deleted_rows_no_op"
Task T013: "Create contract test scenario delete_selective.yaml"
Task T014: "Create integration test file test_pipeline_with_delete.rs"

# After tests fail, implement core functionality:
Task T015: "Implement execute_delete function"
Task T016: "Integrate compile_selector with DSL parser"
Task T017: "Implement metadata update logic"

# Then integrate with pipeline (sequential):
Task T018: "Add delete operation to pipeline executor"
Task T019: "Implement automatic deleted row filtering"

# Finally verify (can run in parallel):
Task T020: "Verify unit tests pass"
Task T021: "Verify contract test passes"
Task T022: "Verify integration test passes"
```

---

## Parallel Example: User Story 2

```bash
# Launch all tests for User Story 2 together:
Task T023: "Write failing test test_delete_without_selector_marks_all_active_rows"
Task T024: "Write failing test test_delete_no_selector_preserves_already_deleted"
Task T025: "Create contract test scenario delete_all.yaml"

# Implement no-selector behavior (extends existing execute_delete):
Task T026: "Implement no-selector default behavior"
Task T027: "Add validation for no-selector delete"

# Verify tests pass:
Task T028: "Verify unit tests pass"
Task T029: "Verify contract test passes"
```

---

## Parallel Example: User Story 3

```bash
# Launch all tests for User Story 3 together:
Task T030: "Write failing test test_output_excludes_deleted_rows_by_default"
Task T031: "Write failing test test_output_includes_deleted_when_requested"
Task T032: "Create contract test scenario delete_output_visibility.yaml"

# Implement output filtering (independent from delete operation):
Task T033: "Extend OutputOperationParams with include_deleted field"
Task T034: "Implement conditional filtering in execute_output"
Task T035: "Add default behavior to output parameter deserialization"

# Verify tests pass:
Task T036: "Verify unit tests pass"
Task T037: "Verify contract test passes"
```

---

## Implementation Strategy

### MVP First (User Story 1 Only)

1. Complete Phase 1: Setup (T001-T002)
2. Complete Phase 2: Foundational (T003-T007) - **CRITICAL - blocks all stories**
3. Complete Phase 3: User Story 1 (T008-T022)
4. **STOP and VALIDATE**: Test User Story 1 independently with cargo test
5. Deploy/demo if ready

**MVP Deliverable**: Basic delete operation with selector-based row filtering, automatic exclusion from downstream operations

### Incremental Delivery

1. Complete Setup + Foundational -> Foundation ready
2. Add User Story 1 (T008-T022) -> Test independently -> Deploy/Demo (MVP!)
3. Add User Story 2 (T023-T029) -> Test independently -> Deploy/Demo (adds delete-all capability)
4. Add User Story 3 (T030-T037) -> Test independently -> Deploy/Demo (adds output control)
5. Add Polish (T038-T047) -> Final production-ready release

Each story adds value without breaking previous stories.

### Parallel Team Strategy

With multiple developers:

1. Team completes Setup + Foundational together (T001-T007)
2. Once Foundational is done:
   - Developer A: User Story 1 (T008-T022) - Core delete functionality
   - Developer B: User Story 2 (T023-T029) - No-selector behavior
   - Developer C: User Story 3 (T030-T037) - Output visibility control
3. Stories complete and integrate independently
4. All developers: Polish tasks (T038-T047) in parallel

---

## Notes

- [P] tasks = different files, no dependencies
- [Story] label maps task to specific user story for traceability
- Each user story should be independently completable and testable
- Verify tests fail before implementing (Red-Green-Refactor TDD cycle)
- Commit after each task or logical group
- Stop at any checkpoint to validate story independently
- Avoid: vague tasks, same file conflicts, cross-story dependencies that break independence

---

## Task Summary

**Total Tasks**: 47

### By Phase:
- Phase 1 (Setup): 2 tasks
- Phase 2 (Foundational): 5 tasks (BLOCKING)
- Phase 3 (User Story 1 - P1): 15 tasks (7 tests + 8 implementation)
- Phase 4 (User Story 2 - P2): 7 tasks (3 tests + 4 implementation)
- Phase 5 (User Story 3 - P3): 8 tasks (3 tests + 5 implementation)
- Phase 6 (Polish): 10 tasks

### By Type:
- Tests (TDD): 13 unit/integration/contract tests (MANDATORY before implementation)
- Implementation: 27 tasks
- Validation/Polish: 7 tasks

### Parallel Opportunities:
- 23 tasks marked [P] can run in parallel with other tasks in same phase
- All 3 user stories can run in parallel after Foundational phase complete
- Test-writing tasks can all run in parallel within each user story

### Suggested MVP Scope:
- Phase 1 (Setup): T001-T002
- Phase 2 (Foundational): T003-T007
- Phase 3 (User Story 1): T008-T022
- **Total MVP**: 22 tasks

This delivers core delete functionality with selector-based filtering and automatic exclusion from downstream operations, meeting the primary business requirement.

---

## Format Validation

PASS: **All tasks follow strict checklist format**: `- [ ] [ID] [P?] [Story?] Description with file path`

PASS: **Task IDs**: Sequential (T001-T047) in execution order

PASS: **[P] markers**: 23 tasks marked as parallelizable (different files, no dependencies)

PASS: **[Story] labels**: All user story phase tasks have [US1], [US2], or [US3] labels

PASS: **File paths**: All implementation/test tasks include exact file paths

PASS: **Test-first**: All user story phases start with test tasks before implementation

PASS: **Independent stories**: Each user story can be implemented and tested independently after Foundational phase
