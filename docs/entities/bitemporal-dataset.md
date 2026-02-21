# Entity: Bitemporal Dataset

**Status**: Draft  
**Created**: 2026-02-21  
**Domain**: Data Layer / Temporality

## Definition

A Bitemporal Dataset is a Dataset in which one or more tables are flagged as bitemporal — meaning each row in those tables is tracked along two explicit time axes simultaneously: the **period axis** (which Period range the row is effective for) and the **valid axis** (when the real-world fact the row represents was actually true). Bitemporality is a table-level capability, not a Dataset-level switch; individual tables within a Dataset independently opt in. Non-bitemporal tables in the same Dataset continue to use the standard `_period` metadata column.

## Purpose & Role

Standard Datasets track data as a snapshot in time. Bitemporal tables are needed whenever the system must distinguish between *which accounting period a value belongs to* and *when that value was actually true in the real world* — for example, a salary record effective for period `202601` that was only entered (or corrected) on Feb 5. Without bitemporality, retroactive corrections silently overwrite history, making it impossible to reconstruct what the system "knew" at a specific point in time or to audit changes to effective values.

## Attributes

Bitemporal tables inherit all standard Dataset and TableRef attributes. The following additions apply:

### TableRef extension

| Attribute | Type | Required | Constraints | Description |
|---|---|---|---|---|
| `bitemporal` | `Boolean` | No | Default `false` | When `true`, enables both time axes on this table and replaces `_period` with `_period_from` / `_period_to` |

### Row Metadata — Bitemporal Axes (replaces `_period` for bitemporal tables)

| Column | Type | Mutable | Nullable | Description |
|---|---|---|---|---|
| `_period_from` | `String` | No | No | Period identifier string (e.g., `"202601"`) — the first Period this row is effective for |
| `_period_to` | `String` | No | Yes | Period identifier string — the last Period this row is effective for; `NULL` = open-ended |
| `_valid_from` | `Date` | No | No | The date from which the real-world fact this row represents became true |
| `_valid_to` | `Date` | No | Yes | The date on which the real-world fact ceased to be true; `NULL` = currently valid |

> For bitemporal tables: `_period` is absent; `_period_from` and `_period_to` take its place.  
> `_created_at` and `_updated_at` continue to serve as system time (when the record was stored/last changed).  
> All other standard metadata columns (`_row_id`, `_source_dataset_id`, `_labels`, etc.) remain unchanged.

## Relationships

| Relationship | Related Entity | Cardinality | Description |
|---|---|---|---|
| extends | Dataset | N:1 | A Bitemporal Dataset is a Dataset with one or more bitemporal-flagged tables |
| period axis references | Period | N:M | `_period_from` and `_period_to` reference Period identifiers within a Calendar |

## Behaviors & Rules

- **BR-001**: When `bitemporal: true` is set on a TableRef, `_period` MUST NOT be present on rows from that table. `_period_from` and `_period_to` MUST be present instead.
- **BR-002**: `_valid_to = NULL` means the row represents a currently valid real-world fact. `_valid_to` MUST be set when the fact is superseded or corrected.
- **BR-003**: `_period_to = NULL` means the row is effective through an open-ended future period.
- **BR-004**: `_valid_from` MUST be earlier than or equal to `_valid_to` when `_valid_to` is not `NULL`.
- **BR-005**: `_period_from` MUST reference a Period that is earlier than or equal to the Period referenced by `_period_to` (when `_period_to` is not `NULL`), according to the Calendar's ordering.
- **BR-006**: To update a bitemporal row (correction or supersession), the engine MUST: (1) set `_valid_to` on the existing row to the day before the correction takes effect; (2) insert a new row with the corrected values and `_valid_from = correction date`, `_valid_to = NULL`. The original row is preserved as historical record.
- **BR-007**: The "current view" of a bitemporal table is defined as `WHERE _valid_to IS NULL`. Period-scoped queries additionally filter on `_period_from`/`_period_to`.
- **BR-008**: Dynamic joins involving a bitemporal table MUST account for the valid axis — join conditions must include valid-time alignment (e.g., only join rows where the valid time ranges overlap with the query context).
- **BR-009**: A non-bitemporal table in the same Dataset MUST NOT use `_period_from` or `_period_to` — it continues to use `_period` as a single string identifier.
- **BR-010**: Locking a Period (via the Period entity) for a bitemporal table MUST prevent any new rows with `_period_from` or `_period_to` referencing that Period from being inserted, and MUST prevent existing rows for that Period from being corrected.

## Lifecycle

Bitemporal tables follow the same Dataset lifecycle (`active` / `disabled`). Individual rows have their own implicit temporal lifecycle governed by `_valid_from`/`_valid_to`:

| Row State | Condition | Meaning |
|---|---|---|
| Current | `_valid_to IS NULL` | Represents the currently accepted real-world fact |
| Superseded | `_valid_to IS NOT NULL` | Was true for a time but has since been corrected or replaced |

## Boundaries

- Bitemporality does NOT replace the Period entity — `_period_from`/`_period_to` store Period *identifiers*; the Period entity governs their meaning, ordering, and status.
- Bitemporality does NOT track who made a change — that is an audit/user-action concern outside this entity's scope.
- The valid axis does NOT imply scheduling — `_valid_from`/`_valid_to` describe real-world fact validity, not when computations should run.
- System time (when data was stored) is handled by `_created_at`/`_updated_at` — no additional `_system_from`/`_system_to` columns are required.

## Open Questions

- [ ] Should the engine enforce that no two rows for the same logical entity (identified by `natural_key_columns`) have overlapping `_valid_from`/`_valid_to` ranges within the same period range?
- [ ] How should the DSL express "give me the valid state of this table as of date D for periods P1–P3"? This needs an explicit temporal query syntax.
- [ ] When a bitemporal table is used as a lookup in a join, should the join automatically filter to `_valid_to IS NULL` (current only), or should the caller specify the valid-time context?

## Serialization (YAML DSL)

Bitemporality is declared on a per-table basis within the standard Dataset YAML. No separate top-level document type is needed.

```yaml
# Enabling bitemporality on a table in a Dataset definition
main_table:
  name: salaries
  bitemporal: true                  # enables _period_from, _period_to, _valid_from, _valid_to
  datasource_id: "ds-0000-0000-0000-000000000001"
  table: salaries
```

The following metadata columns are automatically present on every row of a **bitemporal** table:

```yaml
# Row metadata for a bitemporal table (replaces _period; others unchanged)
_row_id: uuid-v7                    # system-generated, immutable
_period_from: string                # e.g. "202601" — first effective Period
_period_to: string | null           # e.g. "202603" — last effective Period; null = open-ended
_valid_from: date                   # ISO 8601 date — when real-world fact became true
_valid_to: date | null              # ISO 8601 date — when fact ceased to be true; null = current
_source_dataset_id: uuid
_source_table: string
_created_by_project_id: uuid
_created_by_run_id: uuid
_created_at: timestamp
_updated_at: timestamp
_labels:
  <key>: <value>
```

```yaml
# Example: Salary record effective for 202601–202603, currently valid
_row_id: "019500f0-0000-7000-0000-000000000001"
_period_from: "202601"
_period_to: "202603"
_valid_from: "2026-01-01"
_valid_to: null                     # currently valid
_source_dataset_id: "d1a2b3c4-0000-0000-0000-000000000001"
_source_table: salaries
_created_by_project_id: "p1a2b3c4-0000-0000-0000-000000000001"
_created_by_run_id: "run-0000-0000-0000-000000000001"
_created_at: "2026-01-05T09:00:00Z"
_updated_at: "2026-01-05T09:00:00Z"
_labels: {}

---

# Same logical row after correction — original superseded on Feb 10
# Original row (now closed):
_row_id: "019500f0-0000-7000-0000-000000000001"
_period_from: "202601"
_period_to: "202603"
_valid_from: "2026-01-01"
_valid_to: "2026-02-09"             # closed when correction was applied
_updated_at: "2026-02-10T08:00:00Z"

# Corrected row (new, currently valid):
_row_id: "019500f0-0000-7000-0000-000000000002"
_period_from: "202601"
_period_to: "202603"
_valid_from: "2026-02-10"           # correction effective from this date
_valid_to: null                     # currently valid
_created_at: "2026-02-10T08:00:00Z"
_updated_at: "2026-02-10T08:00:00Z"
```

## Related Entities

- [[Dataset]] — Bitemporal Dataset is a Dataset with one or more tables flagged `bitemporal: true`; all standard Dataset rules apply.
- [[Period]] — `_period_from` and `_period_to` store Period identifier strings; the Period entity governs their ordering and status.
- [[Run]] — Runs that write to bitemporal tables must follow the close-and-insert correction pattern; the `_created_by_run_id` on each row identifies which Run produced it.
