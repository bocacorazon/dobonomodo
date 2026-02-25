# Implementation Plan: Append Operation

**Branch**: `009-append-operation` | **Date**: 2026-02-22 | **Spec**: [spec.md](./spec.md)
**Input**: Feature specification from `/specs/009-append-operation/spec.md`

**Note**: This template is filled in by the `/speckit.plan` command. See `.specify/templates/plan-template.md` for the execution workflow.

## Summary

Implement the `append` operation type that loads rows from a source Dataset, optionally filters with `source_selector`, optionally aggregates before appending, aligns columns with the working dataset, and appends the rows. This enables combining data from different datasets for comparative analysis (e.g., budget vs actual comparisons).

## Technical Context

**Language/Version**: Rust 2021 edition  
**Primary Dependencies**: Polars 0.46 (DataFrame processing), Serde (serialization), UUID v7, Chrono  
**Storage**: N/A (data loading via resolver pattern, processing in-memory with Polars)  
**Testing**: Cargo test (unit tests in crates/core/tests/, contract tests in crates/core/tests/contracts/)  
**Target Platform**: Linux server (workspace crate with core, api-server, engine-worker, cli, test-resolver)
**Project Type**: Single workspace with multiple crates (core library + supporting binaries)  
**Performance Goals**: Simple append <10ms for 100k rows, filtered append <20ms for 100k rows, aggregated append <50ms for 100k rows  
**Constraints**: Must align with existing temporal_mode filtering (period/bitemporal), maintain column type safety  
**Scale/Scope**: Enterprise data processing (datasets with 10k-1M rows, 50-500 columns)

## Constitution Check

*GATE: Must pass before Phase 0 research. Re-check after Phase 1 design.*

**Constitutional Principles to Verify**:

- [x] **Principle I (TDD)**: All implementation tasks paired with tests written FIRST - contract tests exist in crates/core/tests/contracts/, following established TDD pattern
- [x] **Principle II (Quality Gates)**: Build, lint, and test infrastructure configured - cargo build, cargo test, cargo clippy available in workspace
- [x] **Principle III (Completion Bias)**: Ambiguities resolved; no open decision blocks - ✓ Research completed: performance target <50ms, parse aggregate expressions to Polars Expr, fail early on missing datasets at planning phase
- [x] **Principle IV (Comprehensive Testing)**: Test suite execution plan covers all test types - will include unit tests (operation parsing, validation), contract tests (append operation behavior), integration tests (temporal filtering, aggregation)

**Post-Phase 1 Re-Evaluation**:

✅ **All principles verified after design phase**:
- Design artifacts complete: data-model.md defines all entities with validation rules
- Contracts defined: JSON/YAML schemas specify operation structure and validation
- Quickstart guide provides implementation patterns and error handling
- Agent context updated: Technology stack documented for AI assistance
- No constitutional violations identified
- Ready to proceed to Phase 2 (tasks generation)

**Notes**: No constitutional violations. Design follows existing Rust workspace patterns, leverages Polars for performance (<50ms target achievable), and maintains TDD discipline with clear test strategy.

## Project Structure

### Documentation (this feature)

```text
specs/009-append-operation/
├── plan.md              # This file (/speckit.plan command output)
├── research.md          # Phase 0 output (/speckit.plan command)
├── data-model.md        # Phase 1 output (/speckit.plan command)
├── quickstart.md        # Phase 1 output (/speckit.plan command)
├── contracts/           # Phase 1 output (/speckit.plan command)
└── tasks.md             # Phase 2 output (/speckit.tasks command - NOT created by /speckit.plan)
```

### Source Code (repository root)

```text
# Rust workspace structure
crates/
├── core/                    # Core library (append operation implementation here)
│   ├── src/
│   │   ├── model/           # Data models (operation.rs, dataset.rs, expression.rs)
│   │   ├── engine/          # Execution engine (operation execution, IO traits)
│   │   ├── resolver/        # Data resolver pattern (MetadataStore, DataLoader)
│   │   ├── dsl/             # Domain-specific language parsing
│   │   └── trace/           # Execution tracing
│   └── tests/
│       ├── unit/            # Unit tests (operation parsing, validation logic)
│       ├── integration/     # Integration tests (end-to-end append scenarios)
│       └── contracts/       # Contract tests (operation behavior contracts)
│
├── api-server/              # API server binary
├── engine-worker/           # Background worker binary  
├── cli/                     # CLI binary
└── test-resolver/           # Test resolver implementation

specs/009-append-operation/  # This feature's documentation
├── plan.md                  # This file
├── research.md              # Phase 0 output
├── data-model.md            # Phase 1 output
├── quickstart.md            # Phase 1 output
└── contracts/               # Phase 1 API contracts
```

**Structure Decision**: Single Rust workspace with core library pattern. Append operation implementation will be in `crates/core/src/model/operation.rs` (data structures) and `crates/core/src/engine/` (execution logic). Tests follow existing pattern: contract tests in `tests/contracts/`, integration tests demonstrate complete append scenarios, unit tests cover component-level validation.

## Complexity Tracking

> **Fill ONLY if Constitution Check has violations that must be justified**

| Violation | Why Needed | Simpler Alternative Rejected Because |
|-----------|------------|-------------------------------------|
