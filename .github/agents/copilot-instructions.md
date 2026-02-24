# dobonomodo Development Guidelines

Auto-generated from all feature plans. Last updated: 2026-02-22

## Active Technologies
- Rust 1.75+ (edition 2021) + Polars 0.46 (lazy API for DataFrames), serde/serde_json, uuid v7, anyhow/thiserror (001-aggregate-operation)
- PostgreSQL (metadata store), in-memory working dataset (Polars LazyFrame) (001-aggregate-operation)

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
- 001-aggregate-operation: Added Rust 1.75+ (edition 2021) + Polars 0.46 (lazy API for DataFrames), serde/serde_json, uuid v7, anyhow/thiserror
- 001-aggregate-operation: Added Polars 0.46 lazy API for group-by aggregations (SUM, COUNT, AVG, MIN_AGG, MAX_AGG), UUID v7 for row IDs, PostgreSQL metadata store


<!-- MANUAL ADDITIONS START -->
<!-- MANUAL ADDITIONS END -->
