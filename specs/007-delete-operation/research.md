# Research: Delete Operation

**Feature**: Delete Operation  
**Date**: 2026-02-22  
**Context**: Technical research for implementing soft-delete operation in DobONoMoDo pipeline engine

## Overview

This document consolidates research findings for implementing the delete operation type in the Rust-based DobONoMoDo computation engine.

## Decision 1: Operation Integration Pattern

**What was chosen**: Extend existing `OperationKind` enum with delete-specific execution logic

**Rationale**: 
- `OperationKind::Delete` variant already exists in `crates/core/src/model/operation.rs`
- Follows established pattern: enum-based dispatch with flexible `parameters: serde_json::Value`
- Consistent with existing operations (Update, Aggregate, Append, Output)

**Alternatives considered**:
- Create separate delete handler outside operation enum -> Rejected: Breaks architectural consistency
- Use special-case Update operation with delete flag -> Rejected: Violates single responsibility, confuses semantics

**Implementation approach**:
```rust
// In OperationInstance.parameters for delete operation:
{
  "selector": "optional_selector_expression"  // null/missing = delete all active rows
}
```

## Decision 2: Selector Integration

**What was chosen**: Reuse existing selector evaluation pipeline (interpolate -> parse -> type-check -> compile to Polars)

**Rationale**:
- Selectors already defined as `BTreeMap<String, String>` in `Project.selectors`
- Existing validation chain handles `{{NAME}}` interpolation, boolean type-checking
- Polars `.filter()` API directly supports compiled selector expressions
- Zero new infrastructure needed

**Alternatives considered**:
- Custom delete-specific filter syntax -> Rejected: Increases complexity, diverges from existing operations
- Hardcoded condition lists -> Rejected: Not flexible, requires code changes for new rules

**Integration points**:
1. Parse selector from `OperationInstance.parameters["selector"]`
2. Interpolate `{{NAME}}` references from `Project.selectors`
3. Compile expression to `polars::lazy::dsl::Expr`
4. Apply filter to working LazyFrame: `df.filter(selector_expr)`

## Decision 3: Row Metadata Handling

**What was chosen**: Use `_deleted` boolean column and `_modified_at` timestamp column in Polars DataFrame

**Rationale**:
- Soft deletion preserves data for auditing and potential recovery
- Polars column operations are zero-copy and efficient
- Metadata persists through pipeline without additional tracking
- Aligns with existing row metadata pattern (`_row_id`, modification tracking)

**Alternatives considered**:
- Separate deleted rows table -> Rejected: Breaks lazy evaluation, increases memory overhead
- Physical row deletion -> Rejected: Violates soft-delete requirement, loses audit trail
- External deletion registry -> Rejected: Complicates synchronization, breaks single DataFrame model

**Metadata update pattern**:
```rust
// For rows matching selector:
df = df.with_column(
    when(selector_expr)
        .then(lit(true))
        .otherwise(col("_deleted"))
        .alias("_deleted")
);
df = df.with_column(
    when(selector_expr)
        .then(lit(current_timestamp()))
        .otherwise(col("_modified_at"))
        .alias("_modified_at")
);
```

## Decision 4: Deleted Row Visibility

**What was chosen**: Automatic filtering of deleted rows for non-output operations; configurable inclusion for output operations

**Rationale**:
- Prevents deleted data pollution in calculations without manual filtering
- Consistent with business rule: "deleted rows don't affect downstream operations"
- Output operations need explicit control for export/archival scenarios
- Follows principle of secure defaults (exclude unless explicitly requested)

**Alternatives considered**:
- Always include deleted rows, require manual filtering -> Rejected: Error-prone, violates spec requirement
- Hard-exclude deleted rows everywhere -> Rejected: Prevents legitimate archival use cases
- Per-operation delete visibility flag -> Rejected: Over-complicates API, most operations don't need this

**Implementation strategy**:
```rust
// After any operation execution (except output):
let working_df = working_df.filter(col("_deleted").eq(lit(false)));

// For output operation:
if !parameters.get("include_deleted").unwrap_or(false) {
    df = df.filter(col("_deleted").eq(lit(false)));
}
```

## Decision 5: No-Selector Behavior

**What was chosen**: When selector is null/missing, delete ALL currently active rows

**Rationale**:
- Supports "purge" and "reset" workflows without artificial `true` expressions
- Simplifies configuration for total-deletion scenarios
- Consistent with spec requirement FR-003
- Follows principle of least surprise (no selector = all rows, like SQL `DELETE` without `WHERE`)

**Alternatives considered**:
- Require explicit `selector: "true"` for all-rows deletion -> Rejected: Unnecessary boilerplate
- Error on missing selector -> Rejected: Prevents valid use cases, overly restrictive
- Default to no-op when selector missing -> Rejected: Violates spec, confusing semantics

**Implementation**:
```rust
let selector_expr = match parameters.get("selector") {
    Some(sel) if !sel.is_null() => parse_and_compile(sel),
    _ => lit(true),  // No selector = match all active rows
};
```

## Technology Patterns

### Polars Lazy Evaluation Best Practices

**Pattern**: Use `.with_column()` for metadata updates, maintain lazy evaluation throughout

**Key insights**:
- Polars lazy API defers computation until `.collect()` or `.sink()`
- Metadata updates via `.with_column()` are expression-based (no eager materialization)
- Filter operations (`.filter()`) preserve lazy evaluation
- Zero-copy operations where possible (column selection, filtering)

**Anti-patterns to avoid**:
- Eager `.collect()` in middle of pipeline -> Forces materialization, increases memory
- Row-by-row iteration -> Breaks vectorization, 100x slower
- Multiple passes over data -> Single-pass updates preferred

**Recommended approach for delete**:
```rust
// Single lazy pass with conditional column updates
df = df
    .with_column(
        when(selector_expr).then(lit(true)).otherwise(col("_deleted")).alias("_deleted")
    )
    .with_column(
        when(selector_expr).then(current_timestamp()).otherwise(col("_modified_at")).alias("_modified_at")
    );
```

### Rust Operation Execution Pattern

**Pattern**: Match on `OperationKind`, delegate to type-specific handler

**Example structure**:
```rust
impl OperationExecutor {
    fn execute(&self, op: &OperationInstance, df: LazyFrame) -> Result<LazyFrame> {
        match op.kind {
            OperationKind::Delete => self.execute_delete(op, df),
            OperationKind::Update => self.execute_update(op, df),
            // ... other variants
        }
    }

    fn execute_delete(&self, op: &OperationInstance, df: LazyFrame) -> Result<LazyFrame> {
        // 1. Parse selector from op.parameters
        // 2. Compile to Polars expr
        // 3. Update metadata columns
        // 4. Return updated LazyFrame
    }
}
```

### Test Strategy

**Unit tests** (`crates/core/tests/unit/`):
- Selector compilation correctness
- Metadata update logic
- Edge cases (null selector, zero matches, all matches)

**Integration tests** (`crates/core/tests/integration/`):
- Multi-operation pipelines with delete
- Deleted row exclusion from subsequent operations
- Operation sequencing correctness

**Contract tests** (`crates/test-resolver/tests/scenarios/`):
- YAML-based acceptance scenarios from spec
- End-to-end pipeline execution with delete steps
- Output visibility configuration (include_deleted flag)

## Open Questions (RESOLVED)

All technical unknowns resolved during research phase:

- [PASS] How selectors work: String expressions, `{{NAME}}` interpolation, compiled to Polars
- [PASS] Operation structure: Enum-based dispatch with flexible parameters
- [PASS] Metadata handling: `_deleted` boolean + `_modified_at` timestamp columns
- [PASS] Execution model: Lazy evaluation, single-pass updates, automatic filtering

## Next Steps

Proceed to Phase 1 design artifacts:
1. **data-model.md**: Define DeleteOperation parameters, row metadata schema
2. **contracts/**: API specifications (if REST API changes needed)
3. **quickstart.md**: Developer guide for implementing and testing delete operation
