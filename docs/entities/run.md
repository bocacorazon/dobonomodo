# Entity: Run

**Status**: Draft  
**Created**: 2026-02-21  
**Domain**: Computation / Execution

## Definition

A Run is a single execution instance of a Project for one or more specified Periods. It captures a full snapshot of the Project's definition at the moment of execution — making it permanently self-contained and reproducible. A Run progresses through a defined set of states, tracks which operation it reached, and always preserves partial output on failure to enable resumption from the point of failure. Sub-project executions are first-class Runs in their own right, linked to their parent Run.

## Purpose & Role

A Run is the unit of execution in DobONoMoDo. Without it, Projects are inert recipes that never produce results. It binds a Project snapshot to a set of Periods, drives the computation engine through the ordered operation sequence, and produces an output Dataset. Its snapshot model ensures that the provenance of every Dataset row (via `_created_by_run_id`) is permanently traceable to the exact operations and parameters that produced it.

## Attributes

| Attribute | Type | Required | Constraints | Description |
|---|---|---|---|---|
| `id` | `UUID` | Yes | Immutable, system-generated | Unique identifier |
| `project_id` | `UUID` | Yes | Must reference a valid Project | The Project this Run executes |
| `project_version` | `Integer` | Yes | The Project version at time of execution | The specific Project version executed |
| `project_snapshot` | `ProjectSnapshot` | Yes | Full copy of the Project definition captured at execution time | Immutable snapshot ensuring permanent reproducibility |
| `period_ids` | `List<UUID>` | Yes | At least one; all must reference `open` or `closed` Periods | The Periods this Run is scoped to |
| `status` | `Enum` | Yes | `queued` \| `running` \| `completed` \| `failed` \| `cancelled` | Current execution state |
| `trigger_type` | `Enum` | Yes | `manual` \| `scheduled` | How the Run was initiated |
| `triggered_by` | `String` | Yes | User identifier or scheduler identifier | Who or what triggered the Run |
| `last_completed_operation` | `Integer` | No | Operation `order` value; present when `status` is `failed` or `running` | The last operation that completed successfully; used for resume-from-failure |
| `output_dataset_id` | `UUID` | No | References the output Dataset; populated on `completed` | The Dataset produced by this Run |
| `parent_run_id` | `UUID` | No | References a Run; present when this Run executes a sub-project | The parent Run that spawned this Run as a sub-project execution |
| `error` | `ErrorDetail` | No | Present only when `status` is `failed` | Details of the failure |
| `started_at` | `Timestamp` | No | Set when status transitions to `running` | Execution start time |
| `completed_at` | `Timestamp` | No | Set when status reaches a terminal state | Execution end time |
| `created_at` | `Timestamp` | Yes | System-set on creation; immutable | Time the Run was created (queued) |

### ProjectSnapshot (embedded structure)

A full copy of the Project definition captured immutably at execution time.

| Attribute | Type | Description |
|---|---|---|
| `input_dataset_id` | `UUID` | The Dataset used as input |
| `input_dataset_version` | `Integer` | The pinned Dataset version at time of execution |
| `materialization` | `Enum` | `eager` \| `runtime` — the strategy in effect |
| `operations` | `List<OperationInstance>` | Full ordered operation list with all parameters |

### ErrorDetail (embedded structure)

| Attribute | Type | Description |
|---|---|---|
| `operation_order` | `Integer` | The operation that caused the failure |
| `message` | `String` | Human-readable error description |
| `detail` | `String` | Technical detail or stack trace |

## Relationships

| Relationship | Related Entity | Cardinality | Description |
|---|---|---|---|
| executes | Project | N:1 | A Run executes one Project; a Project may have many Runs |
| scoped to | Period | N:M | A Run targets one or more Periods; a Period may be targeted by many Runs |
| produces | Dataset | 1:1 | A completed Run produces one output Dataset; failed Runs preserve partial output |
| child of | Run | N:1 | A sub-project Run has one parent Run; a parent Run may spawn many child Runs |
| spawns | Run | 1:N | A Run may spawn child Runs for each sub-project operation it encounters |

## Behaviors & Rules

- **BR-001**: A Run MUST capture a full `ProjectSnapshot` at the moment of execution. This snapshot is immutable and MUST NOT be updated after the Run is created.
- **BR-002**: A Run MUST NOT be created for a Project with `status` other than `active`.
- **BR-003**: A Run MUST NOT target a `locked` Period. It MAY target `open` or `closed` Periods.
- **BR-004**: Operations MUST be executed in the order declared in `project_snapshot.operations`. `last_completed_operation` MUST be updated after each operation completes.
- **BR-005**: On failure, the Run MUST preserve all partial output produced up to and including `last_completed_operation`. This output is retained until explicitly cleaned up.
- **BR-006**: A failed Run MAY be retried. Retry MUST resume execution from the operation immediately following `last_completed_operation`, reusing the preserved partial output.
- **BR-007**: When a sub-project operation is encountered during execution, the system MUST create a child Run for it with its own `id`, `status`, and snapshot. The parent Run MUST NOT advance past the sub-project operation until the child Run reaches a terminal state.
- **BR-008**: If a child Run fails, the parent Run MUST also transition to `failed`.
- **BR-009**: A `cancelled` Run MUST NOT be retried. Cancellation discards all partial output.
- **BR-010**: `output_dataset_id` MUST be populated only when the Run reaches `completed` status. Partial output from failed Runs is NOT registered as an output Dataset.
- **BR-011**: Partial output retained from a failed Run MAY be explicitly cleaned up by the user after the issue is resolved. The system MUST NOT automatically clean it up.

## Lifecycle

| State | Description | Transitions To |
|---|---|---|
| `queued` | Run is created and waiting to be picked up for execution | `running`, `cancelled` |
| `running` | Execution is in progress | `completed`, `failed`, `cancelled` |
| `completed` | All operations finished successfully; output Dataset is available | — (terminal) |
| `failed` | An operation encountered an error; partial output is preserved | `running` (on retry), or remains `failed` until cleaned up |
| `cancelled` | Execution was stopped before completion; partial output is discarded | — (terminal) |

**What creates a Run**: A user triggers it manually, or a scheduler triggers it automatically, against an `active` Project and one or more `open`/`closed` Periods.  
**What modifies a Run**: Status transitions and updates to `last_completed_operation`, `output_dataset_id`, `error`, `started_at`, and `completed_at` — all system-managed.  
**What destroys a Run**: Open question — see below.

## Boundaries

- A Run does NOT define what computations to perform — that is the **Project** and **DSL**.
- A Run does NOT manage scheduling or triggers — that is a future **Scheduler** entity.
- A Run does NOT own the output Dataset — it produces and references it; the Dataset entity owns its own lifecycle.
- A Run does NOT modify the Project — the snapshot is a read-only copy; the Project is never changed by execution.

## Open Questions

- [ ] What are the retention and deletion rules for completed Runs? Can they be deleted, or are they kept permanently for audit/lineage?
- [ ] Can a failed Run's partial output be promoted to a full output Dataset for inspection (outside of retry)?
- [ ] Should a retry create a new Run entity (preserving the failed Run for audit), or update the existing Run in place?

## Serialization (YAML DSL)

Schema for serializing a Run for inter-component communication.

```yaml
# run.schema.yaml
run:
  id: uuid                          # system-generated, immutable
  project_id: uuid                  # required; references the Project
  project_version: integer          # required; the Project version at execution time
  project_snapshot:                 # required; immutable copy of Project definition
    input_dataset_id: uuid
    input_dataset_version: integer
    materialization: eager | runtime
    operations:
      - order: integer
        type: string
        alias: string               # optional
        parameters:
          <key>: <value>
  period_ids: [uuid]                # required; at least one
  status: queued | running | completed | failed | cancelled  # required
  trigger_type: manual | scheduled  # required
  triggered_by: string              # required; user or scheduler identifier
  last_completed_operation: integer # optional; present during running or after failure
  output_dataset_id: uuid           # optional; populated on completed
  parent_run_id: uuid               # optional; present for sub-project Runs
  error:                            # optional; present only on failed
    operation_order: integer
    message: string
    detail: string
  started_at: timestamp             # optional; ISO 8601; set on transition to running
  completed_at: timestamp           # optional; ISO 8601; set on terminal state
  created_at: timestamp             # system-set on creation; ISO 8601; immutable
```

> All state fields (`status`, `last_completed_operation`, `output_dataset_id`, `error`, `started_at`, `completed_at`) are system-managed. Components consuming a Run MUST treat them as read-only.

```yaml
# Example: A completed manual Run of the "Monthly Sales Summary" project for two periods
run:
  id: "run-0000-0000-0000-000000000001"
  project_id: "p1a2b3c4-0000-0000-0000-000000000001"
  project_version: 2
  project_snapshot:
    input_dataset_id: "d1a2b3c4-0000-0000-0000-000000000001"
    input_dataset_version: 4
    materialization: eager
    operations:
      - order: 1
        type: filter
        alias: recent_orders
        parameters:
          selector: "order_date >= '2026-01-01'"
      - order: 2
        type: aggregate
        alias: regional_totals
        parameters:
          group_by: [region_id, product_category_id]
          aggregations:
            - column: total_amount
              function: sum
              output_column: total_sales
      - order: 3
        type: output
        parameters:
          selector: "*"
          destinations:
            - location:
                type: parquet
                options:
                  path: "s3://my-bucket/output/monthly_sales/"
  period_ids:
    - "per-0000-0000-0000-000000000001"   # 202601
    - "per-0000-0000-0000-000000000002"   # 202602
  status: completed
  trigger_type: manual
  triggered_by: "user-marcos"
  last_completed_operation: 3
  output_dataset_id: "d9e8f7a6-0000-0000-0000-000000000099"
  started_at: "2026-02-21T16:00:00Z"
  completed_at: "2026-02-21T16:04:32Z"
  created_at: "2026-02-21T15:59:55Z"
```

## Related Entities

- [[Project]] — A Run executes a Project; the ProjectSnapshot is a full copy of the Project's definition at execution time.
- [[Period]] — A Run is scoped to one or more Periods; the Period's status controls whether it can be targeted.
- [[Dataset]] — A Run consumes an input Dataset (via the snapshot) and produces an output Dataset on completion.
- [[Calendar]] — Periods referenced by a Run belong to a Calendar, providing temporal context.
