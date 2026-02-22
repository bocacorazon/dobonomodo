# S00: Workspace Scaffold

## Feature
Set up the Cargo workspace with all crate skeletons, entity model structs, IO trait definitions, and empty module stubs — the entire project must compile with `cargo build`.

## Context
- Read: `docs/architecture/system-architecture.md` (Cargo workspace structure, IO trait boundary, key technology choices)
- Read: `docs/architecture/sample-datasets.md` (entity structures to model)
- Read all entity docs in `docs/entities/` for struct definitions

## Scope

### In Scope
- Cargo workspace root `Cargo.toml` with member crates: `core`, `api-server`, `engine-worker`, `cli`, `test-resolver`
- `core` crate with module structure: `model/`, `dsl/`, `engine/`, `resolver/`, `trace/`, `validation/`
- Entity model structs in `core::model` with serde derives: `Dataset`, `TableRef`, `ColumnDef`, `LookupDef`, `Project`, `OperationInstance`, `Run`, `ProjectSnapshot`, `ResolverSnapshot`, `Resolver`, `ResolutionRule`, `ResolutionStrategy`, `Expression` (as a String newtype), `Calendar`, `Period`, `DataSource`
- Enum types: `TemporalMode`, `ColumnType`, `RunStatus`, `ProjectStatus`, `OperationKind`, `StrategyType`, `TriggerType`
- IO traits in `core`: `DataLoader`, `OutputWriter`, `MetadataStore`, `TraceWriter` — with method signatures matching the architecture doc
- Empty `main.rs` for each binary crate (must compile)
- `Cargo.toml` dependencies: `polars` (lazy feature), `serde`, `serde_yaml`, `serde_json`, `uuid`, `chrono`, `anyhow`/`thiserror`
- Basic `lib.rs` re-exports in `core`

### Out of Scope
- Any implementation logic — all functions may be `todo!()` or unimplemented stubs
- PostgreSQL, axum, kube-rs, clap dependencies (added in their respective specs)
- Tests beyond `cargo build` succeeding

## Dependencies
None — this is the first spec.

## Parallel Opportunities
Once this spec is complete, S01, S02, S16, and S17 can all start in parallel.

## Key Design Decisions (do not re-debate)
- Language: Rust
- Computation engine: Polars (lazy API)
- Workspace layout: monorepo with 5 crates as defined in the architecture doc
- IO is injected via traits — `core` has zero IO dependencies
- All entity model structs use `serde` for YAML/JSON serialization

## Success Criteria
- `cargo build` succeeds with zero errors
- `cargo test` succeeds (even if there are no tests yet)
- All entity structs can be deserialized from YAML using `serde_yaml`
- All IO traits are defined with correct method signatures
