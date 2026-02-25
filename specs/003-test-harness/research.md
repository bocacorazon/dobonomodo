# Research: Test Harness

**Feature**: 003-test-harness  
**Date**: 2026-02-22  
**Phase**: 0 (Research & Resolution)

## Overview

This research document consolidates technical decisions and best practices for implementing the test harness. All NEEDS CLARIFICATION items from the Technical Context have been resolved through research of existing codebase patterns, Polars documentation, and Rust best practices.

---

## Research Area 1: Polars DataFrame Comparison

**Decision**: Use `polars-testing` crate's `assert_dataframe_equal!` macro with `DataFrameEqualOptions`

**Rationale**:
- Polars provides native testing infrastructure optimized for DataFrame comparison
- Collects ALL mismatches without early exit (requirement FR-011)
- Supports order-insensitive comparison via `.with_check_row_order(false)`
- Handles null values, NaN, and special floats correctly
- Provides detailed error messages with row indices and column names
- Performance: O(n log n) for unordered comparison, O(n) for ordered

**Alternatives Considered**:
1. **Manual row-by-row iteration**: Rejected because it's slower, error-prone, and requires custom sorting logic for order-insensitive comparison
2. **Convert to JSON and use text diff**: Rejected because it loses type information and provides poor error messages
3. **Third-party diff libraries (similar-asserts, pretty_assertions)**: Rejected because they're not optimized for columnar data and lack DataFrame-specific features

**Implementation Pattern**:
```rust
use polars_testing::{assert_dataframe_equal, asserts::DataFrameEqualOptions};

// For exact match mode with order-insensitive comparison
let options = DataFrameEqualOptions::default()
    .with_check_row_order(false)
    .with_check_exact(false)  // Use tolerance for floats
    .with_rel_tol(1e-5)
    .with_abs_tol(1e-8);

// Collect mismatches via Result type
match std::panic::catch_unwind(|| {
    assert_dataframe_equal!(&actual, &expected, options);
}) {
    Ok(_) => TestResult::pass(),
    Err(e) => TestResult::fail_with_mismatches(parse_panic_message(e)),
}
```

**Dependencies**: Add `polars-testing` to `cli` and `test-resolver` Cargo.toml

---

## Research Area 2: System Metadata Injection

**Decision**: Generate UUID v7 using `Uuid::now_v7()` for `_row_id`, inject timestamps using `chrono::Utc::now()`

**Rationale**:
- UUID v7 provides time-ordered IDs, improving database index performance when tests run against real databases later
- Thread-safe by design (uses internal monotonic counter)
- Performance: ~100 ns/UUID, easily handles 10k rows without batching
- Already in workspace dependencies: `uuid = { version = "1", features = ["serde", "v7"] }`
- Chrono already in use for timestamp fields in existing models (Run, Dataset)

**Alternatives Considered**:
1. **UUID v4 (random)**: Rejected because v7's time-ordering benefits indexing and natural sorting
2. **Sequential integers for test IDs**: Rejected because it doesn't match production behavior (production uses UUIDs)
3. **Manual timestamp generation with std::time::SystemTime**: Rejected because chrono provides better serialization and timezone support

**Implementation Pattern**:
```rust
use uuid::Uuid;
use chrono::Utc;

pub fn inject_metadata(
    rows: Vec<HashMap<String, Value>>,
    table_name: &str,
    temporal_mode: TemporalMode,
    dataset_id: Uuid,
) -> Vec<HashMap<String, Value>> {
    let now = Utc::now();
    
    rows.into_iter().map(|mut row| {
        row.insert("_row_id".to_string(), Uuid::now_v7().to_string().into());
        row.insert("_deleted".to_string(), false.into());
        row.insert("_created_at".to_string(), now.to_rfc3339().into());
        row.insert("_updated_at".to_string(), now.to_rfc3339().into());
        row.insert("_source_dataset_id".to_string(), dataset_id.to_string().into());
        row.insert("_source_table".to_string(), table_name.into());
        
        // Temporal columns based on temporal_mode
        match temporal_mode {
            TemporalMode::Period => {
                // _period already in row (user-provided)
            }
            TemporalMode::Bitemporal => {
                // _period_from, _period_to already in row (user-provided)
            }
            TemporalMode::NonTemporal => {
                // No temporal columns
            }
        }
        
        row
    }).collect()
}
```

---

## Research Area 3: YAML Scenario Parsing

**Decision**: Use `serde_yaml` with `anyhow::Context` for error handling, add `serde_path_to_error` for precise field location reporting

**Rationale**:
- `serde_yaml` already in workspace dependencies
- Existing codebase uses serde extensively for Dataset, Project, Operation models
- `anyhow::Context` provides error chains with user-friendly messages
- `serde_path_to_error` adds field path to error messages (e.g., "Failed at 'input.data.orders.rows[2].amount'")
- Two-phase validation (serde structural + custom semantic) separates syntax errors from logic errors

**Alternatives Considered**:
1. **Manual YAML parsing with yaml-rust**: Rejected because it requires manual type conversion and is more error-prone
2. **JSON as scenario format**: Rejected because YAML is more human-friendly for test scenarios and already used in docs/architecture samples
3. **Custom DSL parser**: Rejected as overengineering; serde handles complex nested structures well

**Implementation Pattern**:
```rust
use anyhow::{Context, Result};
use serde::{Deserialize, Serialize};

#[derive(Debug, Serialize, Deserialize)]
pub struct TestScenario {
    pub name: String,
    pub periods: Vec<PeriodDef>,
    pub input: TestInput,
    pub project: ProjectDef,
    pub expected_output: TestOutput,
    #[serde(default)]
    pub config: TestConfig,
}

impl TestScenario {
    pub fn from_yaml_file(path: &Path) -> Result<Self> {
        let content = std::fs::read_to_string(path)
            .with_context(|| format!("Failed to read {}", path.display()))?;
        
        let scenario: Self = serde_yaml::from_str(&content)
            .with_context(|| format!("Invalid YAML in {}", path.display()))?;
        
        scenario.validate()?;
        Ok(scenario)
    }
    
    fn validate(&self) -> Result<()> {
        // Post-deserialization validation (DataBlock one-of constraint, etc.)
        for (name, block) in &self.input.data {
            match (&block.rows, &block.file) {
                (Some(_), Some(_)) => bail!("DataBlock '{}': cannot have both rows and file", name),
                (None, None) => bail!("DataBlock '{}': must have either rows or file", name),
                _ => {}
            }
        }
        Ok(())
    }
}
```

**Dependencies**: Consider adding `serde_path_to_error = "0.1"` for enhanced error reporting (optional)

---

## Research Area 4: Test Isolation Patterns

**Decision**: Implement in-memory IO trait adapters (`InMemoryDataLoader`, `InMemoryMetadataStore`, `InMemoryTraceWriter`) in `test-resolver` crate

**Rationale**:
- Follows existing architecture: core defines IO traits, implementations live in separate crates
- In-memory adapters prevent test pollution (no writes to production stores)
- Existing pattern in codebase: `DataLoader`, `MetadataStore`, `TraceWriter` traits defined in core
- Enables test scenarios to run without any external infrastructure (databases, S3, etc.)
- Fast execution: in-memory operations are 1000x faster than disk/network IO

**Alternatives Considered**:
1. **Mock framework (mockall crate)**: Rejected because in-memory implementations are simpler and provide better debugging
2. **SQLite in-memory database**: Rejected because it's heavier than HashMap-based storage and requires SQL schema
3. **Temporary directories with file-based storage**: Rejected because cleanup is error-prone and slower

**Implementation Pattern**:
```rust
// In test-resolver/src/loader.rs
use std::collections::HashMap;
use polars::prelude::*;
use core::{DataLoader, ResolvedLocation};

pub struct InMemoryDataLoader {
    data: HashMap<String, LazyFrame>,  // table_name -> data
}

impl DataLoader for InMemoryDataLoader {
    fn load(&self, location: &ResolvedLocation, schema: &TableSchema) -> Result<LazyFrame> {
        self.data.get(&location.table_name)
            .cloned()
            .ok_or_else(|| anyhow!("Table '{}' not found in test data", location.table_name))
    }
}

// In test-resolver/src/metadata.rs
pub struct InMemoryMetadataStore {
    datasets: HashMap<Uuid, Dataset>,
    projects: HashMap<Uuid, Project>,
    runs: HashMap<Uuid, Run>,
}

impl MetadataStore for InMemoryMetadataStore {
    fn get_dataset(&self, id: &Uuid, version: Option<i32>) -> Result<Dataset> {
        self.datasets.get(id)
            .cloned()
            .ok_or_else(|| anyhow!("Dataset {} not found", id))
    }
    // ... other trait methods
}
```

---

## Research Area 5: Match Mode Implementation

**Decision**: Implement exact mode using `assert_dataframe_equal!` with `check_row_order=false`, subset mode using set difference operations

**Rationale**:
- Exact mode: Native Polars support via `DataFrameEqualOptions` handles all edge cases (nulls, NaN, order)
- Subset mode: Use DataFrame join/anti-join to find missing expected rows efficiently
- Both modes leverage Polars' optimized operations instead of manual row iteration
- Performance: O(n log n) for both modes due to internal sorting

**Alternatives Considered**:
1. **Row-by-row iteration with HashSet**: Rejected because it's slower and requires custom hash implementation for rows
2. **SQL-style EXCEPT operation**: Rejected because Polars doesn't have native EXCEPT, would require custom implementation
3. **Convert to sets of JSON strings**: Rejected because it loses type precision and is inefficient

**Implementation Pattern**:
```rust
pub enum MatchMode {
    Exact,   // All rows must match, no extras
    Subset,  // Expected rows must exist, extra actual rows tolerated
}

pub fn compare_output(
    actual: &DataFrame,
    expected: &DataFrame,
    mode: MatchMode,
) -> Result<Vec<DataMismatch>> {
    let mut mismatches = Vec::new();
    
    match mode {
        MatchMode::Exact => {
            // Use Polars testing with order-insensitive comparison
            let options = DataFrameEqualOptions::default()
                .with_check_row_order(false);
            
            match std::panic::catch_unwind(|| {
                assert_dataframe_equal!(&actual, &expected, options);
            }) {
                Ok(_) => Ok(vec![]),  // No mismatches
                Err(e) => parse_polars_error(e),
            }
        }
        MatchMode::Subset => {
            // Find missing expected rows: expected LEFT ANTI JOIN actual
            let missing = expected.left_anti_join(actual, &expected.columns())?;
            for row in missing.iter() {
                mismatches.push(DataMismatch::MissingRow { expected: row });
            }
            Ok(mismatches)
        }
    }
}
```

---

## Research Area 6: Pipeline Execution Stubbing

**Decision**: Create passthrough mock in `cli/src/harness/executor.rs` that returns input data unchanged until S10 (core::engine) is implemented

**Rationale**:
- Unblocks test harness development without waiting for S10
- Passthrough behavior is sufficient to validate metadata injection, comparison logic, and CLI integration
- Easy to swap for real pipeline executor later (single function call site)
- Enables self-testing: passthrough scenario from spec validates entire harness

**Alternatives Considered**:
1. **Wait for S10 to start test harness**: Rejected because it creates dependency blocking
2. **Complex mock with operation simulation**: Rejected as overengineering; passthrough is sufficient
3. **No-op executor that returns empty DataFrame**: Rejected because it can't validate comparison logic

**Implementation Pattern**:
```rust
// Temporary mock in cli/src/harness/executor.rs
pub fn execute_pipeline_mock(
    input: LazyFrame,
    project: &Project,
) -> Result<DataFrame> {
    // Passthrough: return input unchanged
    // TODO: Replace with core::engine::execute_pipeline when S10 is complete
    input.collect()
}

// Later, when S10 is ready:
pub fn execute_pipeline(
    input: LazyFrame,
    project: &Project,
) -> Result<DataFrame> {
    core::engine::execute_pipeline(input, project)
}
```

---

## Research Area 7: CLI Suite Discovery

**Decision**: Use `glob` or `walkdir` crate to discover `tests/scenarios/**/*.yaml` files, default to this convention with CLI override

**Rationale**:
- Convention-over-configuration reduces user friction
- `glob` crate provides pattern matching already used in Rust ecosystem
- Recursive directory traversal handles nested organization
- CLI override (`--suite <dir>` or explicit file paths) provides flexibility

**Alternatives Considered**:
1. **Hardcode single directory**: Rejected because it prevents logical test organization
2. **Require explicit test list file**: Rejected as less user-friendly
3. **Use cargo test integration**: Rejected because test scenarios are data-driven, not Rust test functions

**Implementation Pattern**:
```rust
use std::path::{Path, PathBuf};

pub fn discover_scenarios(suite_path: Option<&Path>) -> Result<Vec<PathBuf>> {
    let base_dir = suite_path.unwrap_or_else(|| Path::new("tests/scenarios"));
    
    let mut scenarios = Vec::new();
    for entry in walkdir::WalkDir::new(base_dir)
        .into_iter()
        .filter_map(Result::ok)
        .filter(|e| e.path().extension().map_or(false, |ext| ext == "yaml" || ext == "yml"))
    {
        scenarios.push(entry.into_path());
    }
    
    scenarios.sort();  // Deterministic order
    Ok(scenarios)
}
```

**Dependencies**: Add `walkdir = "2"` to `cli` Cargo.toml

---

## Summary of Decisions

| Area | Decision | Key Rationale |
|------|----------|---------------|
| DataFrame comparison | `polars-testing::assert_dataframe_equal!` | Native support, collects all mismatches, order-insensitive |
| UUID generation | `Uuid::now_v7()` | Time-ordered, thread-safe, fast |
| YAML parsing | `serde_yaml` + `anyhow::Context` | Already in use, two-phase validation |
| Test isolation | In-memory trait implementations | No side effects, fast, simple |
| Match modes | Polars testing (exact), anti-join (subset) | Leverages optimized operations |
| Pipeline stub | Passthrough mock | Unblocks development, easy to replace |
| Suite discovery | `walkdir` with `**/*.yaml` convention | User-friendly, flexible |

---

## Unresolved Items

None. All technical context items have been researched and decided.

---

## Dependencies to Add

```toml
# In cli/Cargo.toml
[dependencies]
polars-testing = "0.46"
walkdir = "2"

# Optional enhancement
serde_path_to_error = "0.1"
```

---

## Next Phase

Proceed to Phase 1: Design (data-model.md, contracts/, quickstart.md).
