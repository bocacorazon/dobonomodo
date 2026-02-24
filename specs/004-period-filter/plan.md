# Implementation Plan: Period Filter

**Branch**: `004-period-filter` | **Date**: 2026-02-24 | **Spec**: [spec.md](./spec.md)
**Input**: Feature specification from `spec.md`

**Note**: This template is filled in by the `/speckit.plan` command. See `.specify/templates/plan-template.md` for the execution workflow.

## Summary

Implement `Period` and `TemporalMode` filtering for `LazyFrame`s in the Engine Worker. This ensures data is filtered by the execution period before processing, supporting both standard period-based and bitemporal data models.

## Technical Context

<!--
  ACTION REQUIRED: Replace the content in this section with the technical details
  for the project. The structure here is presented in advisory capacity to guide
  the iteration process.
-->

**Language/Version**: Rust 1.75
**Primary Dependencies**: `polars` (lazy), `chrono`, `uuid`, `serde`
**Storage**: N/A (Processing layer)
**Testing**: `cargo test` (unit/integration)
**Target Platform**: Linux server
**Project Type**: Library (Rust workspace member `core`)
**Performance Goals**: Utilize `polars` predicate pushdown for efficient filtering.
**Constraints**: Must support both `Period` and `Bitemporal` modes.
**Scale/Scope**: Core filtering logic for all dataset loading.

## Constitution Check

*GATE: Must pass before Phase 0 research. Re-check after Phase 1 design.*

**Constitutional Principles to Verify**:

- [x] **Principle I (TDD)**: All implementation tasks paired with tests written FIRST
- [x] **Principle II (Quality Gates)**: Build, lint, and test infrastructure configured
- [x] **Principle III (Completion Bias)**: Ambiguities resolved; no open decision blocks
- [x] **Principle IV (Comprehensive Testing)**: Test suite execution plan covers all test types

**Notes**:
- TDD will be strictly followed for the filter implementation.
- All new code will be covered by unit tests in `crates/core`.

## Project Structure

### Documentation (this feature)

```text
specs/004-period-filter/
 plan.md              # This file (/speckit.plan command output)
 research.md          # Phase 0 output (/speckit.plan command)
 data-model.md        # Phase 1 output (/speckit.plan command)
 quickstart.md        # Phase 1 output (/speckit.plan command)
 contracts/           # Phase 1 output (/speckit.plan command)
 tasks.md             # Phase 2 output (/speckit.tasks command - NOT created by /speckit.plan)
```

### Source Code (repository root)
<!--
  ACTION REQUIRED: Replace the placeholder tree below with the concrete layout
  for this feature. Delete unused options and expand the chosen structure with
  real paths (e.g., apps/admin, packages/something). The delivered plan must
  not include Option labels.
-->

```text
crates/core/
 src/
   ├── model/
   │   ├── dataset.rs       # Existing entity (confirm TemporalMode)
   │   ├── period.rs        # New Period struct
   │   └── ...
   ├── engine/
   │   └── filter.rs        # New filtering logic
   └── lib.rs
 tests/
    └── engine_filter_test.rs # Integration tests
```

**Structure Decision**: Modify `crates/core` to include `Period` entity and implement filtering logic in `engine` module.

## Complexity Tracking

> **Fill ONLY if Constitution Check has violations that must be justified**

| Violation | Why Needed | Simpler Alternative Rejected Because |
|-----------|------------|-------------------------------------|
| N/A | | |
