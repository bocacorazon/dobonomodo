# dobonomodo Development Guidelines

Auto-generated from all feature plans. Last updated: 2026-02-22

## Active Technologies
- Rust 1.93.1 (edition 2021) + serde/serde_json (serialization), uuid (identifiers), chrono (date handling), polars (data processing context) (012-resolver-engine)
- PostgreSQL (entity metadata), object storage (trace files), file/database/catalog via DataSource adapters (012-resolver-engine)

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
- 012-resolver-engine: Added Rust 1.93.1 (edition 2021) + serde/serde_json (serialization), uuid (identifiers), chrono (date handling), polars (data processing context)
- 012-resolver-engine: Added Rust 1.93.1 (edition 2021) + serde/serde_json (serialization), uuid (identifiers), chrono (date handling), polars (data processing context)

- 001-workspace-scaffold: Added Rust (stable toolchain policy; project MSRV to be pinned when CI toolchain baseline is introduced) + `polars` (lazy feature), `serde`, `serde_yaml`, `serde_json`, `uuid`, `chrono`, `anyhow`/`thiserror`

<!-- MANUAL ADDITIONS START -->
<!-- MANUAL ADDITIONS END -->
