# Entity: CalendarMapping

**Status**: Draft  
**Created**: 2026-02-21  
**Domain**: Temporal / Calendar

## Definition

A CalendarMapping is a versioned, directional set of 1:1 Period pairings from a source Calendar to a target Calendar. Each pairing maps exactly one Period in the source Calendar to exactly one Period in the target Calendar. Because mappings are versioned, any computation that uses cross-calendar conversion can reference the exact mapping version in effect at execution time, ensuring rollup reproducibility over time.

## Purpose & Role

A CalendarMapping enables the system to translate temporal scope across different calendar systems. Without it, a Run scoped to Gregorian periods cannot be aggregated into Fiscal periods (or vice versa), and cross-calendar rollups are impossible. Its versioning model ensures that if mapping definitions change (e.g., a fiscal calendar is restated), historical Runs remain reproducible against the mapping version that was in effect when they ran.

## Attributes

| Attribute | Type | Required | Constraints | Description |
|---|---|---|---|---|
| `id` | `UUID` | Yes | Immutable, system-generated | Unique identifier |
| `name` | `String` | Yes | Non-empty; unique within deployment | Human-readable name (e.g., `"Gregorian → Fiscal 2026"`) |
| `description` | `String` | No | — | Optional narrative description |
| `owner` | `User` | Yes | Must reference a valid user | The user responsible for this mapping |
| `status` | `Enum` | Yes | `active` \| `disabled` | Controls availability for use |
| `version` | `Integer` | Yes | Auto-incremented on every change; starts at 1 | Tracks evolution of the mapping for reproducibility |
| `source_calendar_id` | `UUID` | Yes | Must reference a valid Calendar; immutable after creation | The Calendar being mapped from |
| `target_calendar_id` | `UUID` | Yes | Must reference a valid Calendar; immutable after creation; must differ from `source_calendar_id` | The Calendar being mapped to |
| `mappings` | `List<PeriodPairing>` | Yes | At least one entry; each source/target Period must belong to their respective Calendars | The ordered set of Period-to-Period pairings |
| `created_at` | `Timestamp` | Yes | System-set on creation; immutable | Creation time |
| `updated_at` | `Timestamp` | Yes | System-set on every change | Last modification time |

### PeriodPairing (embedded structure)

| Attribute | Type | Required | Constraints | Description |
|---|---|---|---|---|
| `source_period_id` | `UUID` | Yes | Must reference a Period belonging to `source_calendar_id`; unique within this CalendarMapping | The Period in the source Calendar |
| `target_period_id` | `UUID` | Yes | Must reference a Period belonging to `target_calendar_id` | The Period in the target Calendar it maps to |

## Relationships

| Relationship | Related Entity | Cardinality | Description |
|---|---|---|---|
| maps from | Calendar | N:1 | Many CalendarMappings may originate from the same source Calendar |
| maps to | Calendar | N:1 | Many CalendarMappings may target the same Calendar |
| pairs | Period | N:M | Each PeriodPairing references one source and one target Period |

## Behaviors & Rules

- **BR-001**: `source_calendar_id` and `target_calendar_id` MUST differ — a CalendarMapping cannot map a Calendar to itself.
- **BR-002**: `source_calendar_id` and `target_calendar_id` are immutable after creation. To map a different pair of Calendars, a new CalendarMapping must be created.
- **BR-003**: Each `source_period_id` MUST be unique within a CalendarMapping — a source Period can only map to one target Period in a given directional mapping.
- **BR-004**: All `source_period_id` values MUST reference Periods belonging to `source_calendar_id`. All `target_period_id` values MUST reference Periods belonging to `target_calendar_id`.
- **BR-005**: Version MUST be auto-incremented on every change to `mappings`, `name`, `description`, or `status`.
- **BR-006**: A `disabled` CalendarMapping MUST NOT be used in new cross-calendar operations. Existing Runs that reference a specific version of a disabled CalendarMapping remain valid.
- **BR-007**: A Run that performs cross-calendar aggregation MUST record the `CalendarMapping` `id` and `version` it used, to guarantee reproducibility.
- **BR-008**: A CalendarMapping is directional — it does not imply the reverse mapping. A separate CalendarMapping must be created for the reverse direction if needed.

## Lifecycle

| State | Description | Transitions To |
|---|---|---|
| `active` | Available for use in cross-calendar operations | `disabled` |
| `disabled` | Cannot be used in new operations; existing Run references remain valid | `active` (re-enabled) |

**What creates a CalendarMapping**: A user creates it explicitly, designating a source Calendar, a target Calendar, and an initial set of Period pairings.  
**What modifies a CalendarMapping**: Addition, removal, or change of any PeriodPairing; changes to name, description, or status. Each modification auto-increments the version.  
**What destroys a CalendarMapping**: Open question — see below.

## Boundaries

- A CalendarMapping does NOT define Periods — it only pairs existing Periods from two Calendars.
- A CalendarMapping is NOT bidirectional — it maps strictly from source to target. The reverse requires a separate entity.
- A CalendarMapping does NOT perform aggregation itself — it provides the period translation table; the computation engine performs the actual rollup.
- A CalendarMapping does NOT validate date overlap between paired Periods — it trusts the user's intent.

## Open Questions

- [ ] What are the deletion rules? Can a CalendarMapping be deleted if no Runs reference its versions?
- [ ] Should the system warn when a new Period is added to a source Calendar but has no corresponding PeriodPairing in an active CalendarMapping?

## Serialization (YAML DSL)

Schema for serializing a CalendarMapping for inter-component communication.

```yaml
# calendarmapping.schema.yaml
calendar_mapping:
  id: uuid                          # system-generated, immutable
  name: string                      # required; unique within deployment
  description: string               # optional
  owner: string                     # required; user identifier
  status: active | disabled         # required
  version: integer                  # auto-incremented; starts at 1
  source_calendar_id: uuid          # required; immutable after creation
  target_calendar_id: uuid          # required; immutable after creation
  created_at: timestamp             # system-set on creation; ISO 8601; immutable
  updated_at: timestamp             # system-set on every change; ISO 8601
  mappings:                         # required; at least one entry
    - source_period_id: uuid        # required; Period in source Calendar; unique within this mapping
      target_period_id: uuid        # required; Period in target Calendar
```

> `version`, `created_at`, and `updated_at` are system-managed and MUST be treated as read-only.

```yaml
# Example: Gregorian → Fiscal mapping for Q1 2026
# Gregorian Jan/Feb/Mar each map to their corresponding Fiscal month
calendar_mapping:
  id: "cm-0000-0000-0000-000000000001"
  name: "Gregorian → Fiscal 2026"
  description: "Maps Gregorian calendar months to Fiscal 2026 periods"
  owner: "user-marcos"
  status: active
  version: 1
  source_calendar_id: "cal-0000-0000-0000-000000000000"   # Gregorian (default)
  target_calendar_id: "cal-0000-0000-0000-000000000001"   # Fiscal Calendar
  created_at: "2026-01-01T00:00:00Z"
  updated_at: "2026-01-01T00:00:00Z"
  mappings:
    - source_period_id: "per-greg-202601"     # Gregorian January 2026
      target_period_id: "per-0000-0000-0000-000000000001"  # Fiscal 202601
    - source_period_id: "per-greg-202602"     # Gregorian February 2026
      target_period_id: "per-0000-0000-0000-000000000002"  # Fiscal 202602
    - source_period_id: "per-greg-202603"     # Gregorian March 2026
      target_period_id: "per-0000-0000-0000-000000000003"  # Fiscal 202603
```

## Related Entities

- [[Calendar]] — A CalendarMapping connects exactly two Calendars: a source and a target.
- [[Period]] — Each PeriodPairing references one Period from the source Calendar and one from the target Calendar.
- [[Run]] — A Run performing cross-calendar aggregation MUST record the CalendarMapping `id` and `version` it used for reproducibility.
