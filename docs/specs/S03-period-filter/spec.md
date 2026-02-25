# S03: Period Filter

## Feature
Load Dataset table data into Polars `LazyFrame`s and apply temporal filtering based on each table's `temporal_mode` — exact match on `_period` for period-mode tables, and asOf query on `_period_from`/`_period_to` for bitemporal tables.

## Context
- Read: `docs/entities/dataset.md` (`temporal_mode` on TableRef, BR-018 through BR-021, system columns)
- Read: `docs/capabilities/execute-project-calculation.md` (resolved OQ-003 — period filter mechanism)
- Read: `docs/architecture/sample-datasets.md` (sample data including bitemporal `exchange_rates`, test scenarios TS-01 and TS-02)

## User Scenarios & Testing

<!--
  IMPORTANT: User stories should be PRIORITIZED as user journeys ordered by importance.
-->

### User Story 1 - Period Mode Filtering (Priority: P1)

As an Engine Worker, I need to filter datasets by a specific period identifier so that calculations only use data relevant to the target period.

**Why this priority**: Core functionality for period-based processing.

**Independent Test**: Can be tested with a dataset and a target period string.

**Acceptance Scenarios**:

1. **Given** a period-mode table with rows for "2024-01" and "2024-02", **When** filtering for "2024-01", **Then** only "2024-01" rows remain.
2. **Given** a table with no matching rows, **When** filtering, **Then** an empty dataset is returned (no error).

---

### User Story 2 - Bitemporal Filtering (Priority: P1)

As an Engine Worker, I need to filter bitemporal datasets using "as-of" logic so that I see the correct version of data effective at the period start date.

**Why this priority**: Essential for handling slow-changing dimensions and bitemporal reference data.

**Independent Test**: Can be tested with bitemporal rows (valid_from, valid_to) and a target date.

**Acceptance Scenarios**:

1. **Given** a rate valid from "2023-01-01" to "2023-12-31", **When** filtering for "2024-01" (start date 2024-01-01), **Then** the row is excluded.
2. **Given** a rate valid from "2024-01-01" to NULL, **When** filtering for "2024-01", **Then** the row is included.

---

### User Story 3 - Deleted Row Exclusion (Priority: P2)

As an Engine Worker, I need to automatically exclude soft-deleted rows so that downstream calculations don't process invalid data.

**Why this priority**: Data integrity requirement.

**Independent Test**: Test with rows marked `_deleted=true` and `_deleted=false`.

**Acceptance Scenarios**:

1. **Given** a row with `_deleted=true` that matches the temporal filter, **When** filtering, **Then** the row is excluded.

### Edge Cases

- **Empty Result**: If no rows match the filter, the system must return an empty dataset with the correct schema, not an error.
- **Missing Columns**: If required system columns (`_period`, `_period_from`, etc.) are missing, the system should fail gracefully (though this is likely handled by upstream validation).
- **Null Validity End**: For bitemporal data, a NULL `_period_to` implies "valid until infinity".

## Requirements

### Functional Requirements

- **FR-001**: The system MUST filter period-mode tables by exact match on the `_period` column against the target period identifier.
- **FR-002**: The system MUST filter bitemporal-mode tables where `_period_from` <= target period start date AND (`_period_to` IS NULL OR `_period_to` > target period start date).
- **FR-003**: The system MUST exclude all rows where `_deleted` is true.
- **FR-004**: The system MUST return an empty result set (not an error) if no rows match the filter criteria.
- **FR-005**: The system MUST support `TemporalMode::Period` and `TemporalMode::Bitemporal` configurations.

### Key Entities

- **Period**: Object containing `identifier` (String) and `start_date` (Date/Datetime).
- **TemporalMode**: Enumeration distinguishing `Period` vs `Bitemporal` filtering logic.
- **Dataset**: Abstract representation of the tabular data to be filtered.

## Success Criteria

### Measurable Outcomes

- **SC-001**: 100% of rows returned for a period-mode query match the requested `_period`.
- **SC-002**: 100% of rows returned for a bitemporal query are effective as of the period start date.
- **SC-003**: 0% of returned rows have `_deleted=true`.
- **SC-004**: Filter operations are optimized for performance (e.g., using query pushdown where applicable).


## Assumptions
- Input `LazyFrame` schema matches the `TemporalMode` requirements (checked by caller or previous validation steps).
- `Period` struct is available from Core (S01/S14).
- `_period` columns are standard system columns defined in `dataset.md`.
