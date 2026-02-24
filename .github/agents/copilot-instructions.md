# dobonomodo Development Guidelines

Auto-generated from all feature plans. Last updated: 2026-02-22

## Active Technologies
- Rust 2021 edition + Polars 0.46 (DataFrame processing), Serde (serialization), UUID v7, Chrono (009-append-operation)
- N/A (data loading via resolver pattern, processing in-memory with Polars) (009-append-operation)

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
- 009-append-operation: Added Rust 2021 edition + Polars 0.46 (DataFrame processing), Serde (serialization), UUID v7, Chrono

- 001-workspace-scaffold: Added Rust (stable toolchain policy; project MSRV to be pinned when CI toolchain baseline is introduced) + `polars` (lazy feature), `serde`, `serde_yaml`, `serde_json`, `uuid`, `chrono`, `anyhow`/`thiserror`

<!-- MANUAL ADDITIONS START -->
<!-- MANUAL ADDITIONS END -->
