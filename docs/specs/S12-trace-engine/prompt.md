# S12: Trace Engine

## Feature
Implement trace event generation: capture before/after diffs for each operation in the pipeline, detect change types (`created`/`updated`/`deleted`), and produce `TraceEvent` structs for downstream persistence.

## Context
- Read: `docs/capabilities/trace-run-execution.md` (trace design — diffs only, reconstruction algorithm, change types, output not traced)
- Read: `docs/architecture/sample-datasets.md` (TS-09 trace validation scenario)

## Scope

### In Scope
- `core::trace` module
- Before/after diff: snapshot `_row_id` + column values before an operation, compare after
- Change type detection:
  - `created`: new `_row_id` present after (from aggregate/append)
  - `updated`: existing `_row_id` with changed column values
  - `deleted`: existing `_row_id` with `_deleted` changed to `true`
- `TraceEvent` struct: `run_id`, `operation_order`, `row_id`, `change_type`, `diff` (changed columns only: old/new values)
- Integration with pipeline executor (S10): hook before/after each operation
- `output` operations produce NO trace events
- Full row reconstruction algorithm: find `created` event → apply `updated` diffs in order → apply `deleted`
- Test scenario TS-09: trace events for FX update

### Out of Scope
- Trace persistence (S18)
- Cross-run comparison (deferred)
- Query API over trace events (deferred)

## Dependencies
- **S10** (Pipeline Executor): hook into operation execution

## Parallel Opportunities
Can run in parallel with **S13** and **S15** once S10 is complete.

## Success Criteria
- Updated rows produce trace events with only changed columns
- New rows (from aggregate/append) produce `created` events with full snapshot
- Deleted rows produce `deleted` events
- `output` operations produce no trace events
- Full row can be reconstructed from trace events for a given `_row_id`
