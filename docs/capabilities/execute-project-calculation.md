# Capability: Execute Project Calculation

**Status**: Draft  
**Created**: 2026-02-22  
**Domain**: Computation / Execution

## Definition

Execute Project Calculation is the process by which the system, given an active Project and a target Period, reads the Project's input Dataset rows for that Period and executes all Operations defined in the Project in sequence — producing a Run record, writing any projected output to configured destinations, and optionally registering output projections as new Datasets in the system.

## Purpose & Role

This is the core execution capability of DobONoMoDo. Without it, Projects are inert definitions — no computation occurs and no results are produced. It is the bridge between the declarative Operation pipeline (the *what*) and the actual transformation of data (the *done*). Every result in the system — every enriched row, every written output, every registered projection Dataset — is a product of this capability.

## Inputs

| Input | Type | Required | Constraints | Description |
|---|---|---|---|---|
| `project_id` | `UUID` | Yes | Project MUST be in `active` status | The Project whose Operations are to be executed |
| `period_id` | `UUID` | Yes | Period MUST exist in the system | The Period that scopes the input data and labels the Run |
| `resume_from_seq` | `Integer` | No | Must reference a valid Operation `seq` in the Project; only used when resuming a failed Run | When provided, execution begins at this Operation sequence rather than the first |

## Outputs

| Output | Type | Description |
|---|---|---|
| `run` | `Run` | A Run record capturing execution status, ProjectSnapshot, period, timing, and lineage. Status is `completed` on success, `failed` on error. |
| `output_writes` | `List<WriteResult>` | One entry per `output` operation that executed — destination written, row count, columns included |
| `registered_datasets` | `List<Dataset>` | Zero or more new (or new-version) Datasets registered by `output` operations with `register_as_dataset` set |

## Trigger

Initiated in one of two ways:

1. **Manual** — A user explicitly invokes the calculation for a given Project and Period.
2. **Scheduled** — A Scheduler entity fires the calculation automatically according to a configured schedule tied to a Project and Period pattern.

In both cases the system creates a Run record and begins execution.

## Preconditions

- **PRE-001**: The referenced Project MUST exist and MUST be in `active` status. A `draft`, `inactive`, or `conflict` Project cannot be calculated.
- **PRE-002**: The referenced Period MUST exist in the system.
- **PRE-003**: There MUST NOT be an existing Run in `queued` or `running` status for the same Project + Period combination. Concurrent calculations on the same Project+Period are prohibited to prevent output write conflicts.
- **PRE-004**: When `resume_from_seq` is provided, a `failed` Run for the same Project + Period MUST already exist, and `resume_from_seq` MUST match the `last_completed_operation + 1` of that Run.

## Postconditions

- **POST-001**: A Run record exists with a terminal status (`completed` or `failed`), recording the exact ProjectSnapshot used and the Period.
- **POST-002**: All `output` operations that executed successfully have written their data to their configured destinations. This data is NOT rolled back on subsequent failure.
- **POST-003**: Any `output` operations with `register_as_dataset` set have created or versioned the corresponding Dataset entity, which is immediately available as input to other Projects.
- **POST-004**: If the Run succeeded, `last_completed_operation` on the Run equals the `seq` of the final Operation.
- **POST-005**: If the Run failed, `last_completed_operation` records the last successfully completed Operation seq, enabling resume.

## Error Cases

| Error | Trigger Condition | Handling |
|---|---|---|
| `ProjectNotActive` | Project is not in `active` status at invocation time | Reject immediately; no Run is created |
| `PeriodNotFound` | The provided `period_id` does not exist | Reject immediately; no Run is created |
| `ConcurrentRunConflict` | A Run in `queued` or `running` status already exists for the same Project + Period | Reject immediately; no Run is created |
| `InvalidResumePoint` | `resume_from_seq` provided but no matching failed Run exists, or seq does not align with `last_completed_operation + 1` | Reject immediately; no Run is created |
| `OperationFailure` | An Operation errors during execution (e.g., dataset unavailable, expression evaluation error, write failure) | Transition Run to `failed`; record the error and the failing Operation seq in the Run; preserve all output written so far; allow future resume from `last_completed_operation + 1` |
| `DatasetUnavailable` | A dataset referenced by a RuntimeJoin or `append` operation is not accessible at execution time | Treated as `OperationFailure` at the point of the failing Operation |

## Boundaries

- This capability does **NOT** define or manage the schedule — that is the responsibility of the Scheduler entity.
- This capability does **NOT** pre-validate that all datasets referenced in Operation joins are available — validation is deferred to the point of use during execution.
- This capability does **NOT** roll back output already written by earlier `output` operations when a later operation fails.
- This capability does **NOT** execute Operations in parallel — the pipeline is strictly sequential by `seq`.
- This capability does **NOT** modify the Project or its Operations — it executes from an immutable ProjectSnapshot captured at Run creation.
- This capability does **NOT** select or filter the Period — the caller (user or Scheduler) provides the target Period explicitly.

## Dependencies

| Dependency | Type | Description |
|---|---|---|
| Project | Entity | Source of the Operation pipeline and input Dataset reference |
| Period | Entity | Scopes the input data rows and labels the Run |
| Dataset | Entity | The input data structure whose rows are processed by Operations |
| Run | Entity | Created and updated throughout execution to record status and lineage |
| Operation | Entity | The ordered units of work executed sequentially by this capability |
| Expression | Entity | Evaluated at runtime within Operations (selectors, assignments, aggregations) |
| DataSource | Entity | Resolved at runtime to access data for RuntimeJoins and `output` destinations |
| Scheduler | Entity | External trigger for scheduled invocations; out of scope for this capability |

## Open Questions

- [ ] Should the system support a **dry-run mode** — executing the pipeline without writing any output — to validate expressions and data availability before committing?
- [ ] When `register_as_dataset` creates a new Dataset version, should the old version remain accessible to other Projects, or should they be notified of the new version (triggering the Project conflict model)?
- [ ] How is the **Period filter** applied to the input Dataset exactly — by matching `_period` (or `_period_from`/`_period_to` for bitemporal tables) on the rows, or by some other mechanism?
