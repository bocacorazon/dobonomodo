# Implementation Plan: Test Harness

**Branch**: `003-test-harness` | **Date**: 2026-02-22 | **Spec**: `/workspace/specs/003-test-harness/spec.md`
**Input**: Feature specification from `/workspace/specs/003-test-harness/spec.md`

**Note**: This template is filled in by the `/speckit.plan` command. See `.specify/templates/plan-template.md` for the execution workflow.

## Summary

Build the data-driven test harness that loads YAML test scenario files, provisions input data with injected system metadata, executes a Project pipeline via `core::engine`, compares actual output to expected output, and produces structured diff reports. Implements `InMemoryDataLoader` in `test-resolver` crate, YAML scenario parser with serde, metadata injection engine, output comparison with exact/subset match modes, trace validation, and CLI integration for single-scenario and suite execution.

## Technical Context

**Language/Version**: Rust (stable toolchain policy; project MSRV deferred to CI baseline)  
**Primary Dependencies**: `polars` (lazy feature), `serde`, `serde_yaml`, `uuid` (v7 feature), `chrono`, `anyhow`/`thiserror`  
**Storage**: In-memory test data only (no persistent storage for test harness); actual DataBlocks from inline YAML rows or file references (CSV/Parquet)  
**Testing**: `cargo test` with scenario-driven integration tests; self-testing via passthrough scenarios  
**Target Platform**: Linux development baseline (CLI binary for local and CI execution)  
**Project Type**: Rust Cargo workspace monorepo (multi-crate) — primary work in `test-resolver` and `cli` crates  
**Performance Goals**: Test execution throughput sufficient for CI/CD (target: <1s per simple scenario, <10s for complex scenarios with large datasets)  
**Constraints**: No production IO dependencies in test harness; pipeline execution via `core::engine` is stubbed until S10 (passthrough or mock); metadata injection must handle all temporal_mode variants; comparison engine must collect ALL mismatches without early exit  
**Scale/Scope**: Support test scenarios with datasets up to 10k rows (inline or file-based); suite discovery across hundreds of scenario files; match mode flexibility (exact/subset); trace validation (optional)

## Constitution Check

*GATE: Must pass before Phase 0 research. Re-check after Phase 1 design.*

### Pre-Phase 0 Gate Assessment

- [x] **Principle I (TDD)**: Plan includes test-first approach — test scenarios will test the test harness itself (self-hosting via passthrough scenarios); all implementation tasks will have corresponding tests
- [x] **Principle II (Quality Gates)**: Required gates defined as `cargo build` and `cargo test`; test harness validates scenarios via automated comparison
- [x] **Principle III (Completion Bias)**: All technical decisions resolved; metadata injection format, comparison algorithms, and data structures are specified in capability doc; no blocking ambiguities remain
- [x] **Principle IV (Comprehensive Testing)**: Test plan includes unit tests for comparison logic, integration tests for scenario execution, and self-testing via passthrough scenarios from spec

### Post-Phase 1 Design Re-check

- [x] **Principle I (TDD)**: Data model and contract outputs identify explicit testable contracts to drive implementation tests first (TestScenario validation, metadata injection, comparison engine all have clear test cases)
- [x] **Principle II (Quality Gates)**: Quickstart and contract artifacts preserve mandatory build/test validation path; CLI contract specifies exit codes and verification behavior
- [x] **Principle III (Completion Bias)**: No unresolved `NEEDS CLARIFICATION` markers remain in plan/research/design outputs; all technical decisions documented in research.md
- [x] **Principle IV (Comprehensive Testing)**: Planned verification includes full workspace tests plus harness self-tests (passthrough scenario validates entire harness end-to-end)

**Notes**: No constitutional conflicts or exceptions identified for S02. The test harness is inherently TDD-compatible as it IS the testing infrastructure.

## Project Structure

### Documentation (this feature)

```text
specs/003-test-harness/
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
├── core/
│   └── src/
│       ├── model/            # TestScenario, TestConfig, TestResult, DataMismatch, TraceMismatch entities
│       └── engine/           # Pipeline executor (stubbed for S02; implemented in S10)
│
├── test-resolver/
│   ├── Cargo.toml
│   └── src/
│       ├── lib.rs
│       ├── loader.rs         # InMemoryDataLoader implementing DataLoader trait
│       ├── metadata.rs       # InMemoryMetadataStore for test isolation
│       ├── trace.rs          # InMemoryTraceWriter for test isolation
│       └── injection.rs      # System metadata injection engine
│
└── cli/
    ├── Cargo.toml
    └── src/
        ├── main.rs
        ├── commands/
        │   └── test.rs       # `dobo test` command implementation
        └── harness/
            ├── parser.rs     # YAML scenario parser (serde)
            ├── executor.rs   # Test execution orchestrator
            ├── comparator.rs # Output comparison engine (exact/subset modes)
            └── reporter.rs   # TestResult assembly and reporting

tests/
└── scenarios/
    └── harness-self-test.yaml  # Passthrough scenario from spec for harness validation
```

**Structure Decision**: Use existing Cargo workspace structure; primary implementation in `test-resolver` (in-memory IO adapters + metadata injection) and `cli` (harness orchestration, parsing, comparison, reporting). Core entity models (TestScenario, TestResult, etc.) live in `core/model` for reuse across crates.

## Complexity Tracking

No constitutional violations requiring justification.
