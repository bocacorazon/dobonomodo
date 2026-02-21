# Entity: Period

**Status**: Draft  
**Created**: 2026-02-21  
**Domain**: Temporal / Calendar

## Definition

A Period is a named, bounded time interval that defines the temporal scope of a unit of work. It has an explicit start and end date, a calendar position (year and sequence number), and belongs to a Calendar. Periods are arranged in a user-defined, arbitrary-depth rollup hierarchy — leaf Periods (e.g., fiscal months) roll up into parent Periods (e.g., fiscal quarters), which in turn roll up into higher-level parents (e.g., fiscal years). A Period governs which Runs may target it and what modifications to its data are permitted, via a three-state lifecycle.

## Purpose & Role

A Period is the temporal anchor of every Run. Without it, the system has no way to scope a unit of computation to a time interval, track which data belongs to which point in time, or aggregate results across time. It enables period-based filtering on Dataset rows (via the `_period` metadata column), controls the validity window for Runs, and provides the rollup structure for cross-period aggregation.

## Attributes

| Attribute | Type | Required | Constraints | Description |
|---|---|---|---|---|
| `id` | `UUID` | Yes | Immutable, system-generated | Unique identifier |
| `identifier` | `String` | Yes | Non-empty; unique within a Calendar | The string key used in the `_period` row metadata column (e.g., `"202601"`, `"FY2026Q1"`) |
| `name` | `String` | Yes | Non-empty | Human-readable label (e.g., `"January 2026"`, `"FY2026 Q1"`) |
| `description` | `String` | No | — | Optional narrative description |
| `calendar_id` | `UUID` | Yes | Must reference a valid Calendar | The Calendar this Period belongs to |
| `year` | `Integer` | Yes | Calendar year this Period falls within | Calendar position — year component |
| `sequence` | `Integer` | Yes | Unique within the same parent and year | Calendar position — sequence number within the level (e.g., month 1, quarter 2) |
| `start_date` | `Date` | Yes | ISO 8601; must be ≤ `end_date` | First day of the Period (inclusive) |
| `end_date` | `Date` | Yes | ISO 8601; must be ≥ `start_date` | Last day of the Period (inclusive) |
| `status` | `Enum` | Yes | `open` \| `closed` \| `locked` | Controls Run targeting and data mutability |
| `parent_id` | `UUID` | No | Must reference a valid Period in the same Calendar | The rollup parent this Period aggregates into; absent for top-level Periods |
| `created_at` | `Timestamp` | Yes | System-set on creation; immutable | Creation time |
| `updated_at` | `Timestamp` | Yes | System-set on every change | Last modification time |

## Relationships

| Relationship | Related Entity | Cardinality | Description |
|---|---|---|---|
| belongs to | Calendar | N:1 | Every Period belongs to exactly one Calendar |
| rolls up into | Period | N:1 | A Period MAY have one parent Period it aggregates into; top-level Periods have none |
| contains | Period | 1:N | A Period MAY have zero or more child Periods that roll up into it |
| targeted by | Run | N:M | A Run may target one or more Periods; a Period may be targeted by many Runs |
| tags rows in | Dataset (via `_period`) | 1:N | Dataset rows carry the Period's `identifier` string in the `_period` metadata column |

## Behaviors & Rules

- **BR-001**: A Period's `identifier` MUST be unique within its Calendar.
- **BR-002**: A Period's `start_date` MUST be earlier than or equal to its `end_date`.
- **BR-003**: Child Periods' date ranges MUST fall within their parent Period's date range.
- **BR-004**: An `open` Period MAY be targeted by new Runs and its data MAY be modified.
- **BR-005**: A `closed` Period MUST NOT be targeted by new Runs. Existing data for the Period MAY still be corrected or modified by authorised operations.
- **BR-006**: A `locked` Period is fully immutable. No new Runs may target it and no modifications to its data are permitted.
- **BR-007**: Status transitions are strictly one-directional: `open` → `closed` → `locked`. A Period MUST NOT be re-opened or un-locked.
- **BR-008**: Locking a Period MUST NOT automatically lock its child or parent Periods — each Period's status is managed independently.
- **BR-009**: The `_period` metadata column on a Dataset row stores the `identifier` string of the Period. The Period entity is the authoritative source for that identifier's meaning, dates, and rollup position.
- **BR-010**: Periods are generated either manually (one at a time) or automatically from a Calendar's generation rules. Both paths produce identical Period entities.

## Lifecycle

| State | Description | Transitions To |
|---|---|---|
| `open` | Accepting new Runs; data is freely modifiable | `closed` |
| `closed` | No new Runs may target this Period; existing data may still be corrected | `locked` |
| `locked` | Fully immutable; no Runs, no data modifications | — (terminal) |

**What creates a Period**: A user creates it manually, or the system generates it automatically from a Calendar rule.  
**What modifies a Period**: Status transitions (`open` → `closed` → `locked`), or corrections to `name`, `description`, `start_date`, `end_date` while still `open`.  
**What destroys a Period**: Open question — see below.

## Boundaries

- A Period does NOT define what computations are performed — that is the Project and DSL's concern.
- A Period does NOT own or store data — it provides a temporal label that rows and Runs reference.
- A Period does NOT enforce date ranges on the data it tags — the `_period` string is a label; alignment to actual row dates is the responsibility of the Project operations that assign it.
- A Period does NOT represent a schedule or trigger — when Runs execute is handled by a future scheduling entity.

## Open Questions

- [ ] Can a rollup Period (e.g., `FY2026Q1`) be targeted directly by a Run for independent computation over aggregated child data, or can rollup Periods only hold data derived from their children?
- [ ] What are the deletion rules for a Period? Can it be deleted if no Runs or Dataset rows reference it?
- [ ] Should locking a Period cascade to its children, or is that always a separate explicit action?

## Serialization (YAML DSL)

Schema for serializing a Period for inter-component communication.

```yaml
# period.schema.yaml
period:
  id: uuid                      # system-generated, immutable
  identifier: string            # required; unique within Calendar; used in _period row metadata
  name: string                  # required, non-empty
  description: string           # optional
  calendar_id: uuid             # required; references a Calendar
  year: integer                 # required; calendar year
  sequence: integer             # required; position within level and year
  start_date: date              # required; ISO 8601; inclusive
  end_date: date                # required; ISO 8601; inclusive
  status: open | closed | locked  # required
  parent_id: uuid               # optional; references rollup parent Period in same Calendar
  created_at: timestamp         # system-set on creation; ISO 8601; immutable
  updated_at: timestamp         # system-set on every change; ISO 8601
```

> `created_at`, `updated_at` are system-managed and MUST be treated as read-only.  
> Status transitions are one-directional (`open` → `closed` → `locked`) — any component that updates `status` MUST enforce this constraint.

```yaml
# Example: Fiscal month January 2026, rolling up into FY2026Q1
period:
  id: "per-0000-0000-0000-000000000001"
  identifier: "202601"
  name: "January 2026"
  description: "First fiscal month of FY2026"
  calendar_id: "cal-0000-0000-0000-000000000001"
  year: 2026
  sequence: 1
  start_date: "2026-01-01"
  end_date: "2026-01-31"
  status: closed
  parent_id: "per-0000-0000-0000-000000000010"  # FY2026Q1
  created_at: "2025-11-01T00:00:00Z"
  updated_at: "2026-02-01T08:00:00Z"

---

# Example: Fiscal quarter FY2026Q1 — rollup of 202601, 202602, 202603
period:
  id: "per-0000-0000-0000-000000000010"
  identifier: "FY2026Q1"
  name: "FY2026 Quarter 1"
  calendar_id: "cal-0000-0000-0000-000000000001"
  year: 2026
  sequence: 1
  start_date: "2026-01-01"
  end_date: "2026-03-31"
  status: open
  parent_id: "per-0000-0000-0000-000000000100"  # FY2026
  created_at: "2025-11-01T00:00:00Z"
  updated_at: "2025-11-01T00:00:00Z"
```

## Related Entities

- [[Calendar]] — A Period belongs to exactly one Calendar, which defines the standard rollup rules and generation logic.
- [[Run]] — A Run is tied to one or more Periods; a Period's status controls whether it can be targeted by new Runs.
- [[Dataset]] — Dataset rows carry a Period's `identifier` in the `_period` metadata column; the Period entity is the authoritative source for that identifier's meaning.
