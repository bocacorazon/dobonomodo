# Implementation Plan: Aggregate Operation

**Branch**: `001-aggregate-operation` | **Date**: 2026-02-22 | **Spec**: [/workspace/specs/001-aggregate-operation/spec.md](spec.md)
**Input**: Feature specification from `/specs/001-aggregate-operation/spec.md`

**Note**: This template is filled in by the `/speckit.plan` command. See `.specify/templates/plan-template.md` for the execution workflow.

## Summary

Implement the `aggregate` operation type that groups working dataset rows by specified columns, computes aggregate expressions (SUM, COUNT, AVG, MIN_AGG, MAX_AGG), and appends summary rows to the working dataset without removing or modifying existing rows. Each summary row will contain grouped key values, computed aggregates, system metadata, and null values for non-produced business columns.

## Technical Context

**Language/Version**: Rust 1.75+ (edition 2021)  
**Primary Dependencies**: Polars 0.46 (lazy API for DataFrames), serde/serde_json, uuid v7, anyhow/thiserror  
**Storage**: PostgreSQL (metadata store), in-memory working dataset (Polars LazyFrame)  
**Testing**: cargo test (unit + integration tests in crates/core/tests)  
**Target Platform**: Linux server (Kubernetes Jobs for Run orchestration)  
**Project Type**: Cargo workspace monorepo (core library + api-server + engine-worker + cli)  
**Performance Goals**: Process 100k+ rows/operation with <1s latency per aggregate operation  
**Constraints**: All operations in-memory except output; no I/O during aggregate; preserve all original rows  
**Scale/Scope**: Support pipelines with 10+ operations, datasets with 1M+ rows, aggregate over 100+ distinct groups

## Constitution Check

*GATE: Must pass before Phase 0 research. Re-check after Phase 1 design.*

**Constitutional Principles to Verify**:

- [x] **Principle I (TDD)**: All implementation tasks paired with tests written FIRST - plan includes test scenarios TS-05, acceptance scenarios in spec, and comprehensive test requirements. Test contracts defined in contracts/api-contract.md section 6.
- [x] **Principle II (Quality Gates)**: Build, lint, and test infrastructure configured - cargo build/test/clippy already in place. All validation rules defined with explicit error types.
- [x] **Principle III (Completion Bias)**: Ambiguities resolved; no open decision blocks - all technical decisions documented in research.md (Phase 0), data model complete in data-model.md, API contracts defined.
- [x] **Principle IV (Comprehensive Testing)**: Test suite execution plan covers all test types - unit tests for validation (6 cases), integration tests for execution (9 scenarios including TS-05), contract tests for serialization.

**Notes**: 
- All tests must be written before implementation (Red-Green-Refactor cycle)
- Full test suite (unit + integration + contract) must pass before any commits
- Any preexisting test failures discovered during work must be fixed immediately
- No violations detected - feature aligns with all constitutional principles

**Post-Phase 1 Re-Check**: ✅ PASSED
- Data model fully specified with validation state machine
- API contracts define all validation rules and error types
- Test contracts require comprehensive coverage (unit, integration, contract levels)
- Quickstart guide provides clear examples and troubleshooting
- All technical unknowns resolved in research.md
- Ready to proceed to Phase 2 (tasks.md generation via /speckit.tasks)

## Project Structure

### Documentation (this feature)

```text
specs/[###-feature]/
├── plan.md              # This file (/speckit.plan command output)
├── research.md          # Phase 0 output (/speckit.plan command)
├── data-model.md        # Phase 1 output (/speckit.plan command)
├── quickstart.md        # Phase 1 output (/speckit.plan command)
├── contracts/           # Phase 1 output (/speckit.plan command)
└── tasks.md             # Phase 2 output (/speckit.tasks command - NOT created by /speckit.plan)
```

### Source Code (repository root)

```text
crates/
├── core/                       # Core computation engine
│   ├── src/
│   │   ├── engine/             # Operation execution engine
│   │   │   ├── ops/            # Operation implementations
│   │   │   │   ├── mod.rs
│   │   │   │   ├── aggregate.rs  # NEW: Aggregate operation implementation
│   │   │   │   └── ...
│   │   │   ├── types.rs        # Working dataset, execution context
│   │   │   ├── io_traits.rs    # I/O abstraction
│   │   │   └── mod.rs
│   │   ├── model/              # Domain model
│   │   │   ├── operation.rs    # OperationKind::Aggregate definition (exists)
│   │   │   ├── expression.rs   # Expression wrapper (exists)
│   │   │   ├── dataset.rs      # Dataset schema definitions (exists)
│   │   │   └── mod.rs
│   │   ├── dsl/                # DSL parsing (S01 dependency)
│   │   ├── resolver/           # Data resolution
│   │   ├── trace/              # Execution tracing
│   │   └── lib.rs
│   └── tests/
│       ├── unit/               # Unit tests
│       │   └── aggregate_validation_test.rs  # NEW: Validation tests
│       ├── integration/        # Integration tests
│       │   └── aggregate_execution_test.rs   # NEW: End-to-end tests
│       └── contracts/          # Contract tests
│           └── aggregate_contract_test.rs    # NEW: Schema/API tests
├── api-server/                 # REST API (not modified)
├── engine-worker/              # Job executor (not modified)
├── cli/                        # CLI tool (not modified)
└── test-resolver/              # Test utilities (may extend for fixtures)
```

**Structure Decision**: Single Cargo workspace with feature implementation in `crates/core`. The aggregate operation is a core engine capability, so all code goes in `core/src/engine/ops/aggregate.rs` with supporting test files. No changes needed to api-server, engine-worker, or cli for this feature - they consume the core library.

## Complexity Tracking

> **Fill ONLY if Constitution Check has violations that must be justified**

| Violation | Why Needed | Simpler Alternative Rejected Because |
|-----------|------------|-------------------------------------|
| [e.g., 4th project] | [current need] | [why 3 projects insufficient] |
| [e.g., Repository pattern] | [specific problem] | [why direct DB access insufficient] |
