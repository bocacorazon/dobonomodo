# Research: Append Operation Implementation

**Feature**: 009-append-operation  
**Date**: 2026-02-22  
**Purpose**: Resolve technical unknowns and establish implementation patterns for append operation

## Executive Summary

The append operation will leverage Polars DataFrame for efficient row processing, filtering, and aggregation. Implementation follows existing patterns in the codebase (resolver pattern for data loading, Expression struct for filters, operation execution via engine). All NEEDS CLARIFICATION items from Technical Context resolved with concrete decisions.

---

## 1. Performance Goals & Constraints

### Decision: Target <50ms for typical append operations

**Rationale**:
- Polars benchmarks show 10k-100k row operations complete in 5-50ms range
- Financial datasets typically process in batches: 10k rows (monthly data) to 100k rows (annual data)
- Budget vs actual comparison (primary use case) involves <50k total rows
- <50ms latency enables interactive analysis workflows

**Alternatives considered**:
- <10ms threshold: Too aggressive for 100k row operations with aggregation
- <200ms threshold: Too relaxed; Polars can achieve better
- Async/streaming: Unnecessary complexity for in-memory DataFrames at this scale

**Concrete targets**:
- Simple append (no aggregation): <10ms for 100k rows
- Filtered append (source_selector): <20ms for 100k rows
- Aggregated append (group_by + 3 aggregations): <50ms for 100k rows

---

## 2. Aggregation Function Implementation Strategy

### Decision: Parse aggregate expressions into Polars Expr at runtime

**Rationale**:
- AppendAggregation config uses string expressions ("SUM(amount_local)")
- Polars provides native functions: `.sum()`, `.mean()`, `.min()`, `.max()`, `.count()`
- Runtime parsing enables flexible aggregation definitions without code generation
- Matches existing Expression pattern in codebase (expression.rs has `source: String`)

**Implementation approach**:
```rust
// Parse "SUM(amount_local)" → col("amount_local").sum().alias("total")
fn parse_aggregation(expr: &str, output_col: &str) -> Result<Expr> {
    // Extract: FUNC(column_name)
    let parts: Vec<&str> = expr.split('(').collect();
    let func = parts[0].trim().to_uppercase();
    let col_name = parts[1].trim_end_matches(')').trim();
    
    match func.as_str() {
        "SUM" => Ok(col(col_name).sum().alias(output_col)),
        "COUNT" => Ok(col(col_name).count().alias(output_col)),
        "AVG" => Ok(col(col_name).mean().alias(output_col)),
        "MIN_AGG" => Ok(col(col_name).min().alias(output_col)),
        "MAX_AGG" => Ok(col(col_name).max().alias(output_col)),
        _ => Err(anyhow!("Unsupported aggregate function: {}", func)),
    }
}
```

**Alternatives considered**:
- **Code generation**: Rejected - adds build complexity, no runtime flexibility
- **Enum-based function selection**: Rejected - requires changing all aggregation configs to structured format (breaks existing patterns)
- **Full expression parser**: Deferred - spec only requires simple aggregations, complex expressions can be added later

**Validation**:
- Parse expression syntax during operation planning (fail early)
- Verify column names exist in source dataset schema
- Reject unsupported functions at parse time

---

## 3. Error Handling Patterns for Missing Datasets

### Decision: Fail at operation planning phase with typed error

**Rationale**:
- Missing dataset is a configuration error, not a runtime error
- Consistent with existing resolver pattern (MetadataStore returns Result)
- Enables clear error messages before expensive data loading
- Prevents partial execution of operation pipeline

**Error handling strategy**:
```rust
// During operation planning (before execution)
pub enum AppendError {
    DatasetNotFound { dataset_id: Uuid },
    DatasetVersionNotFound { dataset_id: Uuid, version: i32 },
    ColumnMismatch { extra_columns: Vec<String> },
    ExpressionParseError { expression: String, error: String },
    AggregationError { message: String },
}

// In planning phase
fn validate_append_operation(
    metadata_store: &MetadataStore,
    dataset_ref: &DatasetRef,
) -> Result<(), AppendError> {
    // Check dataset exists
    let dataset = metadata_store
        .get_dataset(dataset_ref.dataset_id)
        .ok_or(AppendError::DatasetNotFound { 
            dataset_id: dataset_ref.dataset_id 
        })?;
    
    // Check version if pinned
    if let Some(version) = dataset_ref.dataset_version {
        if dataset.version != version {
            return Err(AppendError::DatasetVersionNotFound {
                dataset_id: dataset_ref.dataset_id,
                version,
            });
        }
    }
    
    Ok(())
}
```

**Error locations**:
1. **Planning phase**: Dataset existence, version pinning, schema validation
2. **Execution phase**: Data loading failures, resolver errors, Polars errors
3. **Post-execution**: Schema alignment mismatches (extra columns)

**Alternatives considered**:
- **Runtime-only errors**: Rejected - wastes computation on invalid operations
- **Warning + skip**: Rejected - silent failures corrupt analysis
- **Fallback dataset**: Rejected - adds complexity, unclear semantics

---

## 4. Temporal Filtering Implementation

### Decision: Reuse existing TemporalMode pattern with DataLoader integration

**Rationale**:
- Dataset already has `temporal_mode: Option<TemporalMode>` (period/bitemporal)
- Spec requires: "filter by _period = run_period.identifier" for period mode
- Existing DataLoader likely implements temporal filtering (need to verify in code)
- Consistency with RuntimeJoin temporal behavior

**Implementation approach**:
```rust
// Temporal filtering logic (in DataLoader or operation execution)
fn apply_temporal_filter(
    df: LazyFrame,
    temporal_mode: &Option<TemporalMode>,
    run_period: &str,
    as_of_date: Option<&str>,
) -> Result<LazyFrame> {
    match temporal_mode {
        Some(TemporalMode::Period) => {
            // Filter: _period = run_period.identifier
            Ok(df.filter(col("_period").eq(lit(run_period))))
        }
        Some(TemporalMode::Bitemporal) => {
            // Filter: valid_from <= asOf AND (valid_to > asOf OR valid_to IS NULL)
            let as_of = as_of_date.ok_or(anyhow!("asOf date required for bitemporal"))?;
            Ok(df.filter(
                col("valid_from").lt_eq(lit(as_of))
                    .and(col("valid_to").gt(lit(as_of)).or(col("valid_to").is_null()))
            ))
        }
        None => {
            // Snapshot mode: no filtering
            Ok(df)
        }
    }
}
```

**User Story 4 scenarios**:
- TS-16: Period mode → filter `_period = "2026-01"`
- TS-17: Bitemporal mode → filter `valid_from <= asOf AND valid_to > asOf`
- TS-18: Snapshot mode → no filtering (append all rows)

**Alternatives considered**:
- **Manual filtering in append operation**: Rejected - duplicates temporal logic
- **Pre-filtered dataset loading**: Preferred if DataLoader supports it
- **Post-load filtering**: Fallback if DataLoader doesn't filter

---

## 5. Column Alignment Best Practices

### Decision: Two-phase validation (fail fast on extra columns, fill missing with NULL)

**Rationale**:
- Spec requirement: "source columns must be subset of working dataset columns"
- Extra columns indicate schema mismatch → fail early
- Missing columns in source are expected → fill with NULL (e.g., budget rows missing `journal_id`)
- Polars `with_column()` efficiently adds NULL columns

**Implementation approach**:
```rust
use std::collections::HashSet;

fn align_appended_schema(
    appended_df: &DataFrame,
    working_schema: &[String],
) -> Result<DataFrame> {
    let appended_cols: HashSet<_> = appended_df
        .get_column_names()
        .iter()
        .map(|s| s.to_string())
        .collect();
    
    // Phase 1: Validate no extra columns
    let extra_cols: Vec<_> = appended_cols
        .iter()
        .filter(|col| !working_schema.contains(col))
        .cloned()
        .collect();
    
    if !extra_cols.is_empty() {
        return Err(AppendError::ColumnMismatch { 
            extra_columns: extra_cols 
        }.into());
    }
    
    // Phase 2: Fill missing columns with NULL
    let mut result = appended_df.clone();
    for col_name in working_schema {
        if !appended_cols.contains(col_name) {
            // Add NULL column (type inferred from working schema)
            let null_col = Series::new_null(
                col_name.as_str(),
                result.height(),
            );
            result = result.with_column(null_col)?;
        }
    }
    
    // Phase 3: Reorder to match working schema
    result.select(working_schema)
}
```

**User Story 1 scenarios**:
- TS-02: Budget columns are subset → fill `journal_id`, `description` with NULL
- TS-03: Explicit validation of NULL filling for missing columns
- Edge case: Extra columns in source → error immediately

**Alternatives considered**:
- **Ignore extra columns**: Rejected - silently drops data (dangerous)
- **Auto-add extra columns to working dataset**: Rejected - mutates working schema unexpectedly
- **Row-by-row alignment**: Rejected - inefficient for 100k rows

---

## 6. System Column Generation

### Decision: Generate _row_id with UUID v7, set metadata columns at append time

**Rationale**:
- Spec: "System MUST generate unique _row_id values for all appended rows"
- UUID v7 provides time-ordered uniqueness (aligns with _operation_seq)
- System columns: `_row_id`, `_source_dataset`, `_operation_seq`, `_deleted`
- Generated during append execution (after aggregation, before DataFrame concat)

**Implementation approach**:
```rust
use uuid::Uuid;

fn add_system_columns(
    df: &DataFrame,
    source_dataset_id: Uuid,
    operation_seq: u32,
) -> Result<DataFrame> {
    let row_count = df.height();
    
    // Generate UUID v7 for each row
    let row_ids: Vec<String> = (0..row_count)
        .map(|_| Uuid::now_v7().to_string())
        .collect();
    
    df.clone()
        .lazy()
        .with_column(col("_row_id").fill(lit(row_ids)))
        .with_column(col("_source_dataset").fill(lit(source_dataset_id.to_string())))
        .with_column(col("_operation_seq").fill(lit(operation_seq)))
        .with_column(col("_deleted").fill(lit(false)))
        .collect()
}
```

**System column semantics**:
- `_row_id`: UUID v7 (unique identifier for this row instance)
- `_source_dataset`: UUID of source dataset (for audit trail)
- `_operation_seq`: Operation sequence number in pipeline
- `_deleted`: Boolean flag (false for appended rows)

**Alternatives considered**:
- **Integer _row_id**: Rejected - requires global counter, hard to shard
- **UUID v4**: Rejected - not time-ordered (worse for indexing)
- **Omit system columns**: Rejected - spec requires them

---

## 7. Source Selector Expression Evaluation

### Decision: Reuse existing Expression struct, evaluate with Polars filter()

**Rationale**:
- Spec: "source_selector expression evaluated against source dataset columns"
- Existing `Expression { source: String }` in expression.rs
- Polars supports filter expressions: `col("amount").gt(lit(10000))`
- Apply filter BEFORE aggregation (per spec FR-006)

**Implementation approach**:
```rust
// Parse source_selector into Polars expression
fn parse_source_selector(selector: &str) -> Result<Expr> {
    // For MVP: support simple comparisons
    // Future: full expression parser
    
    // Example: "budget_type = 'original'"
    // Example: "amount > 10000"
    
    // Use existing expression parsing if available
    // Or implement simple parser for MVP
    
    // Polars expression: col("budget_type").eq(lit("original"))
    parse_filter_expression(selector)
}

// Apply during data loading
fn load_and_filter_source(
    data_loader: &DataLoader,
    dataset_id: Uuid,
    source_selector: Option<&Expression>,
) -> Result<DataFrame> {
    let df = data_loader.load_dataset(dataset_id)?;
    
    if let Some(selector) = source_selector {
        let filter_expr = parse_source_selector(&selector.source)?;
        return df.lazy().filter(filter_expr).collect();
    }
    
    Ok(df)
}
```

**User Story 2 scenarios**:
- TS-06: `source_selector: "budget_type = 'original'"` → filter 4 of 12 rows
- TS-07: `source_selector: "amount > 10000"` → numeric comparison
- TS-08: Highly selective filter (5 of 100 rows) → verify count

**Alternatives considered**:
- **SQL WHERE clause**: Rejected - requires SQL parser
- **Row-by-row evaluation**: Rejected - inefficient (not vectorized)
- **Predefined filter library**: Deferred - start with simple expressions

---

## 8. Data Loading & Resolver Integration

### Decision: Use existing MetadataStore + DataLoader pattern with resolver precedence

**Rationale**:
- Spec FR-001: "Same Resolver precedence as RuntimeJoin"
- Resolver precedence: Project overrides → Dataset resolver_id → system default
- Existing `Dataset.resolver_id: Option<String>` field
- Matches codebase pattern (avoid reimplementation)

**Implementation approach**:
```rust
fn resolve_and_load_source(
    metadata_store: &MetadataStore,
    data_loader: &DataLoader,
    dataset_ref: &DatasetRef,
    project_resolver_overrides: &HashMap<String, String>,
) -> Result<DataFrame> {
    // 1. Load dataset metadata
    let dataset = metadata_store.get_dataset(dataset_ref.dataset_id)?;
    
    // 2. Resolve data source (following precedence)
    let resolver_id = project_resolver_overrides
        .get(&dataset_ref.dataset_id.to_string())
        .or(dataset.resolver_id.as_ref())
        .ok_or(anyhow!("No resolver configured"))?;
    
    // 3. Load data via DataLoader
    let df = data_loader.load_dataset_with_resolver(
        dataset_ref.dataset_id,
        resolver_id,
    )?;
    
    Ok(df)
}
```

**Alternatives considered**:
- **Direct dataset loading**: Rejected - bypasses resolver pattern
- **Separate resolver precedence**: Rejected - violates consistency requirement
- **Hardcoded resolver**: Rejected - inflexible

---

## 9. Zero-Row Append Handling

### Decision: Successful no-op when source_selector matches zero rows

**Rationale**:
- Spec edge case: "Append operation succeeds with zero rows appended"
- Not an error condition (filter legitimately matches nothing)
- Working dataset remains unchanged
- Return success status with zero-row metadata

**Implementation approach**:
```rust
fn execute_append(
    working_df: &DataFrame,
    source_df: &DataFrame,
    // ... other params
) -> Result<AppendResult> {
    // After filtering/aggregation
    if source_df.height() == 0 {
        return Ok(AppendResult {
            success: true,
            rows_appended: 0,
            result_df: working_df.clone(),
        });
    }
    
    // Normal append logic
    // ...
}
```

**User Story 2 scenario**:
- Edge case: "zero rows matching source_selector" → success, 0 appended

**Alternatives considered**:
- **Warning message**: Could add warning log, but success is correct semantics
- **Error on zero rows**: Rejected - too strict, breaks filter workflows

---

## 10. Test Strategy

### Decision: Three-tier test structure (contract, integration, unit)

**Rationale**:
- Constitution Principle I: TDD mandatory
- Existing test structure: `tests/contracts/`, `tests/integration/`, unit tests inline
- Contract tests verify operation behavior against spec requirements
- Integration tests demonstrate end-to-end user scenarios
- Unit tests cover component-level logic (expression parsing, schema alignment)

**Test coverage plan**:

**Contract tests** (`tests/contracts/us009_append_operation.rs`):
- Append operation deserializes from YAML/JSON
- Append parameters validate correctly
- Operation execution signature matches engine expectations

**Integration tests** (`tests/integration/us009_append_scenarios.rs`):
- User Story 1: Basic budget vs actual append (TS-01, TS-02, TS-03)
- User Story 2: Filtered source data append (TS-06, TS-07, TS-08)
- User Story 3: Aggregated data append (TS-13, TS-14, TS-15)
- User Story 4: Period-filtered append (TS-16, TS-17, TS-18)
- Edge cases: zero rows, missing dataset, column mismatch

**Unit tests** (inline in implementation files):
- Expression parsing (source_selector, aggregations)
- Schema alignment logic (NULL filling, extra column detection)
- System column generation (UUID v7, metadata)
- Temporal filtering logic (period/bitemporal/snapshot)

**Performance benchmarks**:
- 10k rows: <10ms (simple append)
- 100k rows: <50ms (with aggregation)
- Measured with `cargo bench` (criterion.rs if added)

---

## Implementation Roadmap

### Phase 1: Core Data Structures (TDD)
1. Define AppendAggregation, Aggregation structs in `model/operation.rs`
2. Add DatasetRef struct (dataset_id, optional dataset_version)
3. Contract tests: deserialize append operation config
4. Unit tests: struct validation

### Phase 2: Expression Parsing (TDD)
1. Implement parse_aggregation() for SUM/COUNT/AVG/MIN/MAX
2. Implement parse_source_selector() for simple filters
3. Unit tests: parse various expression formats
4. Unit tests: error handling for invalid expressions

### Phase 3: Data Loading & Filtering (TDD)
1. Implement resolve_and_load_source() with resolver precedence
2. Implement temporal filtering (period/bitemporal/snapshot)
3. Implement source_selector filtering
4. Integration tests: load datasets, apply filters

### Phase 4: Aggregation Logic (TDD)
1. Implement build_agg_expressions() from AppendAggregation
2. Implement group_by + aggregation execution
3. Unit tests: aggregation correctness
4. Integration tests: User Story 3 scenarios (TS-13, TS-14, TS-15)

### Phase 5: Schema Alignment (TDD)
1. Implement align_appended_schema() with validation
2. Implement NULL filling for missing columns
3. Unit tests: column alignment edge cases
4. Integration tests: User Story 1 scenarios (TS-02, TS-03)

### Phase 6: System Columns & Concatenation (TDD)
1. Implement add_system_columns() with UUID v7
2. Implement DataFrame concatenation
3. Integration tests: verify system columns populated
4. Integration tests: verify row counts correct

### Phase 7: End-to-End Integration (TDD)
1. Wire all components into operation execution pipeline
2. Integration tests: all user stories (TS-01 through TS-18)
3. Edge case tests: zero rows, missing dataset, errors
4. Performance benchmarks: verify <50ms target

---

## Dependencies & Prerequisites

### Required Crates (already in workspace)
- `polars = "0.46"` ✓ (DataFrame processing)
- `uuid = { version = "1", features = ["v7"] }` ✓ (row ID generation)
- `serde` ✓ (serialization)
- `anyhow` / `thiserror` ✓ (error handling)

### Existing Code to Leverage
- `model/dataset.rs`: Dataset, TemporalMode, TableRef
- `model/expression.rs`: Expression struct
- `model/operation.rs`: OperationKind enum (Append variant exists)
- `resolver/`: MetadataStore, DataLoader patterns (assume exists)
- `engine/`: Operation execution framework

### Assumed Interfaces (to verify)
```rust
trait DataLoader {
    fn load_dataset(&self, id: Uuid) -> Result<DataFrame>;
    fn load_dataset_with_resolver(&self, id: Uuid, resolver: &str) -> Result<DataFrame>;
}

trait MetadataStore {
    fn get_dataset(&self, id: Uuid) -> Result<Dataset>;
}
```

---

## Risk Assessment

### Low Risk
- ✓ Polars performance well-documented (proven at scale)
- ✓ Existing resolver pattern established
- ✓ Test infrastructure in place
- ✓ Clear spec requirements (no ambiguity)

### Medium Risk
- ⚠️ Expression parsing: Simple MVP sufficient, full parser deferred
- ⚠️ Temporal filtering: Assumes existing implementation (need to verify)
- ⚠️ DataLoader interface: Assumed from codebase patterns (need to verify)

### Mitigation Strategies
- **Expression parsing**: Start with simple regex-based parser, extend later
- **Temporal filtering**: If missing, implement based on TemporalMode enum
- **DataLoader**: Verify interface exists, otherwise implement minimal version

---

## Open Questions (for implementation phase)

1. **DataLoader interface**: Does it already support temporal filtering?
   - Action: Check `crates/core/src/engine/` or `resolver/` for existing implementation
   
2. **Expression parser**: Is there existing expression parsing beyond `Expression.source`?
   - Action: Check `dsl/` module for reusable parsing logic
   
3. **System columns**: Are there existing utilities for _row_id generation?
   - Action: Search codebase for UUID v7 usage patterns

4. **Error types**: Is there a standard error enum for operation failures?
   - Action: Check `engine/types.rs` for error patterns

(These will be resolved during implementation via code exploration)

---

## References

- **Feature Spec**: `/workspace/specs/009-append-operation/spec.md`
- **Polars Documentation**: https://docs.rs/polars/0.46
- **UUID v7 Spec**: RFC 9562 (time-ordered UUID)
- **Existing Dataset Model**: `/workspace/crates/core/src/model/dataset.rs`
- **Constitution**: `/workspace/.specify/memory/constitution.md` (TDD Principle I)

**Research completed**: 2026-02-22  
**Next phase**: Phase 1 - Design & Contracts (data-model.md, contracts/, quickstart.md)
