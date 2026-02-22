# Entity: Project

**Status**: Draft  
**Created**: 2026-02-21  
**Domain**: Computation / Orchestration

## Definition

A Project is a reusable, ordered sequence of parameterized operations — selected from a predefined library — applied to an input Dataset. It acts as a recipe: it defines *what* to compute and *how* to output the results, but does not execute itself. Execution is the concern of a Run. A Project produces an output that is itself a Dataset (registered or transient), enabling Projects to be composed by feeding one Project's output into another.

## Purpose & Role

A Project is the central unit of computation intent in DobONoMoDo. Without it, the computation engine has no instructions to follow. It binds a Dataset (the data) to an ordered set of operations (the logic) and declares where results should go. Its reusability as a recipe — combined with composability through sub-projects — makes it the primary mechanism for building complex, multi-stage data pipelines.

## Attributes

| Attribute | Type | Required | Constraints | Description |
|---|---|---|---|---|
| `id` | `UUID` | Yes | Immutable, system-generated | Unique identifier |
| `name` | `String` | Yes | Non-empty | Human-readable name |
| `description` | `String` | No | — | Optional narrative description |
| `owner` | `User` | Yes | Must reference a valid user | The user who created the Project |
| `version` | `Integer` | Yes | Auto-incremented on every change; starts at 1 | Tracks evolution of the recipe |
| `status` | `Enum` | Yes | `draft` \| `active` \| `inactive` \| `conflict` | Controls editability and executability |
| `visibility` | `Enum` | Yes | `private` \| `public`; user-controlled | Whether the Project is accessible to other users |
| `input_dataset_id` | `UUID` | Yes | Must reference a valid, active registered Dataset | The Dataset this Project operates on |
| `input_dataset_version` | `Integer` | Yes | Pinned at activation; updated only on explicit user upgrade | The specific Dataset version this Project is bound to |
| `materialization` | `Enum` | Yes | `eager` \| `runtime` | How pre-defined Dataset joins are resolved; applies to all joins uniformly |
| `operations` | `List<OperationInstance>` | Yes | At least one entry; executed in declared order | The ordered sequence of operations that constitute the recipe |
| `selectors` | `Map<String, Expression>` | No | Keys are unique names (no spaces); values are boolean Expression strings | Named, reusable row filters scoped to this Project. Referenced in Operation `selector` fields as `{{NAME}}` |
| `resolver_overrides` | `Map<UUID, String>` | No | Keys are Dataset IDs referenced in operations; values are Resolver `id`s | Per-Dataset Resolver overrides for this Project; takes precedence over the Dataset's own `resolver_id`. Useful for testing against alternative data sources |
| `conflict_report` | `ConflictReport` | No | Present only when `status` is `conflict` | Describes which Dataset changes broke which operations |
| `created_at` | `Timestamp` | Yes | System-set on creation; immutable | Creation time |
| `updated_at` | `Timestamp` | Yes | System-set on every change | Last modification time |

### OperationInstance (embedded structure)

| Attribute | Type | Required | Constraints | Description |
|---|---|---|---|---|
| `order` | `Integer` | Yes | Unique within the Project; defines execution sequence | Position in the pipeline |
| `type` | `String` | Yes | Must reference a valid entry in the Operation Library | The operation to perform |
| `alias` | `String` | No | Unique within the Project | Optional name for referencing this step's output in subsequent operations |
| `parameters` | `Map<String, Any>` | Yes | Keys and values are type-specific; may include selectors, columns, expressions, aggregations | Per-instance configuration for the operation |

> The `output` operation type is a first-class member of the Operation Library. It may appear at any position in the sequence and is the **only** operation type permitted to perform IO. Its `parameters` include a selector (which data to emit) and one or more destinations, each with a Location definition (same structure as Dataset's TableRef location) or a `transient` type.

### ConflictReport (embedded structure)

| Attribute | Type | Required | Constraints | Description |
|---|---|---|---|---|
| `dataset_version_from` | `Integer` | Yes | The pinned version at time of conflict detection | The Dataset version the Project was bound to |
| `dataset_version_to` | `Integer` | Yes | The new Dataset version that introduced breaking changes | The Dataset version that caused the conflict |
| `breaking_changes` | `List<BreakingChange>` | Yes | At least one entry | The specific changes that broke this Project |

### BreakingChange (embedded structure)

| Attribute | Type | Required | Constraints | Description |
|---|---|---|---|---|
| `column` | `String` | Yes | The affected column name | The column involved in the breaking change |
| `change_type` | `Enum` | Yes | `removed` \| `renamed` \| `type_changed` | What happened to the column |
| `affected_operations` | `List<Integer>` | Yes | Operation `order` values | Which operations in this Project reference the affected column |
| `resolution` | `Enum` | No | `adapted` \| `pinned`; set when resolved | How the conflict was resolved |

## Relationships

| Relationship | Related Entity | Cardinality | Description |
|---|---|---|---|
| consumes | Dataset | N:1 | A Project operates on exactly one input Dataset; many Projects may share the same Dataset |
| produces | Dataset | 1:1 | Each Project produces one output Dataset (registered or transient) as a result of execution |
| references (as sub-project) | Project | N:M | A Project's output Dataset may be used as the input Dataset of another Project, enabling composition |
| instantiated by | Run | 1:N | A Project may be executed many times; each execution is a separate Run entity |
| uses operations from | Operation Library | N:M | Each OperationInstance references one entry in the Operation Library |

## Behaviors & Rules

- **BR-001**: A Project MUST have exactly one input Dataset, which MUST be `active` at the time the Project is executed.
- **BR-002**: Operations MUST be executed strictly in the declared `order`; no parallel execution within a single Project.
- **BR-003**: Only an operation of type `output` MAY perform IO (write to disk, database, API, etc.). All other operation types MUST NOT produce side effects outside the computation pipeline.
- **BR-004**: An `output` operation MAY appear at any position in the sequence, including mid-pipeline (to checkpoint intermediate results).
- **BR-005**: A Project's output is always a Dataset. It may be registered (named, persisted, reusable by any Project) or transient (anonymous, accessible only within the scope of a Project that references it as a sub-project).
- **BR-006**: A transient output Dataset MUST NOT be referenced by any Project other than the one that immediately consumes it as a sub-project input.
- **BR-007**: The `materialization` strategy applies uniformly to all pre-defined joins in the input Dataset. Per-operation dynamic joins to additional tables are still permitted regardless of the strategy.
- **BR-008**: Version MUST be auto-incremented on every structural change (addition, removal, or modification of any operation or parameter; change of input Dataset or materialization strategy).
- **BR-009**: An `inactive` Project MUST NOT be executed (no new Runs may be created from it). Existing Runs are unaffected.
- **BR-010**: A `draft` Project MAY be edited freely and MAY be executed (manual Runs only), but all `output` operation writes are redirected to the deployment-level sandbox DataSource. Draft Projects MUST NOT write to production destinations.
- **BR-011**: `visibility` is user-controlled. A `private` Project is accessible only to its owner. A `public` Project is accessible to all users but may only be modified by its owner.
- **BR-012**: When a Project is activated, the user MUST confirm which version of the input Dataset to pin. `input_dataset_version` is set at that point and changes only on a subsequent explicit activation.
- **BR-012a**: Structural changes to an `active` Project (operations, `input_dataset_id`, `materialization`) MUST automatically revert it to `draft`. Metadata changes (`name`, `description`, `visibility`, `selectors`, `resolver_overrides`) do NOT trigger reversion.
- **BR-013**: When the input Dataset receives a new version, the system MUST automatically perform impact analysis against the Project's pinned version. A breaking change is any structural change (column removal, rename, or type change) affecting a column referenced in any operation's parameters.
- **BR-014**: Non-breaking Dataset changes (e.g., new columns or tables added) MUST be offered to the user as an optional upgrade. They MUST NOT automatically change `input_dataset_version` or affect Project execution.
- **BR-015**: Breaking Dataset changes MUST transition the Project to `conflict` status and populate `conflict_report`. A `conflict` Project MUST NOT be executed until the conflict is resolved.
- **BR-016**: A conflict is resolved by either: (a) **adapting** — updating the affected operation parameters to work with the new Dataset version, which advances `input_dataset_version` to the new version; or (b) **pinning** — explicitly rejecting the new Dataset version and remaining on the previous one, closing the conflict without upgrading.
- **BR-017**: When all entries in `conflict_report.breaking_changes` have a `resolution` set, the Project MUST automatically return to `active` status and `conflict_report` MUST be cleared.

## Lifecycle

A Project is a long-lived, reusable recipe. Execution state is tracked by the Run entity, not the Project itself.

| State | Description | Transitions To |
|---|---|---|
| `draft` | Development mode — freely editable and manually runnable; all outputs go to sandbox DataSource | `active` (on successful activation) |
| `active` | Production mode — outputs go to configured destinations; structural edits revert to `draft` | `draft` (structural edit), `inactive` (manual deactivation), `conflict` (breaking Dataset change) |
| `conflict` | A breaking Dataset change was detected; execution blocked until resolved | `draft` (on conflict resolution) |
| `inactive` | Suspended — not executable; no scheduled or manual Runs; no new sub-project references | `draft` (re-opened for editing) |

**What creates a Project**: A user explicitly creates it, designating an input Dataset, a materialization strategy, and an ordered list of operations.  
**What modifies a Project**: Any change to the operation sequence, parameters, input Dataset, materialization strategy, visibility, or status. Each modification auto-increments the version.  
**What destroys a Project**: Open question — see below.

## Boundaries

- This entity does NOT represent an execution — that is a **Run**.
- This entity does NOT define the available operations — those belong to the **Operation Library**.
- This entity does NOT own or store data — data is owned by Datasets and the locations they point to.
- This entity does NOT prescribe the DSL syntax for expressions, selectors, or aggregations — that is the **DSL** entity's concern.
- This entity does NOT manage scheduling or triggering of executions — that belongs to a future scheduling entity.

## Open Questions

- [ ] What are the deletion rules for a Project? Should it follow the same guardrails as Dataset (disable preferred; hard delete only when no Run references exist)?
- [ ] Can a `public` Project be forked/copied by another user into their own private Project?
- [ ] Can a Project reference multiple sub-projects (i.e., use outputs from more than one sub-project as inputs to different operations)?

## Serialization (YAML DSL)

Schema for serializing a Project for inter-component communication.

```yaml
# project.schema.yaml
project:
  id: uuid                          # system-generated, immutable
  name: string                      # required, non-empty
  description: string               # optional
  owner: string                     # required, user identifier
  version: integer                  # auto-incremented; starts at 1
  status: draft | active | inactive | conflict  # required
  visibility: private | public      # required; user-controlled
  created_at: timestamp             # system-set on creation; ISO 8601; immutable
  updated_at: timestamp             # system-set on every change; ISO 8601
  input_dataset_id: uuid            # required; references a registered, active Dataset
  input_dataset_version: integer    # required; pinned at activation; changed only on explicit upgrade
  materialization: eager | runtime  # required; applies to all pre-defined Dataset joins
  selectors:                        # optional; named reusable row filters scoped to this Project
    <name>: <boolean expression>    # referenced in operations as {{name}}
  conflict_report:                  # present only when status is `conflict`
    dataset_version_from: integer   # the pinned version at time of conflict detection
    dataset_version_to: integer     # the new Dataset version that introduced breaking changes
    breaking_changes:
      - column: string              # affected column name
        change_type: removed | renamed | type_changed
        affected_operations: [integer]  # operation order values that reference this column
        resolution: adapted | pinned    # set when resolved; absence means unresolved
  operations:                       # required; at least one entry
    - order: integer                # required; unique within Project; defines execution sequence
      type: string                  # required; must match an Operation Library entry
      alias: string                 # optional; unique within Project
      parameters:                   # required; keys are operation-type-specific
        <key>: <value>
```

> For the `output` operation type, `parameters` follows a specific structure:

```yaml
    - order: integer
      type: output
      alias: string                 # optional
      parameters:
        selector: string            # DSL expression defining which rows/columns to emit
        destinations:               # required; at least one
          - location:
              type: database | parquet | csv | api | transient
              options:              # same structure as Dataset TableRef location options
                <key>: <value>
```

> `created_at`, `updated_at`, and `version` are system-managed and MUST be treated as read-only.

```yaml
# Example: Monthly sales summary — computes totals, checkpoints mid-pipeline,
#          and writes final results to both Parquet and a database table
project:
  id: "p1a2b3c4-0000-0000-0000-000000000001"
  name: "Monthly Sales Summary"
  description: "Aggregates orders by region and product category; outputs to warehouse and data lake"
  owner: "user-marcos"
  version: 2
  status: active
  visibility: public
  created_at: "2026-02-21T11:00:00Z"
  updated_at: "2026-02-21T15:00:00Z"
  input_dataset_id: "d1a2b3c4-0000-0000-0000-000000000001"  # Sales Orders dataset
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
          - column: order_id
            function: count
            output_column: order_count
    - order: 3
      type: output
      alias: checkpoint
      parameters:
        selector: "*"
        destinations:
          - location:
              type: transient        # accessible to parent/downstream Projects
    - order: 4
      type: enrich
      alias: with_labels
      parameters:
        join:
          table: region_names
          location:
            type: csv
            options:
              path: "s3://my-bucket/reference/region_names.csv"
          on:
            source_column: region_id
            target_column: id
        columns: [region_name]
    - order: 5
      type: output
      parameters:
        selector: "*"
        destinations:
          - location:
              type: parquet
              options:
                path: "s3://my-bucket/output/monthly_sales/"
          - location:
              type: database
              options:
                connection_string: "postgresql://host:5432/warehouse"
                schema: reporting
                table: monthly_sales_summary
```

## Related Entities

- [[Dataset]] — A Project consumes one input Dataset and produces one output Dataset (registered or transient).
- [[Run]] — Each execution of a Project is a Run; execution state (running, completed, failed) belongs to Run, not Project.
- [[Operation Library]] — The catalog of available operation types that Project operations are selected from.
- [[DSL]] — Expressions, selectors, aggregations, and filters within operation parameters are written in the DSL.
