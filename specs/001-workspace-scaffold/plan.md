# Implementation Plan: Workspace Scaffold Baseline

**Branch**: `001-workspace-scaffold` | **Date**: 2026-02-22 | **Spec**: `/specs/001-workspace-scaffold/spec.md`
**Input**: Feature specification from `/specs/001-workspace-scaffold/spec.md`

## Summary

Establish the baseline Rust Cargo workspace scaffold with five crates (`core`, `api-server`, `engine-worker`, `cli`, `test-resolver`), core module boundaries, shared entity models, and IO trait contracts so that both `cargo build` and `cargo test` pass in a clean environment. Implementation remains compile-safe stub level only; behavior and infrastructure adapters are deferred to subsequent specs.

## Technical Context

**Language/Version**: Rust (stable toolchain policy; project MSRV to be pinned when CI toolchain baseline is introduced)  
**Primary Dependencies**: `polars` (lazy feature), `serde`, `serde_yaml`, `serde_json`, `uuid`, `chrono`, `anyhow`/`thiserror`  
**Storage**: N/A for S00 implementation (no concrete storage adapters); architecture-level targets are PostgreSQL + object/file stores for later specs  
**Testing**: `cargo test` with scaffold-focused unit tests for YAML/JSON deserialization and module compile integration  
**Target Platform**: Linux development/runtime baseline (Kubernetes deployment path deferred; not required in scaffold)  
**Project Type**: Rust Cargo workspace monorepo (multi-crate)  
**Performance Goals**: N/A for scaffold (no runtime throughput or latency targets in this phase)  
**Constraints**: Core crate remains IO-implementation-free; compile-safe placeholders only; no `axum`/`sqlx`/`kube`/`clap` integration in S00; must pass `cargo build` and `cargo test`  
**Scale/Scope**: One workspace root + five crates + core module skeleton + required entities/enums + four IO traits

## Constitution Check

*GATE: Must pass before Phase 0 research. Re-check after Phase 1 design.*

### Pre-Phase 0 Gate Assessment

- [x] **Principle I (TDD)**: Plan includes test-first tasks for serialization and scaffold compile coverage before implementation details
- [x] **Principle II (Quality Gates)**: Required gates are explicitly defined as `cargo build` and `cargo test` for this feature scope
- [x] **Principle III (Completion Bias)**: Ambiguities resolved in spec clarification and research decisions; no blocking open decisions
- [x] **Principle IV (Comprehensive Testing)**: Feature-level plan includes complete available suite execution (`cargo test`) and entity serialization checks

### Post-Phase 1 Design Re-check

- [x] **Principle I (TDD)**: Data model and contract outputs identify explicit testable contracts to drive implementation tests first
- [x] **Principle II (Quality Gates)**: Quickstart and contract artifacts preserve mandatory build/test validation path
- [x] **Principle III (Completion Bias)**: No unresolved `NEEDS CLARIFICATION` markers remain in plan/research/design outputs
- [x] **Principle IV (Comprehensive Testing)**: Planned verification includes full workspace tests plus schema/serde validations across required entities

**Notes**: No constitutional conflicts or exceptions identified for S00.

## Project Structure

### Documentation (this feature)

```text
specs/001-workspace-scaffold/
├── plan.md
├── research.md
├── data-model.md
├── quickstart.md
├── contracts/
│   └── workspace-scaffold.openapi.yaml
└── tasks.md
```

### Source Code (repository root)

```text
Cargo.toml
crates/
├── core/
│   ├── Cargo.toml
│   └── src/
│       ├── lib.rs
│       ├── model/
│       ├── dsl/
│       ├── engine/
│       ├── resolver/
│       ├── trace/
│       └── validation/
├── api-server/
│   ├── Cargo.toml
│   └── src/main.rs
├── engine-worker/
│   ├── Cargo.toml
│   └── src/main.rs
├── cli/
│   ├── Cargo.toml
│   └── src/main.rs
└── test-resolver/
    ├── Cargo.toml
    └── src/lib.rs

tests/
└── scenarios/
```

**Structure Decision**: Use the architecture-defined multi-crate Cargo workspace now (not a single crate) to lock boundaries early: pure domain `core` + executable wrappers + dedicated test resolver.

## Complexity Tracking

No constitutional violations requiring justification.
