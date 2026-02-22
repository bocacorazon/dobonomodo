# S19: Engine Worker Binary

## Feature
Build the `engine-worker` binary: receive a `RunSpec`, load metadata, resolve data locations, load data, execute the pipeline, write output and trace events, and report Run status back to PostgreSQL.

## Context
- Read: `docs/architecture/system-architecture.md` (engine worker responsibilities, data flow diagram)
- Read: `docs/entities/run.md` (Run lifecycle, ProjectSnapshot)

## Scope

### In Scope
- `engine-worker/src/main.rs`
- Receive `RunSpec` (Run ID passed as argument; fetch full spec from PostgreSQL)
- Update Run status: `queued → running`
- Load Resolver via `MetadataStore`; evaluate rules via `core::resolver`; get `Vec<ResolvedLocation>`
- Load data via `DataLoader` implementations (S16)
- Apply period filter (S03) to loaded data
- Execute pipeline via `core::engine::pipeline` (S10)
- Generate trace events via `core::trace` (S12)
- Write output via `OutputWriter` (S16)
- Write trace via `TraceWriter` (S18)
- Update Run status: `running → completed` (with `output_dataset_id`) or `running → failed` (with `ErrorDetail`)
- Graceful error handling: any panic or unrecoverable error → status `failed` with detail

### Out of Scope
- K8s Job definition (S21)
- Sandbox redirect logic (API server responsibility)
- Resume from partial failure (deferred)

## Dependencies
- **S10** (Pipeline Executor), **S11** (Resolver Engine), **S12** (Trace Engine), **S16** (DataSource Adapters), **S17** (Metadata Store), **S18** (Trace Writer)

## Parallel Opportunities
**S20** (CLI) can run in parallel once its dependencies (S10, S13) are met.

## Success Criteria
- Worker binary compiles and runs as a standalone process
- End-to-end: receives Run ID → loads → resolves → executes → writes → reports status
- Failure at any stage results in `failed` status with `ErrorDetail`
- Run status is correctly updated in PostgreSQL at each transition
