# S15: Run Lifecycle

## Feature
Implement Run creation, `ProjectSnapshot` capture (including `ResolverSnapshot`s), status transitions (`queued → running → completed/failed/cancelled`), error recording, and the one-at-a-time guard per Project+Period.

## Context
- Read: `docs/entities/run.md` (Run attributes, ProjectSnapshot, ResolverSnapshot, ErrorDetail, status transitions, BRs)
- Read: `docs/capabilities/execute-project-calculation.md` (trigger types, error cases, partial output)

## Scope

### In Scope
- `core::model::run` — Run creation with full `ProjectSnapshot` assembly
- `ProjectSnapshot` capture: operations, `input_dataset_version`, `materialization`, `selectors`, `resolver_snapshots`
- `ResolverSnapshot` capture: for each resolved Dataset, record `dataset_id`, `resolver_id`, `resolver_version`
- Status transitions: `queued → running → completed`, `running → failed`, `running → cancelled`
- Error recording: `ErrorDetail` with operation_order, message, detail
- One-at-a-time guard: reject new Run for same Project+Period if one is already queued/running
- `started_at` set on transition to running; `completed_at` set on terminal state

### Out of Scope
- K8s Job creation (S19/S21)
- Resume Run capability (deferred)
- Sub-project Run creation (deferred)

## Dependencies
- **S10** (Pipeline Executor), **S12** (Trace Engine)

## Parallel Opportunities
Can run in parallel with **S13** (Activation Validation).

## Success Criteria
- ProjectSnapshot is a complete, immutable copy of the Project at execution time
- ResolverSnapshots are captured for every resolved Dataset
- Status transitions follow the defined state machine
- Concurrent Run for same Project+Period is rejected
- Error detail is captured on failure with correct operation_order
