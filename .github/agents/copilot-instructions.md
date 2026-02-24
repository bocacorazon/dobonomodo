# dobonomodo Development Guidelines

Auto-generated from all feature plans. Last updated: 2026-02-22

## Active Technologies
- Rust 2021 edition (workspace configured) + Polars 0.46 (lazy execution), serde/serde_json/serde_yaml (serialization), uuid v7 (identifiers), chrono (temporal logic), anyhow/thiserror (error handling) (006-runtime-join)
- N/A (engine operates on in-memory LazyFrames; persistence handled by separate IO layer) (006-runtime-join)

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
- 006-runtime-join: Added Rust 2021 edition (workspace configured) + Polars 0.46 (lazy execution), serde/serde_json/serde_yaml (serialization), uuid v7 (identifiers), chrono (temporal logic), anyhow/thiserror (error handling)
- 006-runtime-join: Added Rust 2021 edition (workspace configured) + Polars 0.46 (lazy execution), serde/serde_json/serde_yaml (serialization), uuid v7 (identifiers), chrono (temporal logic), anyhow/thiserror (error handling)

- 001-workspace-scaffold: Added Rust (stable toolchain policy; project MSRV to be pinned when CI toolchain baseline is introduced) + `polars` (lazy feature), `serde`, `serde_yaml`, `serde_json`, `uuid`, `chrono`, `anyhow`/`thiserror`

<!-- MANUAL ADDITIONS START -->
<!-- MANUAL ADDITIONS END -->
