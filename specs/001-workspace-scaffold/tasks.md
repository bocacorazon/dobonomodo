# Tasks: Workspace Scaffold Baseline

**Input**: Design documents from `/specs/001-workspace-scaffold/`
**Prerequisites**: plan.md (required), spec.md (required for user stories), research.md, data-model.md, contracts/

**Tests**: Per constitution principle I (TDD), tests are mandatory and must be written before implementation tasks in each story phase.

**Organization**: Tasks are grouped by user story to enable independent implementation and testing of each story.

## Phase 1: Setup (Shared Infrastructure)

**Purpose**: Initialize workspace scaffolding and repository structure required by all stories.

- [x] T001 Create workspace manifest with member crate declarations in Cargo.toml
- [x] T002 [P] Create crate manifests for core and binaries in crates/core/Cargo.toml, crates/api-server/Cargo.toml, crates/engine-worker/Cargo.toml, crates/cli/Cargo.toml, crates/test-resolver/Cargo.toml
- [x] T003 [P] Create core module directory structure and module placeholders in crates/core/src/model/mod.rs, crates/core/src/dsl/mod.rs, crates/core/src/engine/mod.rs, crates/core/src/resolver/mod.rs, crates/core/src/trace/mod.rs, crates/core/src/validation/mod.rs
- [x] T004 [P] Create runtime entrypoints for executable crates in crates/api-server/src/main.rs, crates/engine-worker/src/main.rs, crates/cli/src/main.rs
- [x] T005 [P] Create test-resolver library entrypoint in crates/test-resolver/src/lib.rs

---

## Phase 2: Foundational (Blocking Prerequisites)

**Purpose**: Establish shared contracts and test fixtures required before story implementation.

**⚠️ CRITICAL**: No user story work can begin until this phase is complete.

- [x] T006 Create core crate root module and public exports scaffold in crates/core/src/lib.rs
- [x] T007 [P] Add shared fixture directory and baseline YAML fixture file in crates/core/tests/fixtures/entities.yaml
- [x] T008 [P] Add shared fixture directory and baseline JSON fixture file in crates/core/tests/fixtures/entities.json
- [x] T009 Create shared test helper loader utilities in crates/core/tests/common/mod.rs
- [x] T010 Create foundational compile/import smoke test in crates/core/tests/foundation_compile.rs

**Checkpoint**: Foundation ready — user story implementation can now begin.

---

## Phase 3: User Story 1 - Initialize Project Skeleton (Priority: P1) 🎯 MVP

**Goal**: Deliver complete workspace and crate skeleton that compiles and tests successfully.

**Independent Test**: From a clean checkout, run `cargo build` and `cargo test`; both succeed and all required crate/module entrypoints are present.

### Tests for User Story 1 (MANDATORY - TDD)

- [x] T011 [P] [US1] Add workspace structure verification test in crates/core/tests/us1_workspace_structure.rs
- [x] T012 [P] [US1] Add scaffold contract test for `/v1/scaffold/validate` expectations in crates/core/tests/contracts/us1_scaffold_validate_contract.rs

### Implementation for User Story 1

- [x] T013 [US1] Finalize workspace dependency declarations and shared package metadata in Cargo.toml
- [x] T014 [US1] Wire crate-level dependencies for scaffold-only compile targets in crates/core/Cargo.toml, crates/api-server/Cargo.toml, crates/engine-worker/Cargo.toml, crates/cli/Cargo.toml, crates/test-resolver/Cargo.toml
- [x] T015 [US1] Implement minimal compile-safe runtime entrypoints in crates/api-server/src/main.rs, crates/engine-worker/src/main.rs, crates/cli/src/main.rs
- [x] T016 [US1] Implement minimal compile-safe library entrypoint for test resolver in crates/test-resolver/src/lib.rs
- [x] T017 [US1] Ensure US1 verification commands are documented for contributors in specs/001-workspace-scaffold/quickstart.md

**Checkpoint**: User Story 1 is independently functional and testable.

---

## Phase 4: User Story 2 - Use Shared Domain Contracts (Priority: P2)

**Goal**: Provide shared entity structs and enums in core, with YAML/JSON deserialization coverage.

**Independent Test**: Domain types compile, are re-exported by `core`, and deserialize from representative YAML and JSON fixtures.

### Tests for User Story 2 (MANDATORY - TDD)

- [x] T018 [P] [US2] Add YAML deserialization tests for required entities in crates/core/tests/us2_deserialize_yaml.rs
- [x] T019 [P] [US2] Add JSON deserialization tests for required entities in crates/core/tests/us2_deserialize_json.rs
- [x] T020 [P] [US2] Add enum roundtrip/validation tests for lifecycle and behavior enums in crates/core/tests/us2_enum_contracts.rs
- [x] T021 [P] [US2] Add contract test for `/v1/entities/deserialize/validate` expectations in crates/core/tests/contracts/us2_entity_deserialize_contract.rs

### Implementation for User Story 2

- [x] T022 [P] [US2] Implement dataset-related model structs and enums in crates/core/src/model/dataset.rs
- [x] T023 [P] [US2] Implement project and operation model structs and enums in crates/core/src/model/project.rs and crates/core/src/model/operation.rs
- [x] T024 [P] [US2] Implement run snapshot/status model structs and enums in crates/core/src/model/run.rs
- [x] T025 [P] [US2] Implement resolver strategy model structs and enums in crates/core/src/model/resolver.rs
- [x] T026 [P] [US2] Implement calendar/period and datasource model structs in crates/core/src/model/calendar.rs, crates/core/src/model/period.rs, crates/core/src/model/datasource.rs
- [x] T027 [P] [US2] Implement expression newtype/value object in crates/core/src/model/expression.rs
- [x] T028 [US2] Export all model modules and public domain types from core in crates/core/src/model/mod.rs and crates/core/src/lib.rs

**Checkpoint**: User Stories 1 and 2 both pass independently.

---

## Phase 5: User Story 3 - Integrate IO Through Contracts (Priority: P3)

**Goal**: Define core IO trait contracts with architecture-aligned signatures and compile-safe stubs.

**Independent Test**: All four IO contracts exist, are importable from `core`, and compile without concrete adapter implementations.

### Tests for User Story 3 (MANDATORY - TDD)

- [x] T029 [P] [US3] Add trait signature compile tests for core IO contracts in crates/core/tests/us3_io_traits_compile.rs
- [x] T030 [P] [US3] Add contract test for `/v1/io/contracts` expectations in crates/core/tests/contracts/us3_io_contracts.rs

### Implementation for User Story 3

- [x] T031 [P] [US3] Define frame and contract support types for IO boundaries in crates/core/src/engine/types.rs and crates/core/src/trace/types.rs
- [x] T032 [US3] Define `DataLoader` and `OutputWriter` traits in crates/core/src/engine/io_traits.rs
- [x] T033 [US3] Define `MetadataStore` trait in crates/core/src/model/metadata_store.rs
- [x] T034 [US3] Define `TraceWriter` trait in crates/core/src/trace/trace_writer.rs
- [x] T035 [US3] Re-export IO traits through core public API in crates/core/src/lib.rs

**Checkpoint**: All three user stories are independently functional.

---

## Phase 6: Polish & Cross-Cutting Concerns

**Purpose**: Final verification and cross-story consistency checks.

- [x] T036 [P] Run full workspace build and test gates (`cargo build` and `cargo test`) and document outcomes in specs/001-workspace-scaffold/quickstart.md
- [x] T037 [P] Align generated task and plan references with final file layout in specs/001-workspace-scaffold/plan.md and specs/001-workspace-scaffold/tasks.md
- [x] T038 Verify no out-of-scope runtime implementations were added by reviewing scaffold boundaries in crates/core/src/, crates/api-server/src/, crates/engine-worker/src/, crates/cli/src/

---

## Dependencies & Execution Order

### Phase Dependencies

- **Phase 1 (Setup)**: No dependencies.
- **Phase 2 (Foundational)**: Depends on Phase 1; blocks all story work.
- **Phase 3+ (User Stories)**: Depend on Phase 2 completion.
- **Phase 6 (Polish)**: Depends on all implemented user stories.

### User Story Dependencies

- **US1 (P1)**: Starts after Foundational; no dependency on other stories.
- **US2 (P2)**: Starts after Foundational; depends on US1 crate/module scaffold being present.
- **US3 (P3)**: Starts after Foundational; depends on US1 scaffold and US2 domain types for trait signatures.

### Story Completion Order (Dependency Graph)

1. **US1** → establishes workspace skeleton and verification gates.
2. **US2** → adds shared entity and enum contracts on the scaffold.
3. **US3** → adds IO contract boundaries on top of shared domain contracts.

---

## Parallel Execution Examples

### User Story 1

- Run T011 and T012 in parallel (different test files).
- Run T015 and T016 in parallel (different crates).

### User Story 2

- Run T018, T019, T020, and T021 in parallel (separate test files).
- Run T022 through T027 in parallel by model area (separate files) before T028.

### User Story 3

- Run T029 and T030 in parallel (separate test files).
- Run T032, T033, and T034 in parallel after T031 (different modules), then complete T035.

---

## Implementation Strategy

### MVP First (US1 only)

1. Complete Phase 1 and Phase 2.
2. Complete all US1 tasks (tests first, then implementation).
3. Validate with `cargo build` and `cargo test`.
4. Demo/deploy scaffold baseline as MVP.

### Incremental Delivery

1. Deliver US1 (workspace skeleton + gate pass).
2. Deliver US2 (shared domain contracts + serde validation).
3. Deliver US3 (IO contract interfaces).
4. Run final polish and full gate verification.

### Parallel Team Strategy

1. Team completes Setup + Foundational together.
2. Then split by story or by `[P]` tasks inside each story phase.
3. Merge only after each story meets its independent test criteria.
