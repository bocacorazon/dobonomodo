# Entity: Operation

**Status**: Draft  
**Created**: 2026-02-21  
**Domain**: Computation / Project Execution

## Definition

An Operation is a single, typed unit of work within a Project. Each Operation specifies *what* to do (`type`), *which rows* to act on (`selector`), and *how* to do it (`arguments` — a type-specific argument schema). Operations execute in sequence, each receiving the full working dataset produced by the previous step. The working dataset is the in-memory state of all rows; deleted rows are automatically excluded from every operation unless explicitly included.

---

## Attributes

| Attribute | Type | Required | Constraints | Description |
|---|---|---|---|---|
| `id` | `UUID` | Yes | Immutable, system-generated | Unique identifier within the Project |
| `seq` | `Integer` | Yes | Positive integer; unique within Project; determines execution order | Sequence position. Reorderable while Project is `draft`; changing order on an `active` Project triggers a new Project version |
| `name` | `String` | Yes | Non-empty | Human-readable label |
| `description` | `String` | No | — | Optional narrative explanation |
| `type` | `Enum` | Yes | `update \| aggregate \| append \| output \| delete` | Determines which argument schema applies |
| `selector` | `Expression` | No | Boolean Expression string; use `{{SELECTOR_NAME}}` to reference a named selector defined in the enclosing Project's `selectors` map; when omitted, the operation applies to all non-deleted rows | Row filter controlling which rows this operation acts on |
| `arguments` | `Map` | Yes | Schema is fully determined by `type`; see per-type argument tables below | Type-specific configuration |

---

## Operation Types

### `update`

Modifies column values on matching rows. Can update existing columns, add new computed columns, and optionally enrich the available columns by joining external tables at runtime before evaluating assignment expressions.

| Argument | Type | Required | Description |
|---|---|---|---|
| `joins` | `List<RuntimeJoin>` | No | Zero or more runtime joins that make additional columns available to assignment expressions |
| `assignments` | `List<Assignment>` | Yes | One or more column-value pairs to set; each is always an expression |

#### RuntimeJoin (embedded structure)

A `RuntimeJoin` loads a Dataset at operation time and makes its columns available under an `alias` for use in any assignment expression within the same `update`. The join Dataset is resolved via the Resolver using the same precedence as the input Dataset (Project `resolver_overrides` → Dataset `resolver_id` → system default) and period-filtered according to the join Dataset's `temporal_mode`.

| Field | Type | Required | Description |
|---|---|---|---|
| `alias` | `String` | Yes | Logical name used to reference joined columns in expressions (e.g., `alias: customers` → `customers.tier`) |
| `dataset_id` | `UUID` | Yes | References an existing Dataset to join against |
| `dataset_version` | `Integer` | No | When set, pins to this specific Dataset version. When omitted, resolves to the latest active version at Run time |
| `on` | `Expression` | Yes | Boolean join condition referencing working dataset and/or other join aliases |

#### Assignment (embedded structure)

| Field | Type | Required | Description |
|---|---|---|---|
| `column` | `String` | Yes | Target column name (existing or new) |
| `expression` | `Expression` | Yes | Value expression; may reference working dataset columns and any `alias` defined in `joins` |

---

### `aggregate`

Summarises rows (optionally filtered by selector) and **appends** the resulting summary rows to the working dataset. Existing rows are not removed.

| Argument | Type | Required | Description |
|---|---|---|---|
| `group_by` | `List<String>` | Yes | Column references to group by (logical_table.column format) |
| `aggregations` | `List<Aggregation>` | Yes | One or more aggregate computations |

#### Aggregation (embedded structure)

| Field | Type | Required | Description |
|---|---|---|---|
| `column` | `String` | Yes | Name of the output column on the appended summary rows |
| `expression` | `Expression` | Yes | Aggregate expression (must use an aggregate function: `SUM`, `COUNT`, `AVG`, `MIN_AGG`, `MAX_AGG`) |

---

### `append`

Brings rows from another Dataset into the working dataset. Optionally aggregates the incoming rows before appending.

| Argument | Type | Required | Description |
|---|---|---|---|
| `source` | `DatasetRef` | Yes | The Dataset whose rows to append |
| `source_selector` | `Expression` | No | Boolean filter applied to the source Dataset rows before appending |
| `aggregation` | `AppendAggregation` | No | If present, aggregate source rows before appending instead of appending raw |

#### AppendAggregation (embedded structure)

| Field | Type | Required | Description |
|---|---|---|---|
| `group_by` | `List<String>` | Yes | Columns from the source Dataset to group by |
| `aggregations` | `List<Aggregation>` | Yes | Aggregate computations; same structure as in the `aggregate` type |

> Schema rule: every column present on appended rows MUST exist in the main working dataset. Columns present in the working dataset but absent from appended rows are set to `NULL` on those rows.

---

### `delete`

Soft-deletes rows matching the selector by setting the system column `_deleted = true`. Deleted rows are excluded from all subsequent operations by default.

| Argument | Type | Required | Description |
|---|---|---|---|
| *(none)* | — | — | No additional arguments. The selector drives which rows are deleted. |

---

### `output`

Writes a projection of the working dataset to an external destination. This is the **only** operation type permitted to perform IO. It may appear anywhere in the pipeline (including mid-pipeline checkpointing). The projection is the intersection of the selector (row filter) and `columns` (column filter); by default, deleted rows are excluded.

Optionally, the output can be **registered as a named Dataset** in the system, making it available as the input Dataset for another Project.

| Argument | Type | Required | Description |
|---|---|---|---|
| `destination` | `TableRef` | Yes | Target location to write to (DataSource reference or inline Location) |
| `columns` | `List<String>` | No | Subset of columns to include in the output. When omitted, all columns are written |
| `include_deleted` | `Boolean` | No | Default `false`. When `true`, rows with `_deleted = true` are included in the output |
| `register_as_dataset` | `String` | No | When provided, registers the output as a new (or new version of an existing) Dataset with this name, making it available as input to other Projects |

---

## Project-Level Named Selectors

Named selectors are defined in the enclosing **Project** under a `selectors` map (`name → boolean Expression`). They are referenced inside any Operation `selector` (or `source_selector` in `append`) using the `{{NAME}}` interpolation syntax:

```
"{{active_orders}}"
```

The engine substitutes the named selector's expression before evaluation. Named selectors are captured verbatim in the ProjectSnapshot at Run creation, ensuring reproducibility.

---

## Behaviors / Rules

| ID | Rule |
|---|---|
| BR-001 | `seq` values MUST be unique and positive within a Project. Gaps are allowed. |
| BR-002 | `seq` MAY be reordered freely while the Project is in `draft` status. |
| BR-003 | Reordering operations on an `active` Project MUST trigger creation of a new Project version. |
| BR-004 | Rows with `_deleted = true` are automatically excluded from the working dataset seen by all operations, unless the operation explicitly opts in (only `output` supports `include_deleted`). |
| BR-005 | When `selector` is omitted, the operation applies to all non-deleted rows. |
| BR-006 | A named selector referenced as `{{NAME}}` MUST be defined in the enclosing Project's `selectors` map. Referencing an undefined name is a compile-time error. |
| BR-007 | Each `update` assignment MUST have an `expression`. Column references in that expression MAY include aliases defined in the same operation's `joins` list. |
| BR-008 | `update` joins are operation-scoped — their aliases are not visible to any other operation in the pipeline. |
| BR-008a | A RuntimeJoin's `dataset_id` MUST reference an existing, active Dataset. The Resolver used to load the join Dataset follows the same precedence as the input Dataset: Project `resolver_overrides` → Dataset `resolver_id` → system default. |
| BR-008b | A RuntimeJoin Dataset is period-filtered according to its own `temporal_mode`: `period` tables use `_period = run_period.identifier`; `bitemporal` tables use the asOf query. The Run's current Period is always used — there is no per-join Period override. |
| BR-008c | When `dataset_version` is omitted on a RuntimeJoin, the engine resolves to the latest active version of the Dataset at Run time. When set, the pinned version is used. The resolved version is captured in the Run's ProjectSnapshot `resolver_snapshots`. |
| BR-009 | `aggregate` appends new rows; it does NOT replace or remove existing rows in the working dataset. |
| BR-009 | In `append`, every column on the incoming rows MUST match an existing column in the working dataset. Additional columns on the working dataset that are absent from incoming rows are set to `NULL`. |
| BR-010 | In `append`, every column on the incoming rows MUST match an existing column in the working dataset. Additional columns on the working dataset that are absent from incoming rows are set to `NULL`. |
| BR-011 | `output` is the only operation type permitted to perform IO. All other types operate only in-memory on the working dataset. |
| BR-012 | `output` MAY appear at any position in the pipeline, including mid-pipeline, to support checkpointing. |
| BR-013 | `output` with `include_deleted: false` (the default) MUST NOT write rows where `_deleted = true`, regardless of the selector. |
| BR-014 | Aggregate functions in expressions are valid only inside `aggregate` and `append` (with aggregation) operations. Using an aggregate expression in `update`, `delete`, `output`, or a selector is a compile-time error. |
| BR-015 | All Expression column references in an Operation MUST resolve to columns in the working dataset or to a `join` alias defined in the same operation. Unknown references are a compile-time error. |

---

## Lifecycle

Operations have no independent lifecycle. They are created, modified, and deleted as part of their enclosing Project. When a Project is snapshotted into a Run's `ProjectSnapshot`, the full ordered list of Operations (including all `selector` and `arguments` values) is captured immutably.

---

## Relationships

| Entity | Relationship |
|---|---|
| Project | An Operation belongs to exactly one Project; a Project contains an ordered list of Operations |
| Expression | Selectors and argument values are inline Expressions |
| Dataset | Column references in Expressions resolve against the Project's input Dataset; RuntimeJoin `dataset_id` references Datasets loaded at operation time |
| Run | At execution, Operations are compiled from the immutable ProjectSnapshot |
| Resolver | RuntimeJoin Datasets are loaded via the Resolver using the same precedence as the input Dataset |

---

## Boundaries (What This Is Not)

- An Operation is **not** a DAG node — operations execute strictly in `seq` order with no branching or parallel paths.
- An Operation is **not** reusable across Projects — it is always scoped to a single Project.
- `delete` is **not** a hard delete — the row is retained in the dataset with `_deleted = true`.
- `aggregate` does **not** replace rows — it appends summary rows alongside the existing rows.

---

## Open Questions

| # | Question | Status |
|---|---|---|
| OQ-001 | Should `output` support writing to multiple destinations simultaneously, or always exactly one? | Deferred |
| OQ-002 | Can a `RuntimeJoin` in an `update` operation join against the same working dataset (self-join)? | Deferred |
| OQ-003 | How are `_row_id` and lineage columns populated on rows appended by `append` or `aggregate`? | Deferred |

---

## Serialization (YAML DSL)

### Schema

```yaml
operation:
  id: uuid
  seq: integer                  # execution order; unique within project
  name: string
  description: string           # optional
  type: update | aggregate | append | output | delete
  selector: <expression>        # optional; boolean expression string; use {{NAME}} to reference a named selector
  arguments: <type-specific>    # see per-type schemas below
```

### Per-type Argument Schemas

```yaml
# --- update ---
arguments:
  joins:                        # optional; defines columns available to assignment expressions
    - alias: string             # logical name used in expressions (e.g., "customers")
      dataset_id: uuid          # references an existing Dataset
      dataset_version: integer  # optional; pin to specific version; omit for latest at Run time
      on: <expression>          # boolean join condition
  assignments:
    - column: string
      expression: <expression>  # may reference working dataset cols and any join alias

# --- aggregate ---
arguments:
  group_by:
    - "logical_table.column"
  aggregations:
    - column: string
      expression: <expression>  # must use aggregate function

# --- append ---
arguments:
  source:
    dataset_id: uuid
    dataset_version: integer    # optional; pin to specific version
  source_selector: <expression> # optional; filter on source rows
  aggregation:                  # optional; aggregate before appending
    group_by:
      - "logical_table.column"
    aggregations:
      - column: string
        expression: <expression>

# --- delete ---
arguments: {}                   # no arguments; selector drives deletion

# --- output ---
arguments:
  destination: <TableRef>       # DataSource ref or inline Location
  columns:                      # optional; subset of columns to write
    - string
  include_deleted: boolean      # default false
  register_as_dataset: string   # optional; registers output as a named Dataset
```

### Annotated Example

```yaml
# Project-level named selectors (defined in Project.selectors, referenced via {{NAME}})
# selectors:
#   active_orders: "orders.status = \"active\""

# Operation 1: tag rows by region
- id: "op-001"
  seq: 1
  name: "Tag EMEA orders"
  type: update
  selector: "orders.region = \"EMEA\""
  arguments:
    assignments:
      - column: "_labels"
        expression: "CONCAT(_labels, \"{region:EMEA}\")"

# Operation 2: enrich with customer tier and compute a discounted price from a runtime join
- id: "op-002"
  seq: 2
  name: "Add customer tier and discounted price"
  type: update
  selector: "{{active_orders}}"
  arguments:
    joins:
      - alias: "customers"
        dataset_id: "d5e6f7g8-0000-0000-0000-000000000010"
        on: "orders.customer_id = customers.id"
    assignments:
      - column: "customer_tier"
        expression: "customers.tier"
      - column: "discounted_price"
        expression: "IF(customers.tier = \"gold\", orders.amount * 0.9, orders.amount)"

# Operation 3: aggregate monthly totals and append as summary rows
- id: "op-003"
  seq: 3
  name: "Monthly totals"
  type: aggregate
  arguments:
    group_by:
      - "orders._period"
      - "orders.region"
    aggregations:
      - column: "total_sales"
        expression: "SUM(orders.amount)"
      - column: "order_count"
        expression: "COUNT(orders.order_id)"

# Operation 4: soft-delete rows with zero amount
- id: "op-004"
  seq: 4
  name: "Remove zero-amount rows"
  type: delete
  selector: "orders.amount = 0"

# Operation 5: write output (active rows only, no deleted)
- id: "op-005"
  seq: 5
  name: "Write to warehouse"
  type: output
  selector: "{{active_orders}}"
  arguments:
    destination:
      datasource_id: "ds-warehouse"
      table: "processed_orders"
    include_deleted: false
```
