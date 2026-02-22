# S18: Trace Writer

## Feature
Implement the `TraceWriter` trait: write `TraceEvent` structs as Parquet files to object storage, partitioned by Run ID.

## Context
- Read: `docs/capabilities/trace-run-execution.md` (trace event structure, storage decisions)
- Read: `docs/architecture/system-architecture.md` (trace stored as files alongside output, `object_store` crate)

## Scope

### In Scope
- `engine-worker::io::trace` module
- `S3TraceWriter` implementing `TraceWriter` trait
- Serialize `Vec<TraceEvent>` → Parquet file
- Write to object storage at path: `{base_path}/runs/{run_id}/trace.parquet`
- Include all fields: `run_id`, `operation_order`, `row_id`, `change_type`, `diff` (as JSON column)
- `FsTraceWriter` for local development/testing

### Out of Scope
- Trace querying/reading (deferred)
- Partitioning by operation (deferred — single file per Run for now)

## Dependencies
- **S12** (Trace Engine): `TraceEvent` struct definition

## Parallel Opportunities
Can run in parallel with **S16, S17** (other IO implementations).

## Success Criteria
- Trace events written as valid Parquet
- Readable back with correct schema
- Works with both S3 and local filesystem
