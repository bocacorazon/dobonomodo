# Research: Aggregate Operation

**Feature**: 001-aggregate-operation  
**Date**: 2026-02-22  
**Phase**: 0 - Outline & Research

## Research Objectives

This document resolves technical unknowns and design decisions for implementing the aggregate operation type in the DobONoMoDo computation engine.

---

## 1. Polars Aggregate API Patterns

**Decision**: Use Polars `LazyFrame::group_by()` with `.agg()` for group-by aggregations

**Rationale**: 
- Polars lazy API allows query optimization before execution
- Native support for common aggregate functions (sum, count, avg, min, max)
- Efficient columnar execution model aligns with our in-memory working dataset approach
- Type safety through Polars expression system

**Implementation Approach**:
```rust
// Pseudo-code based on Polars 0.46 API
let grouped = lazy_frame
    .group_by([col("account_type"), col("_period")])
    .agg([
        col("amount").sum().alias("total_amount"),
        col("order_id").count().alias("order_count"),
    ]);
```

**Alternatives Considered**:
- Manual HashMap-based grouping: Rejected due to complexity, no type safety, slower performance
- SQL-based aggregation via DataFusion: Rejected to maintain single engine dependency (Polars)

**References**:
- Polars documentation: https://docs.pola.rs/api/python/stable/reference/lazyframe/api/polars.LazyFrame.group_by.html
- Aggregate functions: https://docs.pola.rs/user-guide/expressions/aggregation/

---

## 2. UUID v7 Generation for Row IDs

**Decision**: Use `uuid::Uuid::now_v7()` for generating unique `_row_id` on summary rows

**Rationale**:
- Time-ordered UUIDs provide natural sort order by creation time
- Monotonic ordering helps with debugging and tracing
- Already a workspace dependency (`uuid = { version = "1", features = ["serde", "v7"] }`)
- Better index performance in PostgreSQL compared to random UUIDs

**Implementation Approach**:
```rust
use uuid::Uuid;

fn generate_row_id() -> Uuid {
    Uuid::now_v7()
}
```

**Alternatives Considered**:
- UUID v4 (random): Rejected due to lack of temporal ordering
- Sequential integers: Rejected due to collision risk in distributed scenarios
- Snowflake IDs: Rejected to avoid additional dependencies

**References**:
- RFC 9562 UUID v7: https://www.rfc-editor.org/rfc/rfc9562.html
- uuid crate docs: https://docs.rs/uuid/latest/uuid/

---

## 3. System Metadata Population Strategy

**Decision**: Populate required system columns on summary rows using execution context values

**System Columns to Populate**:
| Column | Source | Value |
|--------|--------|-------|
| `_row_id` | Generated | `Uuid::now_v7()` |
| `_created_at` | Execution time | Current timestamp |
| `_updated_at` | Execution time | Current timestamp (same as created) |
| `_source_dataset_id` | Run context | Input dataset UUID |
| `_source_table` | Run context | Primary table name from dataset |
| `_deleted` | Static | `false` (summary rows are never pre-deleted) |
| `_period` | Group-by value | From grouped column if present, else null |

**Rationale**:
- Maintains consistency with existing row contract
- Enables downstream operations to treat summary rows identically to detail rows
- Supports tracing and lineage tracking
- `_deleted: false` ensures summary rows participate in subsequent operations

**Implementation Notes**:
- System columns must be added to Polars DataFrame after aggregation
- Use `.with_columns()` to add metadata columns efficiently
- Timestamp generation uses `chrono::Utc::now()`

**Alternatives Considered**:
- Minimal metadata (only _row_id): Rejected due to downstream operation compatibility requirements
- Copy metadata from first row in group: Rejected as semantically incorrect for summary rows

---

## 4. Null Handling for Non-Aggregated Columns

**Decision**: Set all non-grouped, non-aggregated business columns to `null` on summary rows

**Rationale**:
- Summary rows represent multiple detail rows - no single detail value is correct
- Explicit nulls are clearer than arbitrary defaults or missing columns
- Maintains schema consistency with working dataset
- Follows SQL standard behavior for GROUP BY

**Implementation Approach**:
1. Determine working dataset schema
2. Identify columns not in group_by list or aggregation outputs
3. Add null columns using Polars `.with_columns([lit(null).alias("column_name")])`

**Column Classification**:
- **Group-by columns**: Copied from group key values
- **Aggregation output columns**: Computed via aggregate functions
- **System metadata columns**: Populated per decision #3
- **Other business columns**: Set to null

**Alternatives Considered**:
- Omit non-aggregated columns entirely: Rejected due to schema mismatch with working dataset
- Use default values (0, empty string): Rejected as semantically misleading
- Raise error if non-aggregated columns exist: Rejected as overly restrictive

---

## 5. Validation Strategy

**Decision**: Implement multi-phase validation with fail-fast semantics

**Validation Phases**:

1. **Parse-time validation** (before execution):
   - `group_by` list is non-empty
   - `aggregations` list is non-empty
   - No duplicate column names in group_by list
   - Aggregate output column names don't conflict with system columns

2. **Compile-time validation** (with dataset schema):
   - All group_by columns exist in working dataset schema
   - All column references in aggregate expressions exist
   - Aggregate functions are valid (SUM, COUNT, AVG, MIN_AGG, MAX_AGG)
   - Expression types are compatible with aggregate functions

3. **Runtime checks** (during execution):
   - Selector filter produces valid row subset
   - Group-by columns have compatible types for grouping

**Error Types**:
```rust
#[derive(Debug, thiserror::Error)]
pub enum AggregateError {
    #[error("group_by list cannot be empty")]
    EmptyGroupBy,
    
    #[error("aggregations list cannot be empty")]
    EmptyAggregations,
    
    #[error("duplicate group_by column: {0}")]
    DuplicateGroupByColumn(String),
    
    #[error("unknown column in group_by: {0}")]
    UnknownGroupByColumn(String),
    
    #[error("aggregate output column conflicts with system column: {0}")]
    SystemColumnConflict(String),
    
    #[error("invalid aggregate expression: {0}")]
    InvalidExpression(String),
}
```

**Rationale**:
- Early validation prevents partial execution and data corruption
- Explicit error messages aid debugging during development
- Aligns with TDD principle (tests written for each validation case)

**Alternatives Considered**:
- Runtime-only validation: Rejected due to risk of partial execution failures
- Warning-based validation: Rejected due to data integrity requirements

---

## 6. Append Semantics

**Decision**: Append summary rows to working dataset using Polars `concat()` with vertical stacking

**Implementation Approach**:
```rust
// Pseudo-code
let summary_rows = compute_aggregates(working_dataset, operation_spec)?;
let updated_dataset = concat([working_dataset, summary_rows], UnionArgs::default())?;
```

**Rationale**:
- Preserves all original rows (FR-005 requirement)
- Simple, efficient operation in Polars
- Maintains insertion order (detail rows first, summary rows last)
- No risk of accidental row modification

**Row Ordering**:
- Original detail rows: Retain original order
- Summary rows: Appended in arbitrary group order (Polars may optimize grouping)
- No guaranteed ordering between summary rows (groups processed in parallel)

**Alternatives Considered**:
- In-place modification with marker column: Rejected due to complexity and FR-005 violation risk
- Separate summary DataFrame: Rejected as incompatible with single working dataset model

---

## 7. Expression Parsing Integration

**Decision**: Delegate expression parsing and validation to existing DSL capability (S01 dependency)

**Assumption**: 
- Expression parser is available and handles aggregate function validation
- Expressions return typed Polars expressions compatible with `.agg()`
- Parser validates aggregate function usage context

**Required Expression API** (to be confirmed in Phase 1):
```rust
pub fn parse_aggregate_expression(
    source: &str,
    context: &SchemaContext,
) -> Result<PolarsExpr, ExpressionError>;
```

**Integration Points**:
- Parse all `group_by` column references
- Parse all `aggregation.expression` sources
- Validate aggregate functions (SUM, COUNT, AVG, MIN_AGG, MAX_AGG) only in aggregate context
- Reject aggregate functions in selector expressions

**Deferred to S01**:
- Expression syntax parsing
- Type checking
- Aggregate function name resolution
- Column reference validation against schema

---

## 8. Test Strategy

**Decision**: Comprehensive test coverage across unit, integration, and contract test levels

**Test Categories**:

1. **Unit Tests** (`tests/unit/aggregate_validation_test.rs`):
   - Empty group_by validation
   - Empty aggregations validation
   - Duplicate group_by column detection
   - Unknown column reference detection
   - System column conflict detection
   - Edge case: zero input rows
   - Edge case: single group
   - Edge case: null values in group_by columns

2. **Integration Tests** (`tests/integration/aggregate_execution_test.rs`):
   - Basic group-by with SUM, COUNT
   - Multi-column group-by
   - All aggregate functions (SUM, COUNT, AVG, MIN_AGG, MAX_AGG)
   - Selector filtering before aggregation
   - System metadata population verification
   - Non-aggregated columns set to null
   - Original rows preservation (append, not replace)
   - TS-05 scenario: monthly totals by account type

3. **Contract Tests** (`tests/contracts/aggregate_contract_test.rs`):
   - Operation definition deserialization (YAML/JSON)
   - Aggregation structure serialization
   - Schema validation for summary rows
   - Error message format consistency

**Test Data**:
- Use Polars DataFrame fixtures with known values
- Seed data: 100 rows, 3 groups, multiple time periods
- Expected outcomes pre-calculated for each test case

**TDD Workflow**:
1. Write failing test for validation case
2. Implement minimal code to pass validation test
3. Write failing test for execution case
4. Implement execution logic to pass test
5. Refactor while keeping tests green
6. Run full test suite before commit

**Rationale**:
- Aligns with Constitution Principle I (TDD)
- Comprehensive coverage per Principle IV
- Tests document expected behavior
- Regression protection for future changes

---

## 9. Performance Considerations

**Decision**: Optimize for clarity and correctness first, profile before micro-optimizations

**Expected Performance Profile**:
- Grouping: O(n) with Polars parallel hash-based grouping
- Aggregation: O(n) single pass per aggregate function
- Concatenation: O(n) copy (unavoidable for append semantics)
- Overall: O(n) where n = input row count

**Memory Usage**:
- Working dataset remains in memory (LazyFrame)
- Grouping creates intermediate hash tables (bounded by distinct group count)
- Summary rows add minimal overhead (typically << 1% of detail rows)

**Optimization Opportunities** (deferred unless profiling shows need):
- Parallel group-by for very large datasets (Polars may already optimize)
- Streaming aggregation for memory-constrained environments
- Lazy evaluation of aggregates only when downstream operations need them

**Profiling Plan**:
- Benchmark with 1M row dataset, 1000 groups
- Measure memory usage before/after aggregation
- Profile CPU time for each phase (filter, group, aggregate, append)

**Rationale**:
- Premature optimization violates Principle III (Completion Bias)
- Polars already heavily optimized for aggregation workloads
- Focus on correctness and maintainability first

---

## 10. Open Questions Resolved

**OQ-001**: How are `_row_id` and lineage columns populated on summary rows?
- **Resolution**: Generate new UUIDs via `Uuid::now_v7()` for each summary row. Lineage columns (`_source_dataset_id`, `_source_table`) copied from execution context, not from detail rows.

**OQ-002**: Should selector filter be applied before or after grouping?
- **Resolution**: Selector filters rows **before** grouping. Summary rows are computed only from non-deleted rows matching the selector. This aligns with standard operation behavior.

**OQ-003**: Can aggregate operations be chained (aggregate-on-aggregate)?
- **Resolution**: Yes, via standard operation sequencing. Subsequent operations see all rows (detail + previous summaries). Use selectors to distinguish summary from detail if needed.

---

## Summary

All technical unknowns from Technical Context have been resolved:
- ✅ Polars API patterns established
- ✅ UUID generation strategy defined
- ✅ System metadata population specified
- ✅ Null handling for non-aggregated columns clarified
- ✅ Validation strategy comprehensive
- ✅ Append semantics well-defined
- ✅ Expression parsing integration scoped
- ✅ Test strategy complete
- ✅ Performance considerations documented

**Ready to proceed to Phase 1: Design & Contracts**
