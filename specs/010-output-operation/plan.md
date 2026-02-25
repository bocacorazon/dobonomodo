# Implementation Plan: Output Operation

**Branch**: `010-output-operation` | **Date**: 2026-02-23 | **Spec**: [/workspace/specs/010-output-operation/spec.md](spec.md)
**Input**: Feature specification from `/workspace/specs/010-output-operation/spec.md`

**Note**: This template is filled in by the `/speckit.plan` command. See `.specify/templates/plan-template.md` for the execution workflow.

## Summary

Implement the `output` operation type to apply selector filtering, project columns, handle `include_deleted` flag, write to destination via `OutputWriter` trait, and optionally register the output as a new Dataset in the MetadataStore. Output operations can appear anywhere in the pipeline (mid-pipeline checkpointing) without modifying the working dataset.

## Technical Context

**Language/Version**: Rust 1.75+ (edition 2021)  
**Primary Dependencies**: polars (dataframe operations), serde (serialization), anyhow/thiserror (error handling)  
**Storage**: In-memory dataframes (Polars LazyFrame/DataFrame); output via OutputWriter trait  
**Testing**: cargo test (unit, integration, contract tests)  
**Target Platform**: Linux server (workspace crates: core, api-server, engine-worker, cli)
**Project Type**: Single workspace with multiple crates (monorepo)  
**Performance Goals**: Memory-efficient processing using Polars LazyFrame late materialization to handle million-row datasets without eager full-frame duplication  
**Constraints**: Must preserve working dataset immutability; deleted rows excluded by default  
**Scale/Scope**: 5 operation types, ~15 business rules from operation.md entity, test scenario TS-07

## Constitution Check

*GATE: Must pass before Phase 0 research. Re-check after Phase 1 design.*

**Initial Check (Pre-Phase 0)**: ✅ PASSED

**Post-Design Check (After Phase 1)**: ✅ PASSED

**Constitutional Principles to Verify**:

- [X] **Principle I (TDD)**: All implementation tasks paired with tests written FIRST
  - Test scenario TS-07 defined in sample-datasets.md
  - Contract tests will be written before implementation (ts07_column_projection.rs)
  - Unit tests specified in contracts/api.md (7 test cases defined)
  - Integration tests specified (5 test cases defined)
  - **Post-Design**: All test types mapped to specific test files in project structure
  
- [X] **Principle II (Quality Gates)**: Build, lint, and test infrastructure configured
  - Cargo workspace already configured with test framework
  - `cargo test`, `cargo clippy`, `cargo fmt` available
  - CI/CD pipeline exists (GitHub Actions)
  - **Post-Design**: No new infrastructure needed; uses existing cargo test framework
  
- [X] **Principle III (Completion Bias)**: Ambiguities resolved; no open decision blocks
  - Performance optimization approach RESOLVED in research.md (LazyFrame with late materialization)
  - All other technical decisions made autonomously (error handling: thiserror, testing: 3-tier)
  - **Post-Design**: All design decisions documented; no blocking unknowns remain
  
- [X] **Principle IV (Comprehensive Testing)**: Test suite execution plan covers all test types
  - Unit tests: 7 test cases (selector, projection, deleted flag, schema, validation, errors)
  - Integration tests: 5 test cases (e2e, registration, mid-pipeline, write failure, missing store)
  - Contract tests: TS-07 (column projection from sample-datasets.md)
  - **Post-Design**: Full test coverage matrix defined in contracts/api.md section "Testing Requirements"

**Notes**: No constitutional violations. All principles satisfied. Performance clarification resolved via research (LazyFrame strategy). Design phase complete with full test coverage plan.

## Project Structure

### Documentation (this feature)

```text
specs/010-output-operation/
├── plan.md              # This file (/speckit.plan command output)
├── research.md          # Phase 0 output (/speckit.plan command)
├── data-model.md        # Phase 1 output (/speckit.plan command)
├── quickstart.md        # Phase 1 output (/speckit.plan command)
├── contracts/
│   └── api.md           # Phase 1 output (/speckit.plan command)
└── tasks.md             # Phase 2 output (/speckit.tasks command - NOT created by /speckit.plan)
```

### Source Code (repository root)

```text
crates/
├── core/
│   ├── src/
│   │   ├── engine/
│   │   │   ├── ops/           # NEW: operation implementations
│   │   │   │   ├── mod.rs
│   │   │   │   └── output.rs  # NEW: output operation logic
│   │   │   ├── io_traits.rs   # EXISTING: OutputWriter trait
│   │   │   ├── mod.rs
│   │   │   └── types.rs
│   │   ├── model/
│   │   │   ├── operation.rs   # EXISTING: OutputArguments struct
│   │   │   ├── dataset.rs     # EXISTING: Dataset entity
│   │   │   └── metadata_store.rs  # EXISTING: MetadataStore trait
│   │   └── lib.rs
│   └── tests/
│       ├── unit/
│       │   └── output_op_test.rs  # NEW: unit tests
│       ├── integration/
│       │   └── output_integration_test.rs  # NEW: integration tests
│       └── contract/
│           └── ts07_column_projection.rs  # NEW: TS-07 contract test
├── api-server/         # OUT OF SCOPE
├── engine-worker/      # OUT OF SCOPE
├── cli/                # OUT OF SCOPE
└── test-resolver/      # MAY USE for test fixtures
```

**Structure Decision**: All output operation logic resides in `crates/core/src/engine/ops/output.rs`. This follows the existing pattern where `engine/` contains execution logic and `model/` contains entity definitions. The OutputWriter trait already exists in `engine/io_traits.rs` and will be used for writing. Tests are organized by type (unit, integration, contract) as per existing conventions.

## Complexity Tracking

No constitutional violations requiring justification.
