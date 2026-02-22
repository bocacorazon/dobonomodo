# Quickstart: Workspace Scaffold Baseline (S00)

## Prerequisites

- Rust toolchain installed (`cargo` available)
- Repository checked out on branch `001-workspace-scaffold`

## 1) Confirm workspace skeleton exists

Expected top-level crate layout:

- `crates/core`
- `crates/api-server`
- `crates/engine-worker`
- `crates/cli`
- `crates/test-resolver`

Expected core submodules:

- `model`
- `dsl`
- `engine`
- `resolver`
- `trace`
- `validation`

## 2) Verify compile gate

Run:

```bash
cargo build
```

Expected result: command completes with zero errors.

## 3) Verify test gate

Run:

```bash
cargo test
```

Expected result: command completes successfully.

## 4) Verify serialization contract coverage

- Ensure tests include YAML and JSON deserialization checks for required entities and enums.
- Ensure representative samples cover Dataset, Project, Run, Resolver, Calendar, Period, DataSource, and OperationInstance/Expression shapes.

## 5) Scope guard checks

- No concrete API server, worker orchestration, resolver adapters, or storage integrations implemented in S00.
- Non-`output` operation behavior remains unimplemented placeholders.
- IO trait implementations are deferred; only interfaces exist in `core`.

## Completion Criteria

S00 is complete when all of the following are true:

1. Required workspace/crate/module structure exists.
2. Shared entity and enum contracts compile and are re-exported from `core`.
3. IO traits are defined in `core` with architecture-aligned signatures.
4. `cargo build` and `cargo test` both pass.

## Verification Record

- Date: 2026-02-22
- `cargo build`: PASS
- `cargo test`: PASS
