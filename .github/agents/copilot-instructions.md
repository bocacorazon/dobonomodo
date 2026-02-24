# dobonomodo Development Guidelines

Auto-generated from all feature plans. Last updated: 2026-02-22

## Active Technologies
- Rust 1.75 + `polars` (lazy), `chrono`, `uuid`, `serde` (004-period-filter)
- N/A (Processing layer) (004-period-filter)

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
- 004-period-filter: Added Rust 1.75 + `polars` (lazy), `chrono`, `uuid`, `serde`
- 004-period-filter: Added [if applicable, e.g., PostgreSQL, CoreData, files or N/A]

- 001-workspace-scaffold: Added Rust (stable toolchain policy; project MSRV to be pinned when CI toolchain baseline is introduced) + `polars` (lazy feature), `serde`, `serde_yaml`, `serde_json`, `uuid`, `chrono`, `anyhow`/`thiserror`

<!-- MANUAL ADDITIONS START -->
<!-- MANUAL ADDITIONS END -->
