# Implementation Plan: Runtime Join Resolution

**Branch**: `006-runtime-join` | **Date**: 2026-02-22 | **Spec**: [spec.md](./spec.md)
**Input**: Feature specification from `/specs/006-runtime-join/spec.md`

**Note**: This template is filled in by the `/speckit.plan` command. See `.specify/templates/plan-template.md` for the execution workflow.

## Summary

Implement RuntimeJoin resolution for update operations in the computation engine. The feature enables dynamic enrichment of working datasets by resolving external datasets via the Resolver, loading them through DataLoader, applying period-aware filtering based on temporal_mode (exact match for period tables, asOf query for bitemporal tables), and joining them to the working LazyFrame under operation-scoped aliases. Assignment expressions can then reference joined columns using `alias.column_name` syntax. The primary use case is FX conversion in financial pipelines where transaction amounts are multiplied by exchange rates from a bitemporal lookup table.

## Technical Context

**Language/Version**: Rust 2021 edition (workspace configured)  
**Primary Dependencies**: Polars 0.46 (lazy execution), serde/serde_json/serde_yaml (serialization), uuid v7 (identifiers), chrono (temporal logic), anyhow/thiserror (error handling)  
**Storage**: N/A (engine operates on in-memory LazyFrames; persistence handled by separate IO layer)  
**Testing**: cargo test (unit tests in `crates/core/tests/`, contract tests in `crates/core/tests/contracts/`)  
**Target Platform**: Linux (primary), cross-platform Rust  
**Project Type**: Single Rust workspace with multiple crates (core computation engine, API server, CLI, engine worker, test resolver)  
**Performance Goals**: Lazy execution via Polars (defer materialization until output operation), support datasets with 100k+ rows in bitemporal joins  
**Constraints**: All joins are left joins (inner/right/outer deferred), operation-scoped aliases only, no self-join support (deferred per OQ-002)  
**Scale/Scope**: Financial domain (GL transactions, budgets, exchange rates), test scenarios with 10-100 rows, production datasets anticipated 10k-1M rows per period

## Constitution Check

*GATE: Must pass before Phase 0 research. Re-check after Phase 1 design.*

**Initial Check (Pre-Research)**: PASS

**Constitutional Principles to Verify**:

- [x] **Principle I (TDD)**: All implementation tasks paired with tests written FIRST - Contract tests will define RuntimeJoin structure, unit tests will validate join resolution/filtering/execution, integration test TS-03 will verify FX conversion end-to-end
- [x] **Principle II (Quality Gates)**: Build, lint, and test infrastructure configured - Cargo workspace already configured with rustfmt/clippy, `cargo test` runs all test suites
- [x] **Principle III (Completion Bias)**: Ambiguities resolved; no open decision blocks - All technical unknowns listed in research phase (Polars join API, period filter integration, expression compilation), resolver precedence and temporal_mode behavior fully specified in docs/entities
- [x] **Principle IV (Comprehensive Testing)**: Test suite execution plan covers all test types - Contract tests (RuntimeJoin schema validation), unit tests (resolver precedence, version resolution, period filtering per temporal_mode), integration test (TS-03 FX conversion scenario with InMemoryDataLoader)

**Post-Design Re-evaluation**: PASS

Design artifacts confirm:
- **research.md**: All 6 research tasks resolved with clear decisions, rationale, and alternatives considered. No NEEDS CLARIFICATION markers remain.
- **data-model.md**: RuntimeJoin entity fully specified with 4 attributes, validation rules, relationships, and invariants. ResolverSnapshot extended with join_datasets map for reproducibility.
- **contracts/runtime_join_schema.yaml**: OpenAPI 3.1 contract covering RuntimeJoin validation, resolution, and execution preview endpoints plus schema constraints.
- **quickstart.md**: Step-by-step guide with code examples, test data setup, verification steps, and troubleshooting - demonstrates TDD approach.

**Notes**: No principle violations. This feature extends existing update operation with well-defined join semantics. Deferred items (self-join, non-left joins) are explicitly documented in operation.md as future scope, not blockers. All design decisions support testability and maintainability per constitutional principles.

## Project Structure

### Documentation (this feature)

```text
specs/006-runtime-join/
|-- plan.md              # This file (/speckit.plan command output)
|-- research.md          # Phase 0 output (/speckit.plan command)
|-- data-model.md        # Phase 1 output (/speckit.plan command)
|-- quickstart.md        # Phase 1 output (/speckit.plan command)
|-- contracts/           # Phase 1 output (/speckit.plan command)
|   `-- runtime_join_schema.yaml  # OpenAPI contract for RuntimeJoin workflows
`-- tasks.md             # Phase 2 output (/speckit.tasks command - NOT created by /speckit.plan)
```

### Source Code (repository root)

```text
crates/
|-- core/
|   |-- src/
|   |   |-- engine/
|   |   |   |-- mod.rs           # Module exports
|   |   |   |-- types.rs         # Engine types
|   |   |   |-- io_traits.rs     # DataLoader trait
|   |   |   `-- join.rs          # NEW: RuntimeJoin resolution and execution
|   |   |-- model/
|   |   |   |-- dataset.rs       # Dataset, TableRef, TemporalMode
|   |   |   |-- metadata_store.rs  # Dataset lookup by id/version
|   |   |   `-- calendar.rs      # Period definitions
|   |   |-- resolver/
|   |   |   `-- mod.rs           # Resolver trait, location resolution
|   |   `-- lib.rs
|   `-- tests/
|       |-- contracts/
|       |   `-- runtime_join_contract.rs  # NEW: Schema validation
|       |-- unit/
|       |   `-- engine_join_tests.rs      # NEW: Join resolution, filtering
|       `-- integration/
|           `-- ts03_fx_conversion.rs     # NEW: End-to-end FX scenario
|-- test-resolver/
|   `-- src/
|       `-- lib.rs               # InMemoryDataLoader implementation
`-- Cargo.toml                   # Workspace configuration
```

**Structure Decision**: This is a Rust workspace with a single core library crate (`crates/core`) and supporting crates (api-server, cli, engine-worker, test-resolver). The RuntimeJoin feature is implemented in the `core::engine::join` module, integrating with existing `model` (Dataset, Period), `resolver` (location resolution), and `engine::io_traits` (DataLoader). Tests follow the existing pattern: contracts/ for schema validation, unit/ for isolated logic, integration/ for full scenarios with InMemoryDataLoader.

## Complexity Tracking

No constitutional violations requiring justification.
