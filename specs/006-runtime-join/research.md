# Research: Runtime Join Resolution

**Feature**: 006-runtime-join  
**Created**: 2026-02-22  
**Purpose**: Resolve technical unknowns and establish implementation approach for RuntimeJoin

---

## Research Tasks

### R1: Polars LazyFrame Join API

**Question**: How do we perform left joins between Polars LazyFrames with custom join expressions, and how do we alias joined columns to make them available under a prefix?

**Decision**: Use Polars `LazyFrame::join()` method with `JoinType::Left`, join on compiled column expressions, and use `with_suffix()` to prefix joined columns with the alias.

**Rationale**: 
- Polars 0.46 provides native lazy join support via `LazyFrame::join(other, left_on, right_on, how)` where `how` can be `JoinType::Left`
- The `join()` method accepts expressions for both `left_on` and `right_on`, supporting custom join conditions
- `with_suffix()` (or `suffix()` parameter in join args) automatically prefixes all columns from the right-hand LazyFrame with a string (e.g., `"_fx"` -> `rate_fx`)
- After join, we can use `select()` or `with_columns()` to rename prefixed columns to the desired `alias.column` format
- Alternative considered: Manual column renaming before join - rejected because Polars suffix handling is more efficient and less error-prone

**Alternatives considered**:
- **Manual pre-join renaming**: Rename all join table columns to `alias_column` format before join. Rejected: more verbose, risks name collisions, less idiomatic.
- **Post-join column aliasing**: Join without suffix, manually rename afterward. Rejected: Polars suffix is cleaner and handles name collisions automatically.

**Implementation notes**:
- Compile the `on` expression from the RuntimeJoin to a Polars boolean expression
- Apply suffix matching the alias (e.g., for `alias: "fx"`, use `suffix: "_fx"`)
- In assignment expression compilation, map references like `fx.rate` to `rate_fx`
- Handle multi-column join conditions by ANDing multiple `col(left) == col(right)` expressions

---

### R2: Period Filter Integration with Temporal Mode

**Question**: How do we apply different period filtering logic (exact match for period tables, asOf range query for bitemporal tables) to the join LazyFrame before joining?

**Decision**: Check the join TableRef's `temporal_mode`, then call the existing S03 period filter module with the Run's current Period. For `temporal_mode: period`, filter `_period == period.identifier`. For `temporal_mode: bitemporal`, filter `_period_from <= period.start_date AND (_period_to IS NULL OR _period_to > period.start_date)`.

**Rationale**:
- The Period Filter (S03) already implements both filtering modes and is a documented dependency
- Each TableRef in a Dataset independently declares its `temporal_mode` (per BR-018 in dataset.md)
- The join Dataset's main table or lookup table may have a different `temporal_mode` than the working dataset
- By delegating to S03's filter function, we avoid duplicating temporal logic
- The filter is applied to the join LazyFrame immediately after loading and before the actual join operation

**Alternatives considered**:
- **Inline filtering**: Reimplement period/bitemporal filters directly in the join module. Rejected: violates DRY, increases test surface, risks divergence from S03.
- **Uniform filtering**: Apply only one filtering mode regardless of temporal_mode. Rejected: violates documented behavior in dataset.md BR-019/BR-020.

**Implementation notes**:
- Load join Dataset via DataLoader to get a LazyFrame
- Inspect the join TableRef's `temporal_mode` field (defaults to `Period` if omitted)
- Call `apply_period_filter(join_lf, temporal_mode, run_period)` from the S03 module
- Result is a filtered LazyFrame ready for joining

---

### R3: Resolver Precedence for Join Datasets

**Question**: How do we implement the resolver precedence chain (Project resolver_overrides -> Dataset resolver_id -> system default) for join datasets?

**Decision**: Implement a `resolve_with_precedence(project, dataset_id, period)` helper that checks Project.resolver_overrides for the dataset_id, falls back to the Dataset.resolver_id, and finally uses a system default Resolver instance. Return the resolved location and Resolver instance used.

**Rationale**:
- BR-008a in operation.md specifies this exact precedence chain
- Project-level resolver_overrides enable environment-specific data sources (e.g., test vs production)
- Dataset-level resolver_id allows per-dataset source configuration
- System default provides a fallback for datasets without specific configuration
- Precedence must be identical to input Dataset resolution for consistency

**Alternatives considered**:
- **Only Dataset resolver_id**: Ignore Project overrides. Rejected: prevents environment-specific testing, violates spec.
- **Hardcode test resolver**: Use InMemoryDataLoader for all joins during tests. Rejected: doesn't test production resolver path, limits flexibility.

**Implementation notes**:
- Input: Project (with resolver_overrides map), dataset_id, Period
- Step 1: Check if `project.resolver_overrides.get(dataset_id)` exists -> use that Resolver
- Step 2: Else, load Dataset by id, check `dataset.resolver_id` -> use that Resolver
- Step 3: Else, use system default Resolver (configured at engine initialization)
- Return: `(Resolver instance, resolved Location for each TableRef)`

---

### R4: Dataset Version Resolution

**Question**: When RuntimeJoin specifies `dataset_version`, how do we retrieve the exact version? When omitted, how do we resolve to "latest active"?

**Decision**: Query MetadataStore with `get_dataset(id, version)` when version is provided, or `get_latest_active_dataset(id)` when version is None. Capture the resolved version in the Run's ResolverSnapshot.

**Rationale**:
- BR-008c in operation.md specifies version pinning (exact) vs floating (latest)
- MetadataStore is the authoritative source for Dataset definitions
- Capturing resolved version in ResolverSnapshot ensures reproducibility (re-running a Run uses the same Dataset version, not a newer one)
- Version resolution must happen before Resolver lookup (we need the full Dataset definition to extract resolver_id and TableRefs)

**Alternatives considered**:
- **Always use latest**: Ignore version field. Rejected: breaks pinning use case, violates spec.
- **Snapshot entire Dataset in Run**: Copy full Dataset definition into Run metadata. Rejected: increases storage, complicates schema evolution.

**Implementation notes**:
- If `join.dataset_version.is_some()`: call `metadata_store.get_dataset(join.dataset_id, version)`
- Else: call `metadata_store.get_latest_active_dataset(join.dataset_id)`
- If not found or status is Disabled: return error before attempting data load
- Store `(dataset_id, resolved_version)` in `run.resolver_snapshot.join_datasets` (new field)

---

### R5: Expression Compilation for Join Column References

**Question**: How do we extend the DSL expression compiler to recognize `alias.column_name` references and map them to the suffixed Polars column names?

**Decision**: Extend the expression compiler's symbol table to include join aliases. When parsing a column reference, check if it's in the format `alias.column`. If the alias is in the active RuntimeJoin list, map it to the suffixed Polars column name (e.g., `fx.rate` -> `col("rate_fx")`).

**Rationale**:
- S01 DSL Parser is a documented dependency
- Assignment expressions in the update operation need to reference both working dataset columns and join columns
- Alias scoping is operation-level (not project-level), so the symbol table is scoped to the current operation
- Polars column references use `col("name")` syntax, so the mapping is straightforward

**Alternatives considered**:
- **Nested struct access**: Use Polars struct types to represent joins. Rejected: adds complexity, Polars struct syntax is less natural for tabular joins.
- **No alias support**: Require users to use suffixed names directly. Rejected: poor UX, violates spec requirement for `alias.column` syntax.

**Implementation notes**:
- When compiling an update operation, build a `join_aliases: HashMap<String, String>` mapping alias -> suffix (e.g., `"fx" -> "_fx"`)
- In expression compiler, when encountering a `ColumnRef`, check if it contains a dot
- If `parts[0]` is in `join_aliases`, map to `col(format!("{parts[1]}{}", suffix))`
- If not a join reference, treat as a working dataset column
- Compile-time error if alias is unknown or column doesn't exist in join table schema

---

### R6: Testing Strategy with InMemoryDataLoader

**Question**: How do we structure the TS-03 FX conversion integration test using InMemoryDataLoader?

**Decision**: Create a test fixture that seeds InMemoryDataLoader with sample GL transactions (10 rows, multiple currencies) and exchange_rates table (bitemporal, 9 rows with period ranges). Execute an update operation with RuntimeJoin on currency, assignment `amount_local * fx.rate`. Assert output contains expected converted amounts.

**Rationale**:
- TS-03 is the primary acceptance test for RuntimeJoin (defined in sample-datasets.md)
- InMemoryDataLoader is already available in the test-resolver crate
- Seeding in-memory tables allows full control over test data and expected outcomes
- Test validates end-to-end flow: dataset resolution, version selection, period filtering (bitemporal asOf), join execution, expression evaluation

**Alternatives considered**:
- **Mock all components**: Use mocks for Resolver, DataLoader, MetadataStore. Rejected: doesn't test integration, misses edge cases.
- **File-based test data**: Load from CSV/Parquet. Rejected: harder to maintain, slower, adds file IO dependency to tests.

**Implementation notes**:
- Test structure: `crates/core/tests/integration/ts03_fx_conversion.rs`
- Setup: Create InMemoryDataLoader, seed `transactions` table (10 rows), `accounts` table, `cost_centers`, `exchange_rates` (bitemporal)
- Period: `2026-01` (month, start_date: 2026-01-01)
- Expected: EUR rows use rate 1.0920, GBP use 1.2710, JPY use 0.00672, USD use 1.0000
- Assertions: Check `amount_reporting` for each `journal_id` matches expected conversion

---

## Best Practices

### Polars Lazy Execution

**Source**: Polars 0.46 documentation, Rust API best practices

**Practice**: Use LazyFrame operations wherever possible; defer `collect()` until output operation. Chain transformations using method chaining (join -> filter -> select -> with_columns).

**Application**: RuntimeJoin implementation will:
- Load join dataset as LazyFrame (not DataFrame)
- Apply period filter as lazy expression
- Perform join as lazy operation
- Return modified LazyFrame to update operation executor
- Only `collect()` when output operation writes results

**Benefit**: Reduces memory footprint, enables query optimization, improves performance for large datasets.

---

### Error Handling in Rust

**Source**: Rust error handling patterns, anyhow/thiserror crates

**Practice**: Use `thiserror` for domain errors (JoinResolutionError, DatasetNotFoundError), `anyhow` for application-level error propagation. Return `Result<T, E>` from all fallible functions. Provide context with `.context()`.

**Application**: RuntimeJoin resolution will define:
- `JoinError` enum with variants: DatasetNotFound, DatasetDisabled, UnknownColumn, ResolverFailed
- Each variant includes descriptive context (dataset_id, alias, column name)
- Functions return `Result<LazyFrame, JoinError>` or `anyhow::Result<T>`

**Benefit**: Clear error messages, typed error handling, easy propagation.

---

### Temporal Logic Isolation

**Source**: S03 Period Filter module, dataset.md BR-019/BR-020

**Practice**: Encapsulate period filtering logic in a dedicated module (`core::engine::period_filter`). Provide a single public function `apply_period_filter(lf, temporal_mode, period) -> LazyFrame` that handles both modes.

**Application**: RuntimeJoin will import and call this function rather than duplicating filter logic. If S03 is not yet implemented, stub it with a simple filter for the test (to be replaced with full S03 implementation).

**Benefit**: DRY, single source of truth for temporal logic, easier testing.

---

### Test Data Fixtures

**Source**: Rust testing best practices, sample-datasets.md

**Practice**: Define reusable test fixture functions that create and seed InMemoryDataLoader with standard datasets (GL transactions, exchange rates, budgets). Use consistent sample data across all tests.

**Application**: Create `crates/core/tests/fixtures/sample_datasets.rs` with:
- `create_gl_dataset()` -> Dataset definition
- `seed_gl_data(loader, period)` -> Populate InMemoryDataLoader
- `create_fx_rates_bitemporal()` -> Exchange rates table with period ranges

**Benefit**: Test consistency, reduced duplication, easier maintenance.

---

## Summary

All technical unknowns have been resolved. Implementation approach:

1. **RuntimeJoin data structure**: Defined in `core/src/model/operation.rs` (embedded in UpdateOperation arguments)
2. **Join resolution**: New `core/src/engine/join.rs` module with `resolve_and_load_join()` function
3. **Resolver precedence**: Helper function checks Project overrides -> Dataset resolver_id -> system default
4. **Version resolution**: MetadataStore queries for exact or latest active version
5. **Period filtering**: Delegate to S03 module based on temporal_mode
6. **Polars join**: Use `LazyFrame::join()` with suffix, compile expressions to map `alias.column` -> suffixed names
7. **Testing**: Contract tests for schema, unit tests for resolution/filtering, integration test TS-03 with InMemoryDataLoader

No blocking issues. Ready to proceed to Phase 1 (data model and contracts).
