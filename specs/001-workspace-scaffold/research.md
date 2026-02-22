# Phase 0 Research: Workspace Scaffold Baseline

## Decision 1: Rust toolchain strategy

- **Decision**: Use Rust stable channel with an explicit MSRV policy to be pinned when CI toolchain is introduced.
- **Rationale**: Feature scope requires Rust but does not define a pinned toolchain yet; this keeps progress unblocked while preserving reproducibility planning.
- **Alternatives considered**:
  - Pin an arbitrary Rust version now (rejected: unsupported by current repo constraints).
  - Float indefinitely on latest stable without MSRV guidance (rejected: weak reproducibility).

## Decision 2: Baseline dependency set for S00

- **Decision**: Include only scaffold-critical dependencies: `polars` (lazy), `serde`, `serde_yaml`, `serde_json`, `uuid`, `chrono`, and one error abstraction (`anyhow` or `thiserror`).
- **Rationale**: Matches S00 scope and architecture while avoiding premature framework coupling.
- **Alternatives considered**:
  - Add `axum`, `sqlx`, `kube`, `clap` in S00 (rejected: explicitly out of scope).
  - Delay `polars` until engine features (rejected: IO trait and model contract alignment already references Polars types).

## Decision 3: Workspace structure boundary

- **Decision**: Implement the architecture-defined five-crate Cargo workspace immediately (`core`, `api-server`, `engine-worker`, `cli`, `test-resolver`).
- **Rationale**: Locks boundaries early and enables parallel downstream specs.
- **Alternatives considered**:
  - Start with a single crate then split later (rejected: boundary churn and migration cost).
  - Add extra support crates now (rejected: unnecessary complexity for scaffold phase).

## Decision 4: Core module surface

- **Decision**: Create `core` modules `model/`, `dsl/`, `engine/`, `resolver/`, `trace/`, and `validation/` with compile-safe stubs only.
- **Rationale**: Required by S00 and supports future feature decomposition without behavior implementation.
- **Alternatives considered**:
  - Put all code under one `mod` initially (rejected: weak module contracts).
  - Implement partial behavior now (rejected: violates out-of-scope constraints).

## Decision 5: IO trait contract shape

- **Decision**: Define `DataLoader`, `OutputWriter`, `MetadataStore`, and `TraceWriter` in `core` with signatures aligned to architecture (`LazyFrame`/`DataFrame`/typed entity retrieval/status update/trace event writing).
- **Rationale**: Preserves clean separation of pure domain logic from adapters and infrastructure.
- **Alternatives considered**:
  - Async trait APIs now (rejected: unnecessary complexity for scaffold-only phase).
  - Opaque generic result payloads (rejected: weak cross-crate contract clarity).

## Decision 6: Validation gate for scaffold completion

- **Decision**: Treat both `cargo build` and `cargo test` as required completion gates.
- **Rationale**: Explicitly clarified in spec session and aligns with constitution quality/testing principles.
- **Alternatives considered**:
  - Build-only gate (rejected: insufficient verification).
  - Add strict lint gate in S00 completion criteria (deferred: useful but not part of clarified acceptance gate).

## Decision 7: Entity serialization verification approach

- **Decision**: Add focused tests that verify required entities/enums deserialize from representative YAML and JSON samples.
- **Rationale**: Directly satisfies FR-008/FR-009 and SC-004/SC-005 while keeping tests small and deterministic.
- **Alternatives considered**:
  - No serialization tests (rejected: does not satisfy requirements).
  - Full scenario harness tests in S00 (rejected: belongs to later test-harness specs).

## Decision 8: Scope guardrails for S00

- **Decision**: Keep all runtime/business behavior unimplemented except compile-safe placeholders; do not add runtime frameworks/adapters.
- **Rationale**: Enforces S00 as scaffolding only and avoids leaking implementation from future specs.
- **Alternatives considered**:
  - Implement minimal API/worker behavior now (rejected: out of scope).
  - Add external storage connectors now (rejected: deferred to adapter specs).

## Resolved Clarifications

All technical-context unknowns are resolved for S00 planning. No `NEEDS CLARIFICATION` markers remain.
