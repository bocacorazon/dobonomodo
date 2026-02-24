# dobonomodo Development Guidelines

Auto-generated from all feature plans. Last updated: 2026-02-22

## Active Technologies
- Rust 2021 edition + Polars 0.46 (DataFrame processing), Serde (serialization), UUID v7, Chrono (009-append-operation)
- N/A (data loading via resolver pattern, processing in-memory with Polars) (009-append-operation)
- Rust 1.75+ (Rust 2021 edition) + Polars 0.46 (lazy API), serde, serde_json, serde_yaml, anyhow, thiserror (005-update-operation)
- N/A (operates on in-memory LazyFrame) (005-update-operation)
- Rust 2021 edition + Polars (lazy API for data operations), serde (serialization), uuid (row tracking) (007-delete-operation)
- PostgreSQL (metadata store), Polars DataFrames (in-memory working data) (007-delete-operation)
- Rust 2021 edition (workspace baseline: 0.1.0) (002-dsl-parser)
- N/A (this is a pure compilation/parsing module) (002-dsl-parser)
- Rust (stable toolchain policy; project MSRV deferred to CI baseline) + `polars` (lazy feature), `serde`, `serde_yaml`, `uuid` (v7 feature), `chrono`, `anyhow`/`thiserror` (003-test-harness)
- In-memory test data only (no persistent storage for test harness); actual DataBlocks from inline YAML rows or file references (CSV/Parquet) (003-test-harness)

- Rust (stable toolchain policy; project MSRV to be pinned when CI toolchain baseline is introduced) + `polars` (lazy feature), `serde`, `serde_yaml`, `serde_json`, `uuid`, `chrono`, `anyhow`/`thiserror` (001-workspace-scaffold)

## Project Structure

```text
crates/
  core/
  api-server/
  engine-worker/
  cli/
  test-resolver/
```

## Commands

- `cargo test`
- `cargo clippy`

## Code Style

Rust (stable toolchain policy; project MSRV to be pinned when CI toolchain baseline is introduced): Follow standard conventions

## Recent Changes
- 009-append-operation: Added Rust 2021 edition + Polars 0.46 (DataFrame processing), Serde (serialization), UUID v7, Chrono
- 007-delete-operation: Added Rust 2021 edition + Polars (lazy API for data operations), serde (serialization), uuid (row tracking)
- 005-update-operation: Added Rust 1.75+ (Rust 2021 edition) + Polars 0.46 (lazy API), serde, serde_json, serde_yaml, anyhow, thiserror
- 002-dsl-parser: Added Rust 2021 edition (workspace baseline: 0.1.0)
- 003-test-harness: Added Rust (stable toolchain policy; project MSRV deferred to CI baseline) + `polars` (lazy feature), `serde`, `serde_yaml`, `uuid` (v7 feature), `chrono`, `anyhow`/`thiserror`

- 001-workspace-scaffold: Added Rust (stable toolchain policy; project MSRV to be pinned when CI toolchain baseline is introduced) + `polars` (lazy feature), `serde`, `serde_yaml`, `serde_json`, `uuid`, `chrono`, `anyhow`/`thiserror`

<!-- MANUAL ADDITIONS START -->
- DSL module context (002-dsl-parser):
  - Path: `crates/core/src/dsl/`
  - Entry points: `parse_expression`, `interpolate_selectors`, `validate_expression`, `compile_expression`, `compile_with_interpolation`
  - Core files: `ast.rs`, `parser.rs`, `validation.rs`, `interpolation.rs`, `compiler.rs`, `context.rs`, `error.rs`
  - Contracts/spec artifacts: `specs/002-dsl-parser/contracts/*.md`
<!-- MANUAL ADDITIONS END -->
