# Research: Update Operation

**Feature**: Update Operation (S04)  
**Date**: 2025-02-22  
**Status**: Phase 0 Complete

## Overview

This document consolidates research findings for implementing the `update` operation in the DobONoMoDo computation engine. All technical unknowns from the Technical Context have been investigated, and design decisions are documented with rationale.

---

## Research Tasks

### 1. Performance Goals (Expression Compilation Latency)

**Decision**: Target sub-millisecond expression compilation for typical update expressions (<10 assignments).

**Rationale**: 
- Polars uses a lazy evaluation model where expressions are compiled to an execution plan, not executed immediately
- Compilation overhead is amortized across the entire dataset when the LazyFrame is materialized
- Benchmarks from Polars community show expression building (AST construction) is typically <100µs per expression
- For a typical update operation with 5-10 assignments, compilation should complete in <1ms
- Expression execution (the actual row transformation) dominates total latency, not compilation

**Alternatives Considered**:
- **No explicit performance target**: Rejected because it doesn't align with Principle II (quality gates require measurable criteria)
- **Strict <100µs per expression**: Rejected as overly restrictive for initial implementation; optimization can follow profiling

**Implementation Impact**: 
- No special optimization required in initial implementation
- Add compilation time metrics in trace events for future optimization
- If profiling reveals compilation bottlenecks, consider expression caching or compilation-time optimizations

---

### 2. Memory Constraints (Maximum Dataset Size)

**Decision**: No explicit memory limit in the update operation implementation; rely on Polars' streaming execution and lazy evaluation.

**Rationale**:
- Polars' lazy API defers execution and can stream large datasets in chunks
- Memory usage is primarily determined by the working dataset size, not the update operation logic
- The computation engine's memory limits should be enforced at the Run/Project level, not per-operation
- Kubernetes Job resource limits (configured externally) will cap memory usage in production
- For development/testing, typical datasets will fit in memory (<1GB)

**Alternatives Considered**:
- **Hard-coded memory limit (e.g., 4GB)**: Rejected because it's deployment-specific and should be configured externally
- **Per-operation memory tracking**: Rejected as premature optimization; adds complexity without clear benefit

**Implementation Impact**:
- Update operation works on LazyFrame without materialization (preserves lazy evaluation)
- No memory checks in update operation code
- Document memory considerations in operation-level documentation
- Future work: Add memory metrics to trace events for monitoring

---

### 3. Error Handling Strategy

**Decision**: Use `Result<T, anyhow::Error>` for all fallible operations; panic only on programming errors (invariant violations).

**Rationale**:
- Existing codebase uses `anyhow` for error handling (confirmed in `Cargo.toml`)
- Expression compilation errors (undefined columns, type mismatches) are runtime errors, not panics
- Named selector resolution failures (undefined `{{NAME}}`) are compile-time validation errors, return Result
- Polars expression errors propagate as `PolarsError`, wrapped in `anyhow::Error`
- This aligns with Rust best practices: panics for bugs, Results for expected failures

**Alternatives Considered**:
- **Panic on all errors**: Rejected; loses recoverability and crashes the entire computation
- **Custom error types (thiserror)**: Considered but deferred; `anyhow` provides sufficient context for initial implementation
- **Option<T> for failures**: Rejected; loses error context needed for debugging

**Error Categories**:
| Error Type | Handling | Example |
|------------|----------|---------|
| Undefined selector name | `Err(anyhow!("Selector '{{NAME}}' not found"))` | `{{invalid_selector}}` |
| Undefined column in expression | `Err(PolarsError)` propagated | `unknown_column + 1` |
| Type mismatch in assignment | `Err(PolarsError)` propagated | `string_col + numeric_col` |
| Invalid expression syntax | `Err(anyhow!("Parse error"))` | Malformed expression string |
| Programming error (e.g., null LazyFrame) | `panic!` or `unreachable!` | Internal invariant violation |

**Implementation Impact**:
- All public functions return `Result<LazyFrame, anyhow::Error>` or similar
- Use `?` operator for error propagation
- Add context with `.context()` for meaningful error messages
- Document error conditions in function documentation

---

### 4. Polars Expression Compilation Best Practices

**Decision**: Use Polars' `Expr` API directly for expression compilation; leverage `col()`, `lit()`, and expression builders.

**Rationale**:
- Polars provides a rich expression DSL via the `Expr` type
- Expressions are lazy (not evaluated until materialization)
- Expression builders support all operations needed: column references, literals, functions, conditionals
- Polars handles optimization (predicate pushdown, column pruning) automatically
- No need for custom expression parsing if DSL expressions map cleanly to Polars Expr

**Best Practices** (from Polars documentation and community):
1. **Use `col("name")` for column references**: Type-safe, supports schema validation
2. **Use `lit(value)` for literals**: Handles type coercion automatically
3. **Chain expressions with methods**: `col("a").add(col("b")).alias("result")`
4. **Avoid premature materialization**: Keep LazyFrame lazy until final output
5. **Use `with_columns()` for bulk updates**: More efficient than sequential `with_column()` calls
6. **Leverage `when().then().otherwise()`** for conditional logic (if DSL supports it)

**Alternatives Considered**:
- **Custom expression parser**: Rejected; Polars Expr API is sufficient and battle-tested
- **SQL translation layer**: Rejected; adds complexity, DSL is not SQL
- **Abstract Expr wrapper**: Deferred; start with direct Polars API, abstract if needed

**Implementation Impact**:
- Expression compilation logic in `update.rs` will parse DSL expression strings and build Polars `Expr` trees
- Dependency on S01 (DSL Parser) for parsing expression strings into an AST or IR
- Update operation receives parsed expressions (or expression strings) and compiles to Polars `Expr`

---

### 5. Selector Interpolation Strategy ({{NAME}} Resolution)

**Decision**: Resolve named selectors at compile time (before Polars expression compilation) via string substitution from the Project's `selectors` map.

**Rationale**:
- Named selectors are defined in the Project entity (see `docs/entities/project.md`)
- Selectors are captured in the ProjectSnapshot at Run creation, ensuring reproducibility
- String substitution is simple and aligns with DSL design (no runtime overhead)
- Undefined selector names can be detected early (before Polars compilation)

**Algorithm**:
```
1. Receive selector string from Operation (e.g., "{{active_orders}}")
2. Check if selector contains {{...}} pattern
3. If yes:
   a. Extract selector name (e.g., "active_orders")
   b. Lookup name in Project.selectors map
   c. If found: replace {{NAME}} with selector expression
   d. If not found: return Err("Selector 'NAME' not defined")
4. If no: use selector string as-is
5. Compile resulting expression to Polars Expr
```

**Alternatives Considered**:
- **Runtime interpolation**: Rejected; adds complexity, loses compile-time validation
- **Macro-based expansion**: Rejected; Rust macros are compile-time (language level), not runtime (data level)
- **Recursive interpolation** (selectors referencing selectors): Deferred; not in S04 scope

**Implementation Impact**:
- Update operation receives Project context (or selectors map) to resolve named selectors
- Add helper function: `resolve_selector(selector_str: &str, selectors: &HashMap<String, String>) -> Result<String>`
- Unit tests for selector resolution (valid names, undefined names, no interpolation)

---

### 6. System Column Updates (_updated_at)

**Decision**: Always set `_updated_at` to the Run's current timestamp for every row modified by the update operation.

**Rationale**:
- System columns track metadata for auditing and versioning
- `_updated_at` should reflect the last modification time, which is the Run execution time
- All modified rows get the same timestamp (Run.started_at or Run.current_period.start)
- Non-modified rows (filtered out by selector) retain their original `_updated_at` value

**Timestamp Source**:
- Use Run's `started_at` timestamp (available from execution context)
- If Run object is not available, accept timestamp as parameter to update function
- Timestamp is a `chrono::DateTime<Utc>` (per workspace dependencies)

**Implementation**:
```rust
// Pseudo-code
fn apply_update(
    lazy_frame: LazyFrame,
    selector: Expr,
    assignments: Vec<Assignment>,
    run_timestamp: DateTime<Utc>
) -> Result<LazyFrame> {
    // 1. Filter rows by selector
    let filtered = lazy_frame.filter(selector);
    
    // 2. Apply assignments
    let updated = filtered.with_columns(assignments_as_exprs);
    
    // 3. Add system column update
    let final = updated.with_column(lit(run_timestamp).alias("_updated_at"));
    
    // 4. Union with non-matching rows (unchanged)
    // ...
}
```

**Alternatives Considered**:
- **Current system time (`Utc::now()`)**: Rejected; Run timestamp ensures reproducibility
- **Per-row different timestamps**: Rejected; not needed, adds complexity
- **Optional _updated_at update**: Rejected; always update for consistency

**Implementation Impact**:
- Update function signature includes `run_timestamp: DateTime<Utc>` parameter
- Add `_updated_at` column update after all assignment expressions
- Document in operation contract that `_updated_at` is always set for modified rows

---

## Summary

All NEEDS CLARIFICATION items from Technical Context have been resolved:

| Item | Decision |
|------|----------|
| **Performance Goals** | Sub-millisecond expression compilation (<1ms for <10 assignments) |
| **Memory Constraints** | No explicit limit; rely on Polars streaming + external resource limits |
| **Error Handling** | `Result<T, anyhow::Error>` for fallible ops; panic only for programming errors |

**Additional Research Outcomes**:
- Polars Expr API best practices documented
- Named selector interpolation strategy defined (compile-time string substitution)
- System column update approach specified (`_updated_at` set to Run timestamp)

**Unblocked for Phase 1**: Data model and contract design can proceed with full technical clarity.
