# S03: Period Filter

## Feature
Load Dataset table data into Polars `LazyFrame`s and apply temporal filtering based on each table's `temporal_mode` — exact match on `_period` for period-mode tables, and asOf query on `_period_from`/`_period_to` for bitemporal tables.

## Context
- Read: `docs/entities/dataset.md` (`temporal_mode` on TableRef, BR-018 through BR-021, system columns)
- Read: `docs/capabilities/execute-project-calculation.md` (resolved OQ-003 — period filter mechanism)
- Read: `docs/architecture/sample-datasets.md` (sample data including bitemporal `exchange_rates`, test scenarios TS-01 and TS-02)

## Scope

### In Scope
- `core::engine::period_filter` module
- Function: given a `LazyFrame` + `TemporalMode` + `Period` → filtered `LazyFrame`
  - `TemporalMode::Period`: filter `_period = period.identifier`
  - `TemporalMode::Bitemporal`: filter `_period_from <= period.start_date AND (_period_to IS NULL OR _period_to > period.start_date)`
- Automatic `_deleted = true` exclusion from the filtered frame (applied after temporal filter)
- Unit tests using inline DataFrames (no IO — pure Polars)
- Test harness scenarios TS-01 and TS-02 from sample-datasets.md

### Out of Scope
- Data loading from physical sources (uses `DataLoader` trait — test with `InMemoryDataLoader` from S02)
- Resolver rule evaluation (S11)
- Operation execution (S04+)

## Dependencies
- **S01** (DSL Parser): for expression compilation (used in downstream ops, not directly here — but the `Period` struct is needed)
- **S02** (Test Harness): for running test scenarios

## Parallel Opportunities
Once this spec is complete, **S04 through S09** and **S11** can all start in parallel.

## Key Design Decisions
- `temporal_mode` is per-TableRef, not per-Dataset — a period main table can join a bitemporal lookup
- `_period` stores the Period identifier string; filtering is exact match
- Bitemporal asOf uses `period.start_date` as the reference point
- Non-overlap for bitemporal is a data contract — engine does NOT enforce
- `_deleted = true` rows are excluded from ALL downstream operations by default

## Success Criteria
- Period-mode table with mixed `_period` values returns only matching rows
- Bitemporal table returns correct asOf rows (e.g., `exchange_rates` for 2026-01-01 returns new rates, not old ones)
- Rows with `_deleted = true` are excluded
- Empty result after filtering does not error (returns empty `LazyFrame` with correct schema)
