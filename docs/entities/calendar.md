# Entity: Calendar

**Status**: Draft  
**Created**: 2026-02-21  
**Domain**: Temporal / Calendar

## Definition

A Calendar is a named container that organises Periods into a rollup hierarchy and optionally defines the rules for generating and naming those Periods automatically. A deployment may have multiple Calendars (e.g., Gregorian and Fiscal) that coexist and relate to each other via CalendarMapping entities. Exactly one Calendar per deployment is designated as the default. A Calendar may define its hierarchy levels explicitly via level definitions (identifier patterns and optional date boundary rules), or rely on the parent/child relationships between its Periods to imply structure.

## Purpose & Role

A Calendar provides the temporal framework within which Periods are organised and generated. Without it, Periods have no shared context, rollup hierarchy cannot be validated, and cross-calendar conversions have no anchor point. It is the authoritative source for how time is divided and labelled in the system, and it drives the auto-generation of Periods to ensure consistency and completeness.

## Attributes

| Attribute | Type | Required | Constraints | Description |
|---|---|---|---|---|
| `id` | `UUID` | Yes | Immutable, system-generated | Unique identifier |
| `name` | `String` | Yes | Non-empty; unique within deployment | Human-readable name (e.g., `"Gregorian"`, `"Fiscal 2026"`) |
| `description` | `String` | No | — | Optional narrative description |
| `status` | `Enum` | Yes | `draft` \| `active` \| `deprecated` | Controls availability for use |
| `is_default` | `Boolean` | Yes | Exactly one Calendar per deployment may be `true` | Marks the mutable system default (Gregorian); the default may be edited directly |
| `levels` | `List<LevelDef>` | No | If defined, must form a valid hierarchy (no cycles, unique names); if absent, structure is inferred from Period parent/child relationships | Explicit hierarchy level definitions with optional generation rules |
| `created_at` | `Timestamp` | Yes | System-set on creation; immutable | Creation time |
| `updated_at` | `Timestamp` | Yes | System-set on every change | Last modification time |

### LevelDef (embedded structure)

| Attribute | Type | Required | Constraints | Description |
|---|---|---|---|---|
| `name` | `String` | Yes | Unique within the Calendar | Level name (e.g., `"month"`, `"quarter"`, `"year"`) |
| `parent_level` | `String` | No | Must reference another `name` in this Calendar's levels | The level this level rolls up into; absent for the top level |
| `identifier_pattern` | `String` | No | May include tokens: `{year}`, `{sequence}`, `{parent_identifier}` | Template for generating Period identifiers at this level (e.g., `"FY{year}Q{sequence}"`) |
| `date_rules` | `List<DateRule>` | No | Sequences must be unique within the level | Explicit date boundaries per sequence position; if absent, dates must be set manually on each Period |

### DateRule (embedded structure)

| Attribute | Type | Required | Constraints | Description |
|---|---|---|---|---|
| `sequence` | `Integer` | Yes | Must match a valid sequence number for the level | The sequence number this rule applies to (e.g., `1` for Q1) |
| `start_month` | `Integer` | Yes | 1–12 | Start month of the interval |
| `start_day` | `Integer` | Yes | 1–31; valid for the given month | Start day of the interval |
| `end_month` | `Integer` | Yes | 1–12 | End month of the interval |
| `end_day` | `Integer` | Yes | 1–31; valid for the given month | End day of the interval |

## Relationships

| Relationship | Related Entity | Cardinality | Description |
|---|---|---|---|
| contains | Period | 1:N | A Calendar owns all Periods that belong to it |
| mapped to/from | Calendar (via CalendarMapping) | N:M | A Calendar may define conversion rules to/from other Calendars via separate CalendarMapping entities |
| used by | Run | 1:N | Runs reference Periods which belong to a Calendar; the Calendar provides temporal context |

## Behaviors & Rules

- **BR-001**: A deployment MUST have exactly one Calendar with `is_default: true`.
- **BR-002**: A `draft` Calendar MAY be edited freely but its Periods MUST NOT be used in new Runs until the Calendar is `active`.
- **BR-003**: A `deprecated` Calendar MUST NOT have new Periods added to it. Existing Periods remain valid for historical reference.
- **BR-004**: Status transitions are one-directional: `draft` → `active` → `deprecated`. A deprecated Calendar MUST NOT be reactivated.
- **BR-005**: If `levels` are defined, all Periods in the Calendar MUST have a `parent_id` consistent with the declared level hierarchy (except top-level Periods).
- **BR-006**: If `levels` are absent, the rollup structure is inferred entirely from Period `parent_id` relationships — no validation against a declared hierarchy is performed.
- **BR-007**: When `identifier_pattern` is defined for a level, auto-generated Periods MUST use it. Manually created Periods in that Calendar SHOULD follow the pattern but are not required to.
- **BR-008**: When `date_rules` are defined for a level, auto-generated Periods MUST use them. Date boundaries may be overridden manually after generation.
- **BR-009**: Cross-calendar conversions MUST be handled by CalendarMapping entities — the Calendar entity itself does not encode conversion logic.
- **BR-010**: Renaming a Calendar's `levels[].name` MUST NOT retroactively change any existing Period's structure — levels are advisory for generation and validation, not stored on Periods.

## Lifecycle

| State | Description | Transitions To |
|---|---|---|
| `draft` | Being assembled; Periods may be added but not used in Runs | `active` |
| `active` | Fully operational; Periods may be created, used in Runs, and mapped to other Calendars | `deprecated` |
| `deprecated` | No new Periods may be added; existing Periods remain valid for historical access | — (terminal) |

**What creates a Calendar**: A user creates it explicitly, or the system seeds the default Gregorian Calendar on deployment initialisation.  
**What modifies a Calendar**: Changes to name, description, status, `is_default`, or level definitions.  
**What destroys a Calendar**: Open question — see below.

## Boundaries

- A Calendar does NOT store data — it organises Periods.
- A Calendar does NOT define conversion rules to other Calendars — that is the responsibility of **CalendarMapping**.
- A Calendar does NOT enforce date continuity across its Periods (no gap/overlap validation) — that is an open question for the planning phase.
- A Calendar does NOT control Run execution — it provides the temporal framework Runs reference via Periods.

## Open Questions

- [ ] Should a Calendar enforce that its Periods have no date gaps or overlaps at the same hierarchy level?
- [ ] What are the deletion rules for a Calendar? Can it be deleted if Periods in it are referenced by Dataset rows or Runs?
- [ ] Should deprecating a Calendar cascade to close/lock all its open Periods, or is that a separate explicit action?

## Serialization (YAML DSL)

Schema for serializing a Calendar for inter-component communication.

```yaml
# calendar.schema.yaml
calendar:
  id: uuid                      # system-generated, immutable
  name: string                  # required; unique within deployment
  description: string           # optional
  status: draft | active | deprecated  # required
  is_default: boolean           # required; exactly one Calendar per deployment may be true
  created_at: timestamp         # system-set on creation; ISO 8601; immutable
  updated_at: timestamp         # system-set on every change; ISO 8601
  levels:                       # optional; if absent, hierarchy inferred from Period parent/child
    - name: string              # required; unique within Calendar (e.g., "year", "quarter", "month")
      parent_level: string      # optional; name of the parent level; absent for top level
      identifier_pattern: string  # optional; tokens: {year}, {sequence}, {parent_identifier}
      date_rules:               # optional; if absent, dates set manually per Period
        - sequence: integer     # required; sequence number this rule applies to
          start_month: integer  # required; 1–12
          start_day: integer    # required; 1–31
          end_month: integer    # required; 1–12
          end_day: integer      # required; 1–31
```

> `created_at` and `updated_at` are system-managed and MUST be treated as read-only.

```yaml
# Example: Fiscal calendar with 3 levels — year, quarter, month
# Q1 = Jan–Mar, Q2 = Apr–Jun, Q3 = Jul–Sep, Q4 = Oct–Dec
calendar:
  id: "cal-0000-0000-0000-000000000001"
  name: "Fiscal Calendar"
  description: "Standard fiscal calendar aligned to Gregorian year"
  status: active
  is_default: false
  created_at: "2025-11-01T00:00:00Z"
  updated_at: "2025-11-01T00:00:00Z"
  levels:
    - name: year
      identifier_pattern: "FY{year}"
    - name: quarter
      parent_level: year
      identifier_pattern: "FY{year}Q{sequence}"
      date_rules:
        - sequence: 1
          start_month: 1
          start_day: 1
          end_month: 3
          end_day: 31
        - sequence: 2
          start_month: 4
          start_day: 1
          end_month: 6
          end_day: 30
        - sequence: 3
          start_month: 7
          start_day: 1
          end_month: 9
          end_day: 30
        - sequence: 4
          start_month: 10
          start_day: 1
          end_month: 12
          end_day: 31
    - name: month
      parent_level: quarter
      identifier_pattern: "{year}{sequence:02d}"
      date_rules:
        - sequence: 1
          start_month: 1
          start_day: 1
          end_month: 1
          end_day: 31
        - sequence: 2
          start_month: 2
          start_day: 1
          end_month: 2
          end_day: 28
        - sequence: 3
          start_month: 3
          start_day: 1
          end_month: 3
          end_day: 31
        # ... remaining months follow same pattern
```

## Related Entities

- [[Period]] — A Calendar owns and organises its Periods; level definitions drive Period generation and hierarchy validation.
- [[CalendarMapping]] — A separate entity that encodes conversion rules between two Calendars (e.g., Gregorian month → Fiscal period).
- [[Run]] — Runs reference Periods, which belong to a Calendar; the Calendar provides the temporal framework for all Run scoping.
