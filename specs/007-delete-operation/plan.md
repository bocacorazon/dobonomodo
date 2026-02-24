# Implementation Plan: Delete Operation

**Branch**: `007-delete-operation` | **Date**: 2026-02-22 | **Spec**: [spec.md](./spec.md)
**Input**: Feature specification from `/workspace/specs/007-delete-operation/spec.md`

**Note**: This template is filled in by the `/speckit.plan` command. See `.specify/templates/plan-template.md` for the execution workflow.

## Summary

Implement soft-delete operation type for pipeline execution that marks matching rows with `_deleted = true` metadata flag using selector-based filtering. Deleted rows are automatically excluded from subsequent pipeline operations and from outputs by default. This enables controlled data cleanup without destructive data loss.

## Technical Context

**Language/Version**: Rust 2021 edition  
**Primary Dependencies**: Polars (lazy API for data operations), serde (serialization), uuid (row tracking)  
**Storage**: PostgreSQL (metadata store), Polars DataFrames (in-memory working data)  
**Testing**: cargo test (unit, integration, contract tests via test-resolver crate)  
**Target Platform**: Linux server (Kubernetes Jobs for run orchestration)
**Project Type**: Single project (Cargo workspace with core, api-server, engine-worker, cli, test-resolver crates)  
**Performance Goals**: Process 10k+ rows/second with lazy evaluation, minimize memory allocation  
**Constraints**: Zero-copy where possible, maintain Polars lazy evaluation, no physical row deletion  
**Scale/Scope**: Support pipelines with 100+ operations on datasets up to millions of rows

## Constitution Check

*GATE: Must pass before Phase 0 research. Re-check after Phase 1 design.*

### Pre-Phase 0 Check (PASSED)

**Constitutional Principles to Verify**:

- [x] **Principle I (TDD)**: All implementation tasks will be paired with tests written FIRST (unit tests for selector evaluation, integration tests for pipeline sequencing, contract tests for delete scenarios)
- [x] **Principle II (Quality Gates)**: Existing Rust workspace has cargo build/test/clippy configured; test-resolver crate supports contract testing
- [x] **Principle III (Completion Bias)**: Technical decisions can be made autonomously (selector integration approach, metadata flag handling, Polars filter implementation)
- [x] **Principle IV (Comprehensive Testing)**: Test suite execution will cover unit (selector logic, metadata updates), integration (operation sequencing), and contract (acceptance scenarios from spec)

**Notes**: No constitutional conflicts. All principles can be satisfied with existing infrastructure and TDD approach.

### Post-Phase 1 Check (RE-EVALUATED)

**Constitutional Compliance Verification**:

- [x] **Principle I (TDD)**: [PASS] CONFIRMED
  - `quickstart.md` defines complete TDD workflow with Red-Green-Refactor cycles
  - Unit tests written before `execute_delete()` implementation
  - Integration tests precede pipeline filtering logic
  - Contract tests (YAML scenarios) validate acceptance criteria
  - All test types documented with expected fail -> pass progression

- [x] **Principle II (Quality Gates)**: [PASS] CONFIRMED
  - Design leverages existing `cargo test`, `cargo clippy`, `cargo build` infrastructure
  - Test coverage requirements specified: 100% unit coverage, comprehensive integration coverage
  - Contract test framework (test-resolver) reused for acceptance scenarios
  - No new quality gate infrastructure needed

- [x] **Principle III (Completion Bias)**: [PASS] CONFIRMED
  - All technical decisions made autonomously during research/design phases:
    * Selector integration: Reuse existing DSL parser (no clarification needed)
    * Metadata handling: `_deleted` boolean + `_modified_at` timestamp (standard pattern)
    * Polars API: `.with_column()` for metadata updates (best practice identified)
    * Pipeline filtering: Automatic post-operation filtering (design complete)
  - Zero blocking questions remaining for implementation

- [x] **Principle IV (Comprehensive Testing)**: [PASS] CONFIRMED
  - Test pyramid fully defined in `quickstart.md`:
    * Unit: `execute_delete()`, selector compilation, edge cases
    * Integration: Multi-operation pipelines, deleted row filtering
    * Contract: All 3 user stories from spec (selective delete, delete all, output visibility)
  - Test execution strategy documented: `cargo test` for all levels
  - Performance benchmarks specified (10k rows <10ms, 1M rows <500ms)

**Post-Design Assessment**: All constitutional principles satisfied. Design artifacts complete and ready for task generation. No violations or exceptions required.

**Gate Status**: [PASS] PASS - Proceed to Phase 2 (task generation via `/speckit.tasks` command)

## Project Structure

### Documentation (this feature)

```text
specs/007-delete-operation/
|-- plan.md              # This file (/speckit.plan command output)
|-- research.md          # Phase 0 output (/speckit.plan command)
|-- data-model.md        # Phase 1 output (/speckit.plan command)
|-- quickstart.md        # Phase 1 output (/speckit.plan command)
|-- contracts/           # Phase 1 output (/speckit.plan command)
`-- tasks.md             # Phase 2 output (/speckit.tasks command - NOT created by /speckit.plan)
```

### Source Code (repository root)

```text
crates/
|-- core/
|   |-- src/
|   |   |-- dsl/
|   |   |-- engine/
|   |   |-- model/
|   |   |-- resolver/
|   |   |-- trace/
|   |   `-- validation/
|   `-- tests/
|-- api-server/
|-- engine-worker/
|-- cli/
`-- test-resolver/

tests/
`-- scenarios/              # Planned scenario test suite path
```

**Structure Decision**: Single Cargo workspace; delete behavior changes are centered in `crates/core` (model + engine + validation) with scenario coverage under planned `tests/scenarios`.

## Complexity Tracking

No constitutional violations requiring justification.
