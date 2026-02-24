# Implementation Tasks: Period Filter

> **Feature**: Period Filter
> **Spec**: [spec.md](./spec.md)
> **Plan**: [plan.md](./plan.md)

## Phase 1: Setup
*Goal: Initialize project state and verify prerequisites.*

- [x] T001 Verify project build and test state
- [x] T002 Verify `crates/core` dependencies (`polars`, `chrono`, `uuid`, `serde`) in `crates/core/Cargo.toml`

## Phase 2: Foundational
*Goal: Create core entities and module structure required for filtering logic.*

- [x] T003 Create `Period` struct in `crates/core/src/model/period.rs` with `identifier`, `start_date`, and `end_date`
- [x] T004 Export `Period` in `crates/core/src/model/mod.rs`
- [x] T005 [P] Create `crates/core/src/engine/mod.rs` and expose `filter` module
- [x] T006 [P] Create skeleton `crates/core/src/engine/filter.rs` with empty `apply_filter` function

## Phase 3: User Story 1 - Period Mode Filtering
*Goal: Implement filtering by exact period identifier match.*
*Priority: P1*

**Test Criteria**:
- Given a `LazyFrame` and `TemporalMode::Period`, rows matching `_period` == target are kept.
- Rows with non-matching `_period` are dropped.

- [x] T007 [US1] Define `FilterContext` struct in `crates/core/src/engine/filter.rs` to hold target `Period`
- [x] T008 [US1] Implement `apply_period_filter` in `crates/core/src/engine/filter.rs` using `polars` expression `col("_period").eq(lit(target))`
- [x] T009 [US1] Add unit tests for `apply_period_filter` in `crates/core/src/engine/filter.rs` covering match and no-match scenarios

## Phase 4: User Story 2 - Bitemporal Filtering
*Goal: Implement as-of filtering logic for bitemporal data.*
*Priority: P1*

**Test Criteria**:
- Given a `LazyFrame` and `TemporalMode::Bitemporal`, rows where `_period_from` <= target_start < `_period_to` (or null) are kept.

- [x] T010 [US2] Implement `apply_bitemporal_filter` in `crates/core/src/engine/filter.rs` using `polars` expressions for date range check
- [x] T011 [US2] Handle NULL `_period_to` as "valid until infinity" in `apply_bitemporal_filter`
- [x] T012 [US2] Add unit tests for `apply_bitemporal_filter` in `crates/core/src/engine/filter.rs` covering effective range and edge cases

## Phase 5: User Story 3 - Deleted Row Exclusion
*Goal: Automatically exclude soft-deleted rows from all results.*
*Priority: P2*

**Test Criteria**:
- Rows with `_deleted=true` are excluded regardless of temporal match.

- [x] T013 [US3] Update `apply_filter` in `crates/core/src/engine/filter.rs` to include `col("_deleted").neq(lit(true))` (or `is_null` check if nullable)
- [x] T014 [US3] Add unit tests for `_deleted` flag handling in `crates/core/src/engine/filter.rs`

## Phase 6: Polish & Integration
*Goal: Finalize implementation and ensure comprehensive test coverage.*

- [x] T015 Create integration tests in `crates/core/tests/engine_filter_test.rs` validating full `apply_filter` workflow on `LazyFrame`
- [x] T016 Run `cargo clippy` and fix any linting issues in `crates/core`
- [x] T017 Run `cargo test` to ensure no regressions in existing `dataset` or other modules

## Dependencies
1. **US1 (Period Mode)**: Depends on Foundational (Period struct).
2. **US2 (Bitemporal)**: Independent of US1 implementation details, but shares module.
3. **US3 (Deleted Rows)**: Can be implemented alongside or after US1/US2.

## Parallel Execution Examples
- **US1 & US2**: `apply_period_filter` and `apply_bitemporal_filter` can be implemented by different developers once `crates/core/src/engine/filter.rs` skeleton exists.
- **Tests**: Unit tests for each mode can be written in parallel with implementation.

## Implementation Strategy
- **MVP**: Complete Phase 1-3 to support basic Period-based processing.
- **Full Scope**: Complete Phase 4-5 to support Bitemporal and soft-deletes.
