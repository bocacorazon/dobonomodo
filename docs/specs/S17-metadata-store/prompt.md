# S17: Metadata Store

## Feature
Implement the `MetadataStore` trait against PostgreSQL: entity CRUD for Datasets, Projects, Resolvers, Calendars, Periods, Runs, and DataSources. Include schema migrations.

## Context
- Read: `docs/architecture/system-architecture.md` (PostgreSQL metadata store, sqlx, migrations)
- Read all entity docs in `docs/entities/` for schema definitions
- Read: `docs/entities/run.md` (ProjectSnapshot, ResolverSnapshot — stored as JSONB)

## Scope

### In Scope
- `api-server::db` module (shared with engine-worker via a `db` library crate if needed)
- PostgreSQL schema: one table per entity, with version tracking columns
- JSONB for complex nested structures (ProjectSnapshot, ConflictReport, Resolver rules)
- `sqlx` compile-time checked queries
- Schema migrations via `sqlx-migrate`
- Version auto-increment: on update to Dataset, Project, Resolver → increment version
- `MetadataStore` trait implementation: all methods from S00 trait definition
- One-at-a-time Run guard: query for existing queued/running Run for same Project+Period

### Out of Scope
- API endpoints (S21)
- Auth/multi-tenancy
- Connection pooling configuration (use sqlx defaults)

## Dependencies
- **S00** (Workspace Scaffold): `MetadataStore` trait definition, entity model structs

## Parallel Opportunities
Can start immediately after **S00** — runs in parallel with **S01, S02, S16, S18**.

## Success Criteria
- All entity types can be created, read, updated, and soft-deleted
- Version auto-increment works on every update
- JSONB serialization/deserialization roundtrips correctly
- Migrations run cleanly on a fresh PostgreSQL database
- Compile-time query checking passes
