---

description: "Task list for Runtime Join Resolution feature implementation"
---

# Tasks: Runtime Join Resolution

**Input**: Design documents from `/specs/006-runtime-join/`
**Prerequisites**: plan.md, spec.md, research.md, data-model.md, contracts/runtime_join_schema.yaml, quickstart.md

**Tests**: Per constitutional principle I (TDD), all tasks include tests written BEFORE implementation. Tests are MANDATORY and NON-NEGOTIABLE.

**Organization**: Tasks are grouped by user story to enable independent implementation and testing of each story.

## Format: `[ID] [P?] [Story] Description`

- **[P]**: Can run in parallel (different files, no dependencies)
- **[Story]**: Which user story this task belongs to (e.g., US1, US2, US3)
- Include exact file paths in descriptions

## Path Conventions

Repository structure:
- **Core library**: `crates/core/src/` (engine, model, resolver)
- **Tests**: `crates/core/tests/` (contracts/, unit/, integration/)
- **Test utilities**: `crates/test-resolver/src/` (InMemoryDataLoader)

---

## Phase 1: Setup (Shared Infrastructure)

**Purpose**: Project initialization and basic structure for RuntimeJoin feature

- [X] T001 Create RuntimeJoin data structure in crates/core/src/model/operation.rs
- [X] T002 Extend UpdateOperation with joins argument in crates/core/src/model/operation.rs
- [X] T003 [P] Extend ResolverSnapshot with join_datasets map in crates/core/src/model/run.rs
- [X] T004 [P] Create join module structure in crates/core/src/engine/join.rs

---

## Phase 2: Foundational (Blocking Prerequisites)

**Purpose**: Core infrastructure that MUST be complete before ANY user story can be implemented

**⚠️ CRITICAL**: No user story work can begin until this phase is complete

- [X] T005 Implement resolver precedence helper (Project override → Dataset resolver_id → system default) in crates/core/src/engine/join.rs
- [X] T006 Implement dataset version resolution (pinned vs latest active) in crates/core/src/engine/join.rs
- [X] T007 [P] Define JoinError enum with variants (DatasetNotFound, DatasetDisabled, UnknownColumn, ResolverFailed) in crates/core/src/engine/join.rs
- [X] T008 [P] Create period filter module stub in crates/core/src/engine/period_filter.rs
- [X] T009 Implement apply_period_filter function supporting both period and bitemporal modes in crates/core/src/engine/period_filter.rs

**Checkpoint**: Foundation ready - user story implementation can now begin in parallel

---

## Phase 3: User Story 1 - Execute FX Conversion Join (Priority: P1) 🎯 MVP

**Goal**: Enable update operations to automatically join exchange rate data and compute reporting currency amounts for multi-currency transactions

**Independent Test**: Provide GL transactions in USD/EUR/GBP/JPY and a bitemporal exchange_rates dataset. Execute update with RuntimeJoin on currency match. Verify output contains correctly converted amounts using asOf rates for run period 2026-01 (EUR 1.0920, GBP 1.2710, JPY 0.00672, USD 1.0000).

### Tests for User Story 1 (MANDATORY - TDD Principle I) ⚠️

> **NOTE: Write these tests FIRST, ensure they FAIL before implementation**

- [X] T010 [P] [US1] Contract test for RuntimeJoin schema validation in crates/core/tests/runtime_join_contract.rs
- [X] T011 [P] [US1] Contract test for UpdateArguments with joins array in crates/core/tests/runtime_join_contract.rs
- [X] T012 [P] [US1] Unit test for dataset version resolution (pinned version) in crates/core/tests/engine_join_tests.rs
- [X] T013 [P] [US1] Unit test for dataset version resolution (latest active) in crates/core/tests/engine_join_tests.rs
- [X] T014 [P] [US1] Unit test for period filter with bitemporal mode (asOf query) in crates/core/tests/period_filter_tests.rs
- [X] T015 [P] [US1] Unit test for period filter with period mode (exact match) in crates/core/tests/period_filter_tests.rs
- [X] T016 [US1] Integration test TS-03 FX conversion with EUR/GBP/JPY/USD in crates/core/tests/integration/ts03_fx_conversion.rs

### Implementation for User Story 1

- [X] T017 [P] [US1] Implement resolve_and_load_join function signature in crates/core/src/engine/join.rs
- [X] T018 [US1] Implement dataset resolution logic (version pinning vs latest) in crates/core/src/engine/join.rs
- [X] T019 [US1] Implement resolver precedence chain in crates/core/src/engine/join.rs
- [X] T020 [US1] Implement DataLoader invocation for join dataset in crates/core/src/engine/join.rs
- [X] T021 [US1] Integrate period filter application based on temporal_mode in crates/core/src/engine/join.rs
- [X] T022 [US1] Implement Polars LazyFrame join with suffix using JoinType::Left in crates/core/src/engine/join.rs
- [X] T023 [US1] Add resolver snapshot tracking for join_datasets map in crates/core/src/engine/join.rs
- [X] T024 [P] [US1] Create test fixtures for GL dataset in crates/core/tests/fixtures/sample_datasets.rs
- [X] T025 [P] [US1] Create test fixtures for bitemporal exchange rates in crates/core/tests/fixtures/sample_datasets.rs
- [X] T026 [US1] Seed InMemoryDataLoader with TS-03 scenario data in crates/core/tests/integration/ts03_fx_conversion.rs
- [X] T027 [US1] Implement end-to-end FX conversion test verification in crates/core/tests/integration/ts03_fx_conversion.rs

**Checkpoint**: At this point, User Story 1 should be fully functional - single RuntimeJoin executes correctly with bitemporal filtering and FX conversion works end-to-end

---

## Phase 4: User Story 3 - Apply Correct Period Filtering per Temporal Mode (Priority: P1)

**Goal**: Ensure join datasets are filtered using their own temporal_mode configuration so bitemporal joins return asOf snapshots and period joins return exact matches for the run period

**Independent Test**: Execute join against bitemporal exchange_rates table for period 2026-01. Verify asOf query selects rates where _period_from <= 2026-01-01 AND (_period_to IS NULL OR _period_to > 2026-01-01), returning 2026-01-01 rate (1.0920 for EUR) not 2025-01-01 rate (1.0850).

### Tests for User Story 3 (MANDATORY - TDD Principle I) ⚠️

- [X] T028 [P] [US3] Unit test for bitemporal filter with multiple rate versions in crates/core/tests/unit/period_filter_tests.rs
- [X] T029 [P] [US3] Unit test for period filter with exact match validation in crates/core/tests/unit/period_filter_tests.rs
- [X] T030 [P] [US3] Unit test for join with no matching period data (NULL results) in crates/core/tests/unit/engine_join_tests.rs
- [X] T031 [US3] Integration test for bitemporal asOf rate selection accuracy in crates/core/tests/integration/bitemporal_filtering_tests.rs

### Implementation for User Story 3

- [X] T032 [P] [US3] Implement bitemporal asOf predicate logic (_period_from <= start_date AND _period_to > start_date OR IS NULL) in crates/core/src/engine/period_filter.rs
- [X] T033 [P] [US3] Implement period exact match predicate logic (_period = period_identifier) in crates/core/src/engine/period_filter.rs
- [X] T034 [US3] Add temporal_mode detection from TableRef in crates/core/src/engine/join.rs
- [X] T035 [US3] Integrate temporal_mode-based filter selection in resolve_and_load_join in crates/core/src/engine/join.rs
- [X] T036 [US3] Add logging for period filter applied (temporal_mode, period, row count) in crates/core/src/engine/period_filter.rs

**Checkpoint**: At this point, both User Story 1 AND User Story 3 should work - temporal filtering is correct for both modes, verified via tests showing correct rate selection

---

## Phase 5: User Story 2 - Support Multiple Independent Joins (Priority: P2)

**Goal**: Enable single update operation to support multiple RuntimeJoins so working data can be enriched from multiple external sources in one step

**Independent Test**: Define update operation with two RuntimeJoins (customers for tier, products for category). Execute operation and verify both join aliases are available in assignment expressions and correct enriched values appear in output.

### Tests for User Story 2 (MANDATORY - TDD Principle I) ⚠️

- [X] T037 [P] [US2] Contract test for multiple RuntimeJoins in single operation in crates/core/tests/contracts/runtime_join_contract.rs
- [X] T038 [P] [US2] Unit test for alias uniqueness validation in crates/core/tests/unit/engine_join_tests.rs
- [X] T039 [P] [US2] Unit test for multi-join sequential application in crates/core/tests/unit/engine_join_tests.rs
- [X] T040 [US2] Integration test for dual join (customers + products) in crates/core/tests/integration/multi_join_tests.rs

### Implementation for User Story 2

- [X] T041 [P] [US2] Implement alias uniqueness validation in crates/core/src/engine/join.rs
- [X] T042 [P] [US2] Implement alias conflict detection with working dataset tables in crates/core/src/engine/join.rs
- [X] T043 [US2] Implement sequential join application loop in crates/core/src/engine/join.rs
- [X] T044 [US2] Add error handling for join resolution failures with clear dataset identification in crates/core/src/engine/join.rs
- [X] T045 [P] [US2] Create test fixtures for customers dataset in crates/core/tests/fixtures/sample_datasets.rs
- [X] T046 [P] [US2] Create test fixtures for products dataset in crates/core/tests/fixtures/sample_datasets.rs
- [X] T047 [US2] Implement multi-join test with customers and products enrichment in crates/core/tests/integration/multi_join_tests.rs

**Checkpoint**: At this point, User Stories 1, 2, AND 3 should all work independently - multiple joins execute correctly with mixed temporal modes

---

## Phase 6: User Story 4 - Resolve Dataset via Resolver with Correct Precedence (Priority: P2)

**Goal**: Ensure RuntimeJoin datasets use same resolver precedence as input dataset (Project resolver_overrides → Dataset resolver_id → system default) so join data sources can be controlled consistently

**Independent Test**: Configure Project with resolver_overrides pointing to test resolver. Execute update with RuntimeJoin to dataset with different resolver_id. Verify Project's resolver_override takes precedence and join data loads from test resolver.

### Tests for User Story 4 (MANDATORY - TDD Principle I) ⚠️

- [X] T048 [P] [US4] Unit test for resolver precedence: Project override wins in crates/core/tests/unit/engine_join_tests.rs
- [X] T049 [P] [US4] Unit test for resolver precedence: Dataset resolver_id fallback in crates/core/tests/unit/engine_join_tests.rs
- [X] T050 [P] [US4] Unit test for resolver precedence: System default fallback in crates/core/tests/unit/engine_join_tests.rs
- [X] T051 [US4] Integration test for resolver override in test vs production environments in crates/core/tests/integration/resolver_precedence_tests.rs

### Implementation for User Story 4

- [X] T052 [P] [US4] Implement Project.resolver_overrides lookup in crates/core/src/engine/join.rs
- [X] T053 [P] [US4] Implement Dataset.resolver_id fallback logic in crates/core/src/engine/join.rs
- [X] T054 [US4] Implement system default resolver selection in crates/core/src/engine/join.rs
- [X] T055 [US4] Add resolver source tracking to ResolverSnapshot in crates/core/src/engine/join.rs
- [X] T056 [US4] Implement resolver precedence integration test with mock resolvers in crates/core/tests/integration/resolver_precedence_tests.rs

**Checkpoint**: All user stories should now be independently functional - resolver configuration works consistently across all join scenarios

---

## Phase 7: Edge Cases & Error Handling

**Purpose**: Comprehensive error handling and edge case validation across all user stories

- [X] T057 [P] Unit test for RuntimeJoin with nonexistent dataset_id in crates/core/tests/unit/engine_join_tests.rs
- [X] T058 [P] Unit test for RuntimeJoin with disabled dataset in crates/core/tests/unit/engine_join_tests.rs
- [X] T059 [P] Unit test for RuntimeJoin with unknown column in on expression in crates/core/tests/unit/engine_join_tests.rs
- [X] T060 [P] Unit test for assignment expression with unknown join alias in crates/core/tests/unit/engine_join_tests.rs
- [X] T061 [P] Unit test for assignment expression with unknown join column in crates/core/tests/unit/engine_join_tests.rs
- [X] T062 [P] Unit test for update operation with zero RuntimeJoins (valid baseline) in crates/core/tests/unit/engine_join_tests.rs
- [X] T063 Implement dataset existence validation with clear error messages in crates/core/src/engine/join.rs
- [X] T064 [P] Implement dataset status validation (must be active) in crates/core/src/engine/join.rs
- [X] T065 [P] Implement column reference validation for on expressions in crates/core/src/engine/join.rs
- [X] T066 [P] Implement join alias validation for assignment expressions in crates/core/src/engine/join.rs

---

## Phase 8: Expression Compilation Integration

**Purpose**: Enable `alias.column_name` syntax in assignment expressions

- [X] T067 [P] Unit test for expression compiler with join alias symbol table in crates/core/tests/unit/expression_compiler_tests.rs
- [X] T068 [P] Unit test for alias.column mapping to suffixed Polars columns in crates/core/tests/unit/expression_compiler_tests.rs
- [X] T069 Extend expression compiler symbol table with join aliases in crates/core/src/dsl/compiler.rs
- [X] T070 Implement alias.column reference parsing and mapping in crates/core/src/dsl/compiler.rs
- [X] T071 Add compile-time error for unknown alias or column in crates/core/src/dsl/compiler.rs
- [X] T072 Integration test for assignment expression with multiple join references in crates/core/tests/integration/expression_integration_tests.rs

---

## Phase 9: Polish & Cross-Cutting Concerns

**Purpose**: Improvements that affect multiple user stories

- [X] T073 [P] Add comprehensive logging for join resolution (dataset, version, resolver, filter applied) in crates/core/src/engine/join.rs
- [X] T074 [P] Add performance metrics for join execution (load time, filter time, join time) in crates/core/src/engine/join.rs
- [X] T075 [P] Update API documentation with RuntimeJoin examples in docs/api/runtime-join.md
- [X] T076 [P] Update operation.md entity documentation with RuntimeJoin specification in docs/entities/operation.md
- [X] T077 Code cleanup: Remove redundant error handling and consolidate validation in crates/core/src/engine/join.rs
- [X] T078 Run cargo fmt and cargo clippy on all modified files
- [X] T079 Run full test suite with cargo test --workspace
- [X] T080 Execute quickstart.md validation steps end-to-end

---

## Dependencies & Execution Order

### Phase Dependencies

- **Setup (Phase 1)**: No dependencies - can start immediately
- **Foundational (Phase 2)**: Depends on Setup completion - BLOCKS all user stories
- **User Stories (Phase 3-6)**: All depend on Foundational phase completion
  - User Story 1 (P1): Can start after Foundational - No dependencies on other stories
  - User Story 3 (P1): Can start after Foundational - No dependencies on other stories (can run parallel with US1)
  - User Story 2 (P2): Can start after Foundational - No dependencies on other stories
  - User Story 4 (P2): Can start after Foundational - No dependencies on other stories
- **Edge Cases (Phase 7)**: Can start after any user story completes - tests error paths
- **Expression Integration (Phase 8)**: Depends on User Story 1 completion (needs basic join working)
- **Polish (Phase 9)**: Depends on all desired user stories being complete

### User Story Dependencies

- **User Story 1 (P1)**: Can start after Foundational (Phase 2) - No dependencies on other stories
- **User Story 3 (P1)**: Can start after Foundational (Phase 2) - Can run in parallel with US1
- **User Story 2 (P2)**: Can start after Foundational (Phase 2) - Builds on US1 join mechanics but independently testable
- **User Story 4 (P2)**: Can start after Foundational (Phase 2) - Builds on US1 resolution but independently testable

### Within Each User Story

- Tests MUST be written and FAIL before implementation
- Core join logic before multi-join support
- Basic filtering before temporal mode variations
- Resolver logic before precedence chain
- Story complete before moving to next priority

### Parallel Opportunities

- All Setup tasks marked [P] can run in parallel (T003, T004)
- All Foundational tasks marked [P] can run in parallel (T007, T008)
- Once Foundational phase completes:
  - User Story 1 and User Story 3 can be implemented in parallel (both P1, independent)
  - User Story 2 and User Story 4 can be implemented in parallel (both P2, independent)
- All tests for a user story marked [P] can run in parallel
- Test fixture creation tasks can run in parallel (T024, T025, T045, T046)
- Different user stories can be worked on in parallel by different team members

---

## Parallel Example: User Story 1

```bash
# Launch all tests for User Story 1 together (MANDATORY per TDD principle):
Task T010: "Contract test for RuntimeJoin schema validation in crates/core/tests/contracts/runtime_join_contract.rs"
Task T011: "Contract test for UpdateArguments with joins array in crates/core/tests/contracts/runtime_join_contract.rs"
Task T012: "Unit test for dataset version resolution (pinned version) in crates/core/tests/unit/engine_join_tests.rs"
Task T013: "Unit test for dataset version resolution (latest active) in crates/core/tests/unit/engine_join_tests.rs"
Task T014: "Unit test for period filter with bitemporal mode (asOf query) in crates/core/tests/unit/period_filter_tests.rs"
Task T015: "Unit test for period filter with period mode (exact match) in crates/core/tests/unit/period_filter_tests.rs"

# Launch test fixtures together:
Task T024: "Create test fixtures for GL dataset in crates/core/tests/fixtures/sample_datasets.rs"
Task T025: "Create test fixtures for bitemporal exchange rates in crates/core/tests/fixtures/sample_datasets.rs"
```

---

## Parallel Example: User Story 2

```bash
# Launch all tests for User Story 2 together:
Task T037: "Contract test for multiple RuntimeJoins in single operation in crates/core/tests/contracts/runtime_join_contract.rs"
Task T038: "Unit test for alias uniqueness validation in crates/core/tests/unit/engine_join_tests.rs"
Task T039: "Unit test for multi-join sequential application in crates/core/tests/unit/engine_join_tests.rs"

# Launch test fixtures together:
Task T045: "Create test fixtures for customers dataset in crates/core/tests/fixtures/sample_datasets.rs"
Task T046: "Create test fixtures for products dataset in crates/core/tests/fixtures/sample_datasets.rs"
```

---

## Implementation Strategy

### MVP First (User Story 1 + User Story 3 Only)

1. Complete Phase 1: Setup
2. Complete Phase 2: Foundational (CRITICAL - blocks all stories)
3. Complete Phase 3: User Story 1 (FX conversion join)
4. Complete Phase 4: User Story 3 (period filtering correctness)
5. **STOP and VALIDATE**: Test both stories independently using TS-03 scenario
6. Deploy/demo if ready

**MVP Scope**: Single RuntimeJoin with correct temporal filtering - enables core FX conversion use case with bitemporal exchange rates.

### Incremental Delivery

1. Complete Setup + Foundational → Foundation ready
2. Add User Story 1 + User Story 3 → Test independently → Deploy/Demo (MVP!)
3. Add User Story 2 → Test multi-join scenarios → Deploy/Demo
4. Add User Story 4 → Test resolver precedence → Deploy/Demo
5. Add Edge Cases + Expression Integration → Harden error handling → Deploy/Demo
6. Add Polish → Final release

### Parallel Team Strategy

With multiple developers:

1. Team completes Setup + Foundational together (T001-T009)
2. Once Foundational is done:
   - **Developer A**: User Story 1 (T010-T027) - Core join execution
   - **Developer B**: User Story 3 (T028-T036) - Period filtering
   - **Developer C**: User Story 2 (T037-T047) - Multiple joins
   - **Developer D**: User Story 4 (T048-T056) - Resolver precedence
3. Stories complete and integrate independently
4. Team collaborates on Edge Cases (T057-T066)
5. Team collaborates on Polish (T073-T080)

---

## Notes

- [P] tasks = different files, no dependencies
- [Story] label maps task to specific user story for traceability
- Each user story should be independently completable and testable
- Verify tests fail before implementing
- Commit after each task or logical group
- Stop at any checkpoint to validate story independently
- Avoid: vague tasks, same file conflicts, cross-story dependencies that break independence
- Constitutional compliance: All tests written FIRST (TDD Principle I), all tasks have clear validation criteria
