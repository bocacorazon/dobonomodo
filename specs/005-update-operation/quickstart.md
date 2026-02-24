# Quickstart: Update Operation

**Feature**: Update Operation (S04)  
**Date**: 2025-02-22  
**Audience**: Developers implementing or extending DobONoMoDo

---

## Overview

This quickstart guide shows how to implement and test the `update` operation in the DobONoMoDo computation engine. The update operation modifies column values on rows matching a selector expression.

**Prerequisites**:
- Rust 1.75+ installed
- Cargo workspace setup (already exists)
- S01 (DSL Parser) completed (for expression compilation)
- S02 (Test Harness) completed (for integration tests)

**Time to Complete**: ~2 hours (with TDD approach)

---

## Step 1: Set Up Module Structure

Create the new `ops` module in the `engine` directory:

```bash
cd /workspace/crates/core/src/engine

# Create ops directory and module file
mkdir -p ops
touch ops/mod.rs
touch ops/update.rs
```

Update `/workspace/crates/core/src/engine/mod.rs`:

```rust
pub mod io_traits;
pub mod types;
pub mod ops;  // NEW
```

Create `/workspace/crates/core/src/engine/ops/mod.rs`:

```rust
pub mod update;

pub use update::{execute_update, UpdateOperation, UpdateExecutionContext, Assignment};
```

---

## Step 2: Define Data Structures (TDD - Step 1: Red)

Create `/workspace/crates/core/src/engine/ops/update.rs`:

```rust
use anyhow::{anyhow, Context, Result};
use chrono::{DateTime, Utc};
use polars::prelude::*;
use serde::{Deserialize, Serialize};
use std::collections::HashMap;

/// Update operation configuration
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
pub struct UpdateOperation {
    /// Optional selector (supports {{NAME}} interpolation)
    #[serde(default)]
    pub selector: Option<String>,
    
    /// List of column assignments
    pub assignments: Vec<Assignment>,
}

/// Single column assignment
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
pub struct Assignment {
    /// Target column name
    pub column: String,
    
    /// Value expression
    pub expression: String,
}

/// Execution context for update operation
#[derive(Debug, Clone)]
pub struct UpdateExecutionContext {
    pub working_dataset: LazyFrame,
    pub selectors: HashMap<String, String>,
    pub run_timestamp: DateTime<Utc>,
}

/// Execute an update operation
pub fn execute_update(
    context: &UpdateExecutionContext,
    operation: &UpdateOperation,
) -> Result<LazyFrame> {
    todo!("Implement update operation")
}

// Helper functions (internal)
fn resolve_selector(selector: &str, selectors: &HashMap<String, String>) -> Result<String> {
    todo!("Implement selector resolution")
}

fn compile_selector(selector_expr: &str) -> Result<Expr> {
    todo!("Implement selector compilation")
}

fn compile_assignments(assignments: &[Assignment]) -> Result<Vec<Expr>> {
    todo!("Implement assignment compilation")
}
```

**Verify**: Run `cargo build` - should compile with warnings about unused code.

---

## Step 3: Write First Test (TDD - Step 1: Red)

Create `/workspace/crates/core/tests/unit/update_operation_test.rs`:

```rust
use dobo_core::engine::ops::{Assignment, UpdateExecutionContext, UpdateOperation, execute_update};
use polars::prelude::*;
use chrono::Utc;
use std::collections::HashMap;

#[test]
fn test_update_single_assignment_no_selector() {
    // Arrange
    let df = df![
        "id" => [1, 2, 3],
        "status" => ["active", "active", "inactive"],
        "_updated_at" => [1000i64, 2000i64, 3000i64],
    ].unwrap().lazy();

    let context = UpdateExecutionContext {
        working_dataset: df,
        selectors: HashMap::new(),
        run_timestamp: Utc::now(),
    };

    let operation = UpdateOperation {
        selector: None,  // All rows
        assignments: vec![
            Assignment {
                column: "status".to_string(),
                expression: "\"processed\"".to_string(),
            },
        ],
    };

    // Act
    let result = execute_update(&context, &operation).unwrap();
    let result_df = result.collect().unwrap();

    // Assert
    let status_col = result_df.column("status").unwrap();
    let status_values: Vec<&str> = status_col.utf8().unwrap().into_iter().map(|v| v.unwrap()).collect();
    assert_eq!(status_values, vec!["processed", "processed", "processed"]);
}
```

Add to `/workspace/crates/core/tests/unit/mod.rs` (create if doesn't exist):

```rust
mod update_operation_test;
```

**Verify**: Run `cargo test test_update_single_assignment_no_selector` - should FAIL (Red phase).

---

## Step 4: Implement Minimal Code (TDD - Step 2: Green)

This is a simplified example. Full implementation requires S01 (DSL Parser) integration.

Update `/workspace/crates/core/src/engine/ops/update.rs`:

```rust
pub fn execute_update(
    context: &UpdateExecutionContext,
    operation: &UpdateOperation,
) -> Result<LazyFrame> {
    // 1. Validate assignments
    if operation.assignments.is_empty() {
        return Err(anyhow!("Update operation requires at least one assignment"));
    }

    // 2. Resolve and compile selector
    let selector_expr = match &operation.selector {
        Some(sel) => {
            let resolved = resolve_selector(sel, &context.selectors)?;
            compile_selector(&resolved)?
        }
        None => lit(true), // All rows
    };

    // 3. Compile assignments
    let assignment_exprs = compile_assignments(&operation.assignments)?;

    // 4. Apply update
    let updated = context.working_dataset.clone()
        .filter(selector_expr)
        .with_columns(assignment_exprs)
        .with_column(lit(context.run_timestamp.timestamp()).alias("_updated_at"));

    Ok(updated)
}

fn resolve_selector(selector: &str, selectors: &HashMap<String, String>) -> Result<String> {
    // Check for {{NAME}} pattern
    if selector.starts_with("{{") && selector.ends_with("}}") {
        let name = &selector[2..selector.len()-2];
        selectors.get(name)
            .cloned()
            .ok_or_else(|| anyhow!("Selector '{}' not defined in Project", name))
    } else {
        Ok(selector.to_string())
    }
}

fn compile_selector(selector_expr: &str) -> Result<Expr> {
    // Simplified: For real implementation, use S01 DSL parser
    // This example hardcodes a simple parser for demo purposes
    parse_expression(selector_expr)
        .context("Failed to compile selector expression")
}

fn compile_assignments(assignments: &[Assignment]) -> Result<Vec<Expr>> {
    assignments.iter()
        .map(|a| {
            parse_expression(&a.expression)
                .map(|e| e.alias(&a.column))
                .with_context(|| format!("Failed to compile assignment for column '{}'", a.column))
        })
        .collect()
}

// Placeholder expression parser (replace with S01 integration)
fn parse_expression(expr: &str) -> Result<Expr> {
    // Simplified: only supports string literals for this example
    if expr.starts_with('"') && expr.ends_with('"') {
        Ok(lit(&expr[1..expr.len()-1]))
    } else {
        Err(anyhow!("Unsupported expression: {}", expr))
    }
}
```

**Verify**: Run `cargo test test_update_single_assignment_no_selector` - should PASS (Green phase).

---

## Step 5: Add More Tests (TDD - Continue Red-Green-Refactor)

Add tests for:

1. **Named selector resolution**:

```rust
#[test]
fn test_update_with_named_selector() {
    let df = df![
        "id" => [1, 2, 3],
        "status" => ["active", "active", "inactive"],
        "_updated_at" => [1000i64, 2000i64, 3000i64],
    ].unwrap().lazy();

    let mut selectors = HashMap::new();
    selectors.insert("active_rows".to_string(), "status = \"active\"".to_string());

    let context = UpdateExecutionContext {
        working_dataset: df,
        selectors,
        run_timestamp: Utc::now(),
    };

    let operation = UpdateOperation {
        selector: Some("{{active_rows}}".to_string()),
        assignments: vec![
            Assignment {
                column: "status".to_string(),
                expression: "\"processed\"".to_string(),
            },
        ],
    };

    let result = execute_update(&context, &operation).unwrap();
    // Assert: only 2 rows updated (active ones)
}
```

2. **Error: Undefined selector name**
3. **Error: Empty assignments**
4. **Multiple assignments**
5. **New column creation**

---

## Step 6: Integration with Test Harness

Create test scenario YAML files in `/workspace/crates/core/tests/integration/scenarios/`:

**ts03_fx_conversion.yaml** (simplified, without joins):

```yaml
name: "TS-03: FX Conversion (Update Operation)"
description: "Test update operation with currency conversion assignment"

dataset:
  columns: [order_id, amount_usd, amount_eur, _updated_at]
  rows:
    - [1, 100.0, null, 1000]
    - [2, 200.0, null, 2000]

operation:
  type: update
  selector: null
  arguments:
    assignments:
      - column: amount_eur
        expression: "amount_usd * 0.85"

expected_output:
  columns: [order_id, amount_usd, amount_eur, _updated_at]
  rows:
    - [1, 100.0, 85.0, <run_timestamp>]
    - [2, 200.0, 170.0, <run_timestamp>]
```

Run via test harness:

```bash
cargo test ts03_fx_conversion
```

---

## Step 7: Run Full Test Suite

Execute all tests:

```bash
# Unit tests
cargo test --package dobo-core --test update_operation_test

# Integration tests
cargo test --package dobo-core --test integration

# All tests
cargo test
```

**Expected**: All tests pass (Green phase).

---

## Step 8: Verify Quality Gates

Before committing:

```bash
# Lint
cargo clippy -- -D warnings

# Format
cargo fmt --check

# Build
cargo build --release

# Full test suite
cargo test --all
```

**Principle II**: All quality gates must pass.

---

## Common Issues & Solutions

### Issue: S01 (DSL Parser) Not Available

**Solution**: Implement a simple placeholder expression parser (as shown in Step 4) that supports:
- String literals: `"value"`
- Numeric literals: `123`, `45.67`
- Column references: `column_name`
- Basic operators: `+`, `-`, `*`, `/`

Replace with S01 integration once available.

### Issue: Test Fails with "Column not found"

**Solution**: Ensure test DataFrame includes all columns referenced in expressions. Add missing columns to test setup.

### Issue: Type Mismatch Error

**Solution**: Verify expression result type matches target column type. Use Polars type coercion if needed.

---

## Next Steps

After completing the update operation:

1. **S05**: Implement RuntimeJoin support for update operations
2. **S06**: Implement delete operation
3. **S07**: Implement aggregate operation
4. **Run full regression tests**: Ensure update operation doesn't break existing functionality

---

## Resources

- **Operation Entity Spec**: `/workspace/docs/entities/operation.md`
- **Feature Spec**: `/workspace/docs/specs/S04-update-operation/prompt.md`
- **Data Model**: `/workspace/specs/005-update-operation/data-model.md`
- **API Contract**: `/workspace/specs/005-update-operation/contracts/rust-api.md`
- **Polars Documentation**: https://pola-rs.github.io/polars/

---

## Summary

You've implemented the update operation using TDD:

✅ Module structure created  
✅ Data structures defined  
✅ Tests written FIRST (Red)  
✅ Minimal implementation (Green)  
✅ Refactored and expanded  
✅ Integration tests passing  
✅ Quality gates verified  

**Principle I (TDD)**: Tests written before implementation ✓  
**Principle II (Quality Gates)**: All checks pass ✓  
**Principle IV (Comprehensive Testing)**: Unit + integration tests ✓
