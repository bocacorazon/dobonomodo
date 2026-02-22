# dobonomodo Development Guidelines

Auto-generated from all feature plans. Last updated: 2026-02-22

## Active Technologies
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
- 003-test-harness: Added Rust (stable toolchain policy; project MSRV deferred to CI baseline) + `polars` (lazy feature), `serde`, `serde_yaml`, `uuid` (v7 feature), `chrono`, `anyhow`/`thiserror`
- 003-test-harness: Added Rust (stable toolchain policy; project MSRV deferred to CI baseline) + `polars` (lazy feature), `serde`, `serde_yaml`, `uuid` (v7 feature), `chrono`, `anyhow`/`thiserror`

- 001-workspace-scaffold: Added Rust (stable toolchain policy; project MSRV to be pinned when CI toolchain baseline is introduced) + `polars` (lazy feature), `serde`, `serde_yaml`, `serde_json`, `uuid`, `chrono`, `anyhow`/`thiserror`

<!-- MANUAL ADDITIONS START -->
<!-- MANUAL ADDITIONS END -->
