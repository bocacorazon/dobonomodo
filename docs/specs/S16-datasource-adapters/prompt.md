# S16: DataSource Adapters

## Feature
Implement `DataLoader` and `OutputWriter` trait implementations for the three strategy types: S3/object storage (Parquet, CSV), local filesystem, and database (via sqlx).

## Context
- Read: `docs/entities/datasource.md` (DataSource entity, connection types)
- Read: `docs/entities/resolver.md` (ResolvedLocation structure — what adapters receive)
- Read: `docs/architecture/system-architecture.md` (IO trait boundary, `object_store` crate, technology choices)

## Scope

### In Scope
- `engine-worker::io` module
- `S3DataLoader`: read Parquet/CSV from S3-compatible storage via `object_store` crate → Polars `LazyFrame`
- `FsDataLoader`: read Parquet/CSV from local filesystem → Polars `LazyFrame`
- `DbDataLoader`: read from database table/schema via `sqlx` → Polars `LazyFrame`
- Corresponding `OutputWriter` implementations for each
- Schema validation: loaded data columns must match `ColumnDef` list from Dataset (type + name)
- File format detection from path extension (.parquet, .csv)

### Out of Scope
- `catalog` strategy HTTP calls (deferred — returns endpoint info only)
- Credential management (use environment variables or connection strings for now)
- Connection pooling optimisation

## Dependencies
- **S00** (Workspace Scaffold): IO trait definitions

## Parallel Opportunities
Can start immediately after **S00** — runs in parallel with **S01, S02, S17, S18**.

## Success Criteria
- Parquet and CSV files load correctly into LazyFrame with correct types
- Database tables load correctly
- Schema validation catches type mismatches and missing columns
- Write to Parquet/CSV/database works correctly
- Integration tests with local filesystem and SQLite (for DB adapter)
