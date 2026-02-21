# Entity: Dataset

**Status**: Draft  
**Created**: 2026-02-21  
**Domain**: Data Layer / Computation Inputs

## Definition

A Dataset is a user-assembled structural definition consisting of one designated main table and zero or more lookups — each lookup being either a raw table or a nested Dataset — joined to the main table via explicit foreign key conditions. A Dataset is agnostic of how its relationships are materialised; it describes only the shape and structure of the data available to the computation engine. It does not own or store data. Every row produced from a Dataset carries a fixed set of system-managed metadata columns — including a unique row identifier, lineage information, and user-defined labels — stored inline alongside the user data.

## Purpose & Role

A Dataset is the primary input definition for the computation engine. It specifies what data is available for computation and how the parts relate to one another, without prescribing how those relationships are resolved (that is a Project-level concern). Without a Dataset, the computation engine has no data to operate on. Its reusability across multiple Projects makes it a shared, versioned contract between data structure and computation.

## Attributes

| Attribute | Type | Required | Constraints | Description |
|---|---|---|---|---|
| `id` | `UUID` | Yes | Immutable, system-generated | Unique identifier |
| `name` | `String` | Yes | Non-empty | Human-readable name |
| `description` | `String` | No | — | Optional narrative description |
| `owner` | `User` | Yes | Must reference a valid user | The user who assembled this Dataset |
| `version` | `Integer` | Yes | Auto-incremented on every change, starts at 1 | Tracks evolution of the definition |
| `status` | `Enum` | Yes | `active` \| `disabled` | Controls availability for use in new Projects |
| `main_table` | `TableRef` | Yes | Must include a name and a valid Location definition | The primary table of the Dataset |
| `lookups` | `List<LookupDef>` | No | Each entry must include target (TableRef or DatasetRef) + join conditions | Ordered list of lookup joins |
| `natural_key_columns` | `List<String>` | No | Column names must exist in the main table; defines the user-provided row identity | User-designated columns that form a natural key, preserved alongside the system-generated `_row_id` |
| `created_at` | `Timestamp` | Yes | System-set on creation, immutable | Creation time |
| `updated_at` | `Timestamp` | Yes | System-set on every change | Last modification time |

### TableRef (embedded structure)

| Attribute | Type | Required | Constraints | Description |
|---|---|---|---|---|
| `name` | `String` | Yes | Non-empty; unique within the Dataset | Logical name for the table within this Dataset |
| `datasource_id` | `UUID` | No* | Must reference a valid, active DataSource; preferred over inline `location` | Reference to a named DataSource; table-specific detail (table name or relative path) supplied alongside |
| `table` | `String` | No* | Required when `datasource_id` is a `database` type | Table name within the DataSource's database/schema |
| `path` | `String` | No* | Required when `datasource_id` is a `parquet`, `csv`, or `api` type | Relative path/endpoint appended to the DataSource's `path_prefix` or `endpoint` |
| `location` | `Location` | No* | Used for one-off connections when no DataSource is appropriate | Inline location definition; fallback when `datasource_id` is not provided |

> \* Exactly one of `datasource_id` (+ `table`/`path` as appropriate) or `location` MUST be provided.

### Location (embedded structure — extensible by type)

| Attribute | Type | Required | Constraints | Description |
|---|---|---|---|---|
| `type` | `Enum` | Yes | `database` \| `parquet` \| `csv` \| `api` \| extensible | Source type discriminator |
| `options` | `Map<String, Any>` | Yes | Keys and values are type-specific (see below) | Connection and access properties for the given type |

**Type-specific `options`:**

| `type` | Required options | Optional options |
|---|---|---|
| `database` | `connection_string`, `table` | `schema` |
| `parquet` | `path` | — |
| `csv` | `path` | `delimiter` (default `,`), `has_header` (default `true`) |
| `api` | `endpoint`, `method` | `auth`, `params`, `headers` |

> `options` is intentionally open — new `type` values and their options can be introduced without changing the schema structure.

### LookupDef (embedded structure)

| Attribute | Type | Required | Constraints | Description |
|---|---|---|---|---|
| `target` | `TableRef \| DatasetRef` | Yes | TableRef requires a Location; DatasetRef references an existing Dataset by id | The lookup target |
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
| `_labels` | `Map<String, String>` | Yes | User-defined key-value tags; any operation may add, update, or remove entries |

## Relationships

| Relationship | Related Entity | Cardinality | Description |
|---|---|---|---|
| has main table | Table | 1:1 | Every Dataset has exactly one designated main table |
| has lookups | Table or Dataset | 1:N | A Dataset has zero or more lookup definitions, each pointing to a Table or nested Dataset |
| sourced from | DataSource | 1:N | Tables in a Dataset each carry an inline Location definition; a Dataset may span multiple source types and physical locations |
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
- **BR-010**: Every TableRef in a Dataset (main table or lookup) MUST include a Location definition with a valid `type` and all required `options` for that type.
- **BR-011**: The `type` field of a Location is the authoritative discriminator; any component consuming a Dataset MUST use it to determine how to access the data. Unknown types MUST be treated as an error.
- **BR-012**: User-defined column names MUST NOT use the `_` prefix — that namespace is reserved exclusively for system-managed metadata columns.
- **BR-013**: Every row produced from a Dataset MUST carry a system-generated UUID v7 `_row_id`. This value is immutable once assigned.
- **BR-014**: If `natural_key_columns` is defined, those columns are preserved as regular data columns alongside `_row_id`. The system does not replace or shadow them.
- **BR-015**: Lineage columns (`_source_dataset_id`, `_source_table`, `_created_by_project_id`, `_created_by_run_id`, `_created_at`) are immutable once set on a row.
- **BR-016**: `_labels` is mutable. Any operation in a Project MAY add, update, or remove entries. Label keys and values are user-defined strings with no system-imposed schema.
- **BR-017**: `_updated_at` MUST be updated by the system whenever any operation modifies a row, including label changes.
- **BR-018**: `_period` is an optional string identifier whose meaning is governed by the Period entity. It MAY be set at import time or inherited from the Run that produced the row. Any operation MAY update `_period` on a row.

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

- This entity does NOT store or own actual data — it is a structural definition only.
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
  created_at: timestamp         # system-set on creation; ISO 8601; immutable
  updated_at: timestamp         # system-set on every change; ISO 8601
  natural_key_columns: [string] # optional; column names from main_table that form the natural key
  main_table:
    name: string                # required; logical name within this Dataset
    location:
      type: database | parquet | csv | api   # required; extensible discriminator
      options:                               # required; keys depend on type
        # database:   connection_string, table, [schema]
        # parquet:    path
        # csv:        path, [delimiter], [has_header]
        # api:        endpoint, method, [auth], [params], [headers]
        <key>: <value>
  lookups:                      # optional; ordered list
    - alias: string             # optional; must be unique within the Dataset
      target:
        type: table | dataset   # required; discriminator
        # when type is `table`:
        name: string            # logical name within this Dataset
        location:
          type: database | parquet | csv | api
          options:
            <key>: <value>
        # when type is `dataset`:
        id: uuid                # references an existing Dataset
      join_conditions:          # required; at least one entry
        - source_column: string # column on the parent (main table or parent lookup) side
          target_column: string # column on this lookup's target side
```

> `created_at`, `updated_at`, and `version` are system-managed and MUST be treated as read-only by any component that consumes this format.
> Adding a new source `type` requires only a new value for `type` and its corresponding `options` keys — no schema structure change needed.

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
_period: string               # optional; period identifier (e.g. "2026-01"); meaning defined by Period entity
_labels:                      # user-defined key-value tags; mutable by any operation
  <key>: <value>
```

```yaml
# Example: Orders (database) joined to customers (database, same server),
#          region codes (CSV file), and a nested product-categories Dataset (cross-source API)
dataset:
  id: "d1a2b3c4-0000-0000-0000-000000000001"
  name: "Sales Orders"
  description: "Orders with customer details, region codes, and product category hierarchy"
  owner: "user-marcos"
  version: 4
  status: active
  created_at: "2026-02-21T10:00:00Z"
  updated_at: "2026-02-21T15:45:00Z"
  natural_key_columns: [order_number]   # user's natural key; preserved alongside _row_id
  main_table:
    name: orders
    location:
      type: database
      options:
        connection_string: "postgresql://host:5432/sales"
        schema: public
        table: orders
  lookups:
    - alias: customers
      target:
        type: table
        name: customers
        location:
          type: database
          options:
            connection_string: "postgresql://host:5432/sales"
            schema: public
            table: customers
      join_conditions:
        - source_column: customer_id
          target_column: id
    - alias: region_codes
      target:
        type: table
        name: region_codes
        location:
          type: csv
          options:
            path: "s3://my-bucket/reference/region_codes.csv"
            has_header: true
            delimiter: ","
      join_conditions:
        - source_column: region_id
          target_column: code
    - alias: product_categories
      target:
        type: dataset                        # nested Dataset — resolved separately
        id: "d5e6f7g8-0000-0000-0000-000000000002"
      join_conditions:
        - source_column: product_id
          target_column: product_id
```

## Related Entities

- [[Project]] — A Project references a Dataset to determine what data its computations operate on; multiple Projects may share the same Dataset.
- [[Table]] — The primitive building block; a Dataset's main table and raw-table lookups are Tables.
- [[DataSource]] — The preferred way to define table connections in a Dataset; a TableRef references a DataSource by ID with only table-specific detail supplied inline. Inline Location remains available for one-off connections.
- [[ComputationEngine]] — Consumes a Dataset as its primary input definition when executing DSL programs.
- [[DSL]] — May request dynamic (runtime) joins on top of the Dataset's pre-defined structure.
- [[Period]] — The `_period` metadata column on each row stores a Period string identifier; the Period entity defines its meaning, boundaries, and rollup hierarchy.
