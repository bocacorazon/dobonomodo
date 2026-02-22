# Capability: Trace Run Execution

**Status**: Draft  
**Created**: 2026-02-22  
**Domain**: Computation / Observability

## Definition

Trace Run Execution is a two-part capability: during a Run, the engine passively records a **TraceRecord** for every row that is changed, created, or deleted by each Operation; after the Run completes, users can query those records by `_row_id` to reconstruct the full state of any row at any step of the pipeline. Only diffs (changed column values) are stored per step; the full row state at any step is reconstructed by replaying diffs forward from the row's creation event.

## Purpose & Role

Traceability makes the computation engine auditable and debuggable. Without it, the only observable output is the final state — there is no way to understand why a row has a particular value, which operation changed it, or what it looked like before. This capability enables users to answer: *"What was this row at step 3?"*, *"Which operation deleted this row?"*, and *"What columns did this operation change?"* — all by inspecting the passive trace produced during execution.

## Inputs

### Write path (during Run execution)

| Input | Type | Required | Description |
|---|---|---|---|
| `run_id` | `UUID` | Yes | The Run whose execution is being traced |
| `operation_seq` | `Integer` | Yes | The `seq` of the Operation that produced this change |
| `row_id` | `UUID` | Yes | The `_row_id` of the affected row |
| `change_type` | `Enum` | Yes | `created \| updated \| deleted` |
| `before` | `Map<String, Any>` | No | Changed column values before the operation. `null` for `created` events |
| `after` | `Map<String, Any>` | No | Changed column values after the operation. `null` for `deleted` events |

### Read path (query after Run)

| Input | Type | Required | Constraints | Description |
|---|---|---|---|---|
| `run_id` | `UUID` | Yes | Run must exist | Scopes the query to a specific Run |
| `row_id` | `UUID` | Yes | Must exist in the Run's trace | The row whose history to reconstruct |
| `at_step` | `Integer` | No | Must be a valid `seq` in the Run | Reconstruct state at a specific step. When omitted, returns the full step-by-step history |

## Outputs

### Write path

| Output | Type | Description |
|---|---|---|
| *(side effect)* | `TraceRecord` | Written to the trace store; associated with the Run |

### Read path

| Output | Type | Description |
|---|---|---|
| `row_history` | `List<TraceEntry>` | Ordered list of state snapshots for the row — one per step that changed it |
| `row_at_step` | `Map<String, Any>` | Full reconstructed row state at the requested step (when `at_step` is provided) |

### TraceRecord (stored structure)

| Field | Type | Description |
|---|---|---|
| `run_id` | `UUID` | The Run this trace belongs to |
| `operation_seq` | `Integer` | The Operation step that produced the change |
| `row_id` | `UUID` | `_row_id` of the affected row |
| `change_type` | `Enum` | `created \| updated \| deleted` |
| `before` | `Map<String, Any>` | Diff: only columns that changed, with their pre-change values. `null` for `created` |
| `after` | `Map<String, Any>` | Diff: only columns that changed, with their post-change values. `null` for `deleted` |

### TraceEntry (query result structure)

| Field | Type | Description |
|---|---|---|
| `operation_seq` | `Integer` | The step that produced this change |
| `change_type` | `Enum` | `created \| updated \| deleted` |
| `full_state` | `Map<String, Any>` | Complete row state after this step — reconstructed by replaying all prior diffs |
| `diff` | `Map<String, Any>` | Only the columns changed at this step (the raw diff stored in the TraceRecord) |

## Trigger

### Write path
Triggered automatically by the engine during each Operation execution within a Run. The engine writes a TraceRecord for each row that is created, updated, or deleted by the Operation. Trace writes are **synchronous with operation execution** (a step is not considered complete until its trace records are written).

### Read path
Triggered by a user or external system querying the trace for a specific `run_id` + `row_id` combination after the Run has completed.

## Preconditions

- **PRE-001** *(write)*: A Run in `running` status must exist for the given `run_id`.
- **PRE-002** *(read)*: The Run identified by `run_id` must exist.
- **PRE-003** *(read)*: At least one TraceRecord must exist for the given `row_id` in that Run (i.e., the row was affected by at least one operation).

## Postconditions

- **POST-001** *(write)*: For every Operation that completes, a TraceRecord exists for each row it changed. No Operation is considered complete if its trace records are missing.
- **POST-002** *(read)*: When `at_step` is provided, `row_at_step` contains the complete column state of the row as it existed after that step — reconstructed by applying all diffs from the `created` event through `at_step`.
- **POST-003** *(read)*: When `at_step` is omitted, `row_history` contains one TraceEntry per step that touched the row, in ascending `operation_seq` order.

## Reconstruction Algorithm

To reconstruct the full state of a row at step N:

1. Find the `created` TraceRecord for the `row_id` — its `after` map is the initial full row snapshot.
2. For each subsequent `updated` TraceRecord with `operation_seq ≤ N` (in ascending order), merge its `after` diff into the running state (later values overwrite earlier ones).
3. If a `deleted` TraceRecord exists with `operation_seq ≤ N`, set `_deleted = true` in the reconstructed state.

This produces the complete row state at step N without storing redundant full snapshots.

## Per-Operation Trace Behaviour

| Operation type | TraceRecord written? | `change_type` | `before` | `after` |
|---|---|---|---|---|
| `update` | Yes, for each matched+changed row | `updated` | Changed columns only, pre-change values | Changed columns only, post-change values |
| `delete` | Yes, for each soft-deleted row | `deleted` | `{ _deleted: false }` | `null` |
| `aggregate` | Yes, for each appended summary row | `created` | `null` | Full column snapshot of the new row |
| `append` | Yes, for each appended row | `created` | `null` | Full column snapshot of the new row |
| `output` | **No** | — | — | — |

## Error Cases

| Error | Trigger Condition | Handling |
|---|---|---|
| `TraceWriteFailure` | The trace store is unavailable or rejects a write during Operation execution | The Operation is considered failed; the Run transitions to `failed`; the failed step is recorded in `last_completed_operation` |
| `RowNotFound` *(read)* | No TraceRecord exists for the given `row_id` in the specified Run | Return empty result with a diagnostic; the row was either never changed by this Run or does not exist |
| `StepOutOfRange` *(read)* | `at_step` is provided but no Operation with that `seq` exists in the Run's ProjectSnapshot | Return error with a diagnostic listing valid step values |

## Boundaries

- This capability does **NOT** support live inspection of a running Run — trace is queryable only after the Run completes (or fails).
- This capability does **NOT** trace `output` operations — writing data to a destination produces no TraceRecord.
- This capability does **NOT** store full row snapshots at every step — only diffs. Full reconstruction is a read-time operation.
- This capability does **NOT** support cross-Run comparison — `_row_id` is scoped to a single Run; rows cannot be correlated across Runs without `natural_key_columns` (a future requirement).
- This capability does **NOT** apply to rows that were read but not changed by an Operation — only mutations are recorded.

## Dependencies

| Dependency | Type | Description |
|---|---|---|
| Run | Entity | Provides the execution context; TraceRecords are scoped to and deleted with the Run |
| Operation | Entity | Each TraceRecord references an `operation_seq` from the Run's ProjectSnapshot |
| Dataset | Entity | Row schema defines which column names and types appear in `before`/`after` diffs |

## Open Questions

- [ ] Should TraceRecords be written to the same store as the working dataset, or to a separate dedicated trace store? The answer affects performance and isolation.
- [ ] Should the read API support querying all rows changed at a given step (i.e., query by `run_id + operation_seq` in addition to `run_id + row_id`)?
- [ ] How should the reconstruction algorithm handle rows that appear in multiple `append`/`aggregate` operations with the same `_row_id` (if that is even possible)?
