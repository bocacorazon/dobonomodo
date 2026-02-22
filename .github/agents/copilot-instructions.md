# dobonomodo Development Guidelines

Auto-generated from all feature plans. Last updated: 2026-02-22

## Active Technologies

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

cargo test [ONLY COMMANDS FOR ACTIVE TECHNOLOGIES][ONLY COMMANDS FOR ACTIVE TECHNOLOGIES] cargo clippy

## Code Style

Rust (stable toolchain policy; project MSRV to be pinned when CI toolchain baseline is introduced): Follow standard conventions

## Recent Changes

- 001-workspace-scaffold: Added Rust (stable toolchain policy; project MSRV to be pinned when CI toolchain baseline is introduced) + `polars` (lazy feature), `serde`, `serde_yaml`, `serde_json`, `uuid`, `chrono`, `anyhow`/`thiserror`

<!-- MANUAL ADDITIONS START -->
<!-- MANUAL ADDITIONS END -->
