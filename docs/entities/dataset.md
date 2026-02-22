# Entity: Dataset

**Status**: Draft  
**Created**: 2026-02-21  
**Domain**: Data Layer / Computation Inputs

## Definition

A Dataset is a user-assembled **logical contract** consisting of one designated main table and zero or more lookups — each lookup being either a table schema or a nested Dataset — joined to the main table via explicit foreign key conditions. A Dataset defines only **what** data looks like (schema, structure, relationships) — it has no knowledge of where data physically lives or how to access it. Physical location is entirely the responsibility of the configured Resolver. Every row produced from a Dataset carries a fixed set of system-managed metadata columns — including a unique row identifier, lineage information, and user-defined labels — stored inline alongside the user data.

## Purpose & Role

A Dataset is the primary input definition for the computation engine. It specifies what data is available for computation — its shape, schema, and how the parts relate — without any knowledge of where or how data is physically stored. Physical location is resolved at runtime by the configured Resolver. Its reusability across multiple Projects makes it a shared, versioned contract between data structure and computation. By separating schema from location, the same Dataset definition can transparently serve data from different physical stores across different environments or Periods.

## Attributes

| Attribute | Type | Required | Constraints | Description |
|---|---|---|---|---|
| `id` | `UUID` | Yes | Immutable, system-generated | Unique identifier |
| `name` | `String` | Yes | Non-empty | Human-readable name |
| `description` | `String` | No | — | Optional narrative description |
| `owner` | `User` | Yes | Must reference a valid user | The user who assembled this Dataset |
| `version` | `Integer` | Yes | Auto-incremented on every change, starts at 1 | Tracks evolution of the definition |
| `status` | `Enum` | Yes | `active` \| `disabled` | Controls availability for use in new Projects |
| `resolver_id` | `String` | No | When absent, the system default Resolver is used | Identifies the Resolver implementation to use when loading data for this Dataset |
| `main_table` | `TableRef` | Yes | Must include a name and at least one column definition | The primary table of the Dataset |
| `lookups` | `List<LookupDef>` | No | Each entry must include target (TableRef or DatasetRef) + join conditions | Ordered list of lookup joins |
| `natural_key_columns` | `List<String>` | No | Column names must exist in the main table; defines the user-provided row identity | User-designated columns that form a natural key, preserved alongside the system-generated `_row_id` |
| `created_at` | `Timestamp` | Yes | System-set on creation, immutable | Creation time |
| `updated_at` | `Timestamp` | Yes | System-set on every change | Last modification time |

### TableRef (embedded structure)

| Attribute | Type | Required | Constraints | Description |
|---|---|---|---|---|
| `name` | `String` | Yes | Non-empty; unique within the Dataset | Logical name for the table within this Dataset |
| `columns` | `List<ColumnDef>` | Yes | At least one column; names must not use `_` prefix | Explicit schema definition for this table |
| `bitemporal` | `Boolean` | No | Default `false`; when `true`, replaces `_period` with `_period_from`/`_period_to` and adds `_valid_from`/`_valid_to` on rows | Enables bitemporal tracking on this table — see [[Bitemporal Dataset]] |

### ColumnDef (embedded structure)

| Attribute | Type | Required | Constraints | Description |
|---|---|---|---|---|
| `name` | `String` | Yes | Non-empty; no `_` prefix | Column name |
| `type` | `Enum` | Yes | `string \| integer \| decimal \| boolean \| date \| timestamp` | Declared column type; used for Expression type-checking and schema validation at resolve time |
| `nullable` | `Boolean` | No | Default `true` | Whether the column may contain NULL values |
| `description` | `String` | No | — | Optional narrative description |

### LookupDef (embedded structure)

| Attribute | Type | Required | Constraints | Description |
|---|---|---|---|---|
| `target` | `TableRef \| DatasetRef` | Yes | TableRef defines the schema of the lookup table; DatasetRef references an existing Dataset by id | The lookup target |
| `join_conditions` | `List<JoinCondition>` | Yes | At least one condition; each specifies FK column on both sides | Explicit foreign key join conditions |
| `alias` | `String` | No | Unique within the Dataset | Optional alias for the lookup in DSL expressions |

### JoinCondition (embedded structure)

| Attribute | Type | Required | Constraints | Description |
|---|---|---|---|---|
| `source_column` | `ColumnRef` | Yes | Column on the parent table/dataset side | FK source column |
| `target_column` | `ColumnRef` | Yes | Column on the lookup target side | FK target column |

### Row Metadata (system-managed — present on every row of a Dataset output)

All system metadata column names are prefixed with `_` and MUST NOT conflict with user-defined column names.

| Column | Type | Mutable | Description |
|---|---|---|---|
| `_row_id` | `UUID v7` | No | System-generated time-ordered unique identifier for the row; preserves insertion order |
| `_source_dataset_id` | `UUID` | No | ID of the Dataset from which this row originates |
| `_source_table` | `String` | No | Logical name of the source table within the originating Dataset |
| `_created_by_project_id` | `UUID` | No | ID of the Project whose Run produced this row |
| `_created_by_run_id` | `UUID` | No | ID of the specific Run execution that produced this row |
| `_created_at` | `Timestamp` | No | When the row was first produced; ISO 8601 |
| `_updated_at` | `Timestamp` | Yes | When the row was last modified by any operation; ISO 8601 |
| `_period` | `String` | No | A string identifier referencing a Period entity (e.g., `"2026-01"`, `"FY2026-Q1"`); meaning and rollup rules are defined by the Period entity | The period this row belongs to; set at import time or inherited from the Project Run |
| `_deleted` | `Boolean` | Yes | Default `false`. Set to `true` by a `delete` operation. Rows with `_deleted = true` are excluded from all subsequent pipeline operations and from `output` by default |
| `_labels` | `Map<String, String>` | Yes | User-defined key-value tags; any operation may add, update, or remove entries |

## Relationships

| Relationship | Related Entity | Cardinality | Description |
|---|---|---|---|
| has main table | TableRef | 1:1 | Every Dataset has exactly one designated main table with an explicit schema |
| has lookups | TableRef or Dataset | 1:N | A Dataset has zero or more lookup definitions, each with an explicit schema or nested Dataset reference |
| resolved by | Resolver | N:1 | At runtime, the configured Resolver (default or Dataset-level override) locates and loads the physical data |
| used by | Project | N:M | A Dataset is independent of any single Project and may be referenced by many Projects |

## Behaviors & Rules

- **BR-001**: A Dataset MUST have exactly one main table.
- **BR-002**: Pre-defined lookups (joins specified at assembly time) are ALWAYS included in every computation that uses the Dataset — they cannot be conditionally skipped.
- **BR-003**: Each lookup join MUST have at least one explicit foreign key condition defined at assembly time.
- **BR-004**: Cross-source joins are permitted — the main table and any lookup may reside in different data sources.
- **BR-005**: A lookup target may be either a raw Table or a nested Dataset (recursive composition). From the parent Dataset's perspective, a nested Dataset is just another lookup.
- **BR-006**: Dataset version MUST be auto-incremented on every structural change (change to main table, addition/removal/modification of any lookup or join condition).
- **BR-007**: A Dataset that is referenced by one or more Projects MUST NOT be deleted. The preferred action is to disable it.
- **BR-008**: A disabled Dataset MUST NOT be selectable for use in new Projects. Existing Project references to a disabled Dataset remain valid and unaffected.
- **BR-009**: A Dataset is agnostic of how its relationships are materialised (eagerly flattened vs. resolved at runtime) — that decision belongs to the Project.
- **BR-010**: Every TableRef in a Dataset MUST declare at least one ColumnDef. A TableRef with no columns is invalid.
- **BR-011**: ColumnDef `name` values within a TableRef MUST be unique and MUST NOT use the `_` prefix (reserved for system metadata).
- **BR-012**: User-defined column names MUST NOT use the `_` prefix — that namespace is reserved exclusively for system-managed metadata columns.
- **BR-013**: Every row produced from a Dataset MUST carry a system-generated UUID v7 `_row_id`. This value is immutable once assigned.
- **BR-014**: If `natural_key_columns` is defined, those columns are preserved as regular data columns alongside `_row_id`. The system does not replace or shadow them.
- **BR-015**: Lineage columns (`_source_dataset_id`, `_source_table`, `_created_by_project_id`, `_created_by_run_id`, `_created_at`) are immutable once set on a row.
- **BR-016**: `_labels` is mutable. Any operation in a Project MAY add, update, or remove entries. Label keys and values are user-defined strings with no system-imposed schema.
- **BR-017**: `_updated_at` MUST be updated by the system whenever any operation modifies a row, including label changes.
- **BR-018**: `_period` is an optional string identifier whose meaning is governed by the Period entity. It MAY be set at import time or inherited from the Run that produced the row. Any operation MAY update `_period` on a row.
- **BR-019**: `_deleted` is set to `false` on all rows when first produced. Only a `delete` operation may set it to `true`. Once set, the engine automatically excludes the row from all subsequent operations and from `output` writes (unless `include_deleted: true` is set on the `output` operation).

## Lifecycle

A Dataset is mutable and evolves over time. Version is tracked automatically. Deletion is guarded by active references.

| State | Description | Transitions To |
|---|---|---|
| `active` | Dataset is fully operational; can be referenced by new Projects | `disabled` |
| `disabled` | Dataset cannot be used in new Projects; existing references remain valid | `active` (re-enabled), or hard deletion only if zero Project references exist |

**What creates a Dataset**: A user explicitly assembles it by designating a main table and defining lookup joins.  
**What modifies a Dataset**: Any change to the main table, or addition, removal, or modification of a lookup or join condition. Each modification auto-increments the version.  
**What destroys a Dataset**: Hard deletion is only permitted when there are no Project references. The preferred alternative to deletion is disabling.

## Boundaries

- This entity does NOT store or own actual data — it is a logical schema contract only.
- This entity does NOT know where data physically lives — that is the responsibility of the Resolver.
- This entity does NOT determine how joins are materialised (flattened/denormalised vs. runtime-resolved) — that is a Project-level concern.
- This entity is NOT tied to a specific Project — it is a reusable, independently versioned definition.
- This entity does NOT represent a query, computation, or result — it defines the input shape available to the computation engine.
- This entity does NOT define what calculations are performed — that is the responsibility of the DSL and Computation Engine.

## Serialization (YAML DSL)

Schema for serializing a Dataset for inter-component communication.

```yaml
# dataset.schema.yaml
dataset:
  id: uuid                      # system-generated, immutable
  name: string                  # required, non-empty
  description: string           # optional
  owner: string                 # required, user identifier
  version: integer              # auto-incremented; starts at 1
  status: active | disabled     # required; default: active
  resolver_id: string           # optional; when absent, system default Resolver is used
  created_at: timestamp         # system-set on creation; ISO 8601; immutable
  updated_at: timestamp         # system-set on every change; ISO 8601
  natural_key_columns: [string] # optional; column names from main_table that form the natural key
  main_table:
    name: string                # required; logical name within this Dataset
    bitemporal: boolean         # optional; default false
    columns:                    # required; at least one
      - name: string            # required; no _ prefix
        type: string | integer | decimal | boolean | date | timestamp
        nullable: boolean       # optional; default true
        description: string     # optional
  lookups:                      # optional; ordered list
    - alias: string             # optional; unique within Dataset
      target:
        type: table | dataset   # required; discriminator
        # when type is `table`:
        name: string            # logical name within this Dataset
        bitemporal: boolean     # optional; default false
        columns:
          - name: string
            type: string | integer | decimal | boolean | date | timestamp
            nullable: boolean
        # when type is `dataset`:
        id: uuid                # references an existing Dataset by id
      join_conditions:          # required; at least one entry
        - source_column: string # column on the parent (main table or parent lookup) side
          target_column: string # column on this lookup's target side
```

> `created_at`, `updated_at`, and `version` are system-managed and MUST be treated as read-only by any component that consumes this format.

The following metadata columns are automatically present on **every row** of a Dataset output. They are not declared in the Dataset definition — they are injected by the system at runtime.

```yaml
# Row metadata (system-managed, inline on every row — not declared in dataset definition)
_row_id: uuid-v7              # system-generated, time-ordered, immutable
_source_dataset_id: uuid      # ID of the originating Dataset
_source_table: string         # logical name of the originating table
_created_by_project_id: uuid  # ID of the Project that produced this row
_created_by_run_id: uuid      # ID of the Run execution that produced this row
_created_at: timestamp        # ISO 8601; immutable
_updated_at: timestamp        # ISO 8601; updated on every modification
_deleted: boolean             # default false; set true by delete operation; excluded from pipeline by default
_period: string               # optional; period identifier (e.g. "2026-01"); meaning defined by Period entity
_labels:                      # user-defined key-value tags; mutable by any operation
  <key>: <value>
```

```yaml
# Example: Orders joined to customers and a nested product-categories Dataset
dataset:
  id: "d1a2b3c4-0000-0000-0000-000000000001"
  name: "Sales Orders"
  description: "Orders with customer details and product category hierarchy"
  owner: "user-marcos"
  version: 4
  status: active
  resolver_id: "legacy-parquet-resolver"   # overrides default for this Dataset
  created_at: "2026-02-21T10:00:00Z"
  updated_at: "2026-02-21T15:45:00Z"
  natural_key_columns: [order_number]
  main_table:
    name: orders
    columns:
      - name: order_number
        type: string
        nullable: false
      - name: customer_id
        type: string
        nullable: false
      - name: amount
        type: decimal
      - name: status
        type: string
      - name: region
        type: string
  lookups:
    - alias: customers
      target:
        type: table
        name: customers
        columns:
          - name: id
            type: string
            nullable: false
          - name: country_code
            type: string
          - name: tier
            type: string
      join_conditions:
        - source_column: customer_id
          target_column: id
    - alias: product_categories
      target:
        type: dataset
        id: "d5e6f7g8-0000-0000-0000-000000000002"
      join_conditions:
        - source_column: product_id
          target_column: product_id
```

## Related Entities

- [[Project]] — A Project references a Dataset to determine what data its computations operate on; multiple Projects may share the same Dataset.
- [[Resolver]] — The Resolver is responsible for locating and loading the physical data for each table in the Dataset for a given Period; the Dataset has no knowledge of physical location.
- [[ComputationEngine]] — Consumes a Dataset as its primary input definition when executing DSL programs.
- [[DSL]] — May request dynamic (runtime) joins on top of the Dataset's pre-defined structure.
- [[Period]] — The `_period` metadata column on each row stores a Period string identifier; the Period entity defines its meaning, boundaries, and rollup hierarchy.
- [[Bitemporal Dataset]] — A subtype where individual tables are flagged `bitemporal: true`, replacing `_period` with `_period_from`/`_period_to` and adding `_valid_from`/`_valid_to` axes.
