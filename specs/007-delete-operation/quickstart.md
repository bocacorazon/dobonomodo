# Quickstart: Implementing Delete Operation

**Feature**: Delete Operation  
**Audience**: Developers implementing the delete operation  
**Date**: 2026-02-22

## Overview

This guide walks through implementing the delete operation for the DobONoMoDo pipeline engine. The delete operation marks matching rows as logically deleted using the `_deleted` metadata flag, with automatic exclusion from subsequent pipeline operations.

## Prerequisites

- Rust 2021 edition toolchain installed
- Familiarity with Polars DataFrame API (lazy evaluation)
- Understanding of DobONoMoDo operation execution model
- Read `research.md` and `data-model.md` in this directory

**Key concepts to understand first**:
- Selector evaluation: String expressions compiled to Polars `Expr`
- Operation sequencing: Operations execute in `order` field sequence
- Row metadata: `_deleted` boolean, `_modified_at` timestamp columns

## Implementation Checklist

### Phase 1: Core Delete Logic (TDD)

**Step 1.1: Write failing test for selector-based deletion**

Location: `crates/core/tests/unit/operations/test_delete.rs`

```rust
#[test]
fn test_delete_with_selector_marks_matching_rows() {
    // ARRANGE: Create DataFrame with 3 rows, 1 matching selector
    let df = df! {
        "_row_id" => &["r1", "r2", "r3"],
        "_deleted" => &[false, false, false],
        "_modified_at" => &[timestamp(1), timestamp(1), timestamp(1)],
        "amount" => &[0, 100, 200],
    }.unwrap().lazy();

    let params = DeleteOperationParams {
        selector: Some("amount = 0".to_string()),
    };

    // ACT: Execute delete operation
    let result = execute_delete(params, df).unwrap().collect().unwrap();

    // ASSERT: Only row with amount=0 is marked deleted
    assert_eq!(result.column("_deleted").unwrap().bool().unwrap().get(0), Some(true));
    assert_eq!(result.column("_deleted").unwrap().bool().unwrap().get(1), Some(false));
    assert_eq!(result.column("_deleted").unwrap().bool().unwrap().get(2), Some(false));
    
    // ASSERT: Modified timestamp updated for deleted row
    assert!(result.column("_modified_at").unwrap().datetime().unwrap().get(0).unwrap() > timestamp(1));
}
```

**Expected result**: Test fails (function not implemented)

---

**Step 1.2: Implement minimal delete execution**

Location: `crates/core/src/operations/delete.rs` (new file)

```rust
use polars::prelude::*;
use serde::{Deserialize, Serialize};
use anyhow::Result;
use chrono::Utc;

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
pub struct DeleteOperationParams {
    #[serde(default)]
    pub selector: Option<String>,
}

pub fn execute_delete(params: DeleteOperationParams, df: LazyFrame) -> Result<LazyFrame> {
    // Parse selector or default to "delete all"
    let selector_expr = match params.selector {
        Some(ref sel) if !sel.is_empty() => compile_selector(sel)?,
        _ => lit(true),  // No selector = match all active rows
    };
    
    let current_time = Utc::now().timestamp_millis();
    
    // Update _deleted flag for matching rows
    let df = df.with_column(
        when(selector_expr.clone())
            .then(lit(true))
            .otherwise(col("_deleted"))
            .alias("_deleted")
    );
    
    // Update _modified_at for matching rows
    let df = df.with_column(
        when(selector_expr)
            .then(lit(current_time))
            .otherwise(col("_modified_at"))
            .alias("_modified_at")
    );
    
    Ok(df)
}

fn compile_selector(selector: &str) -> Result<Expr> {
    // TODO: Integrate with existing selector parser
    // For now, simple literal expressions
    todo!("Integrate with DSL parser")
}
```

**Expected result**: Test compiles but still fails (selector compilation not implemented)

---

**Step 1.3: Integrate with existing selector parser**

```rust
use crate::dsl::parse_expression;  // Assuming existing parser
use crate::dsl::typecheck_boolean;

fn compile_selector(selector: &str) -> Result<Expr> {
    let ast = parse_expression(selector)?;
    typecheck_boolean(&ast)?;  // Ensure boolean result
    let polars_expr = ast.to_polars_expr()?;
    Ok(polars_expr)
}
```

**Expected result**: Test passes (green)

---

**Step 1.4: Refactor and add edge case tests**

```rust
#[test]
fn test_delete_without_selector_marks_all_active_rows() {
    let df = /* 3 active rows */;
    let params = DeleteOperationParams { selector: None };
    let result = execute_delete(params, df).unwrap().collect().unwrap();
    
    // All rows should be marked deleted
    assert!(result.column("_deleted").unwrap().bool().unwrap().all());
}

#[test]
fn test_delete_with_zero_matches_leaves_all_unchanged() {
    let df = /* no rows match selector */;
    let params = DeleteOperationParams {
        selector: Some("amount < 0".to_string()),  // No negative amounts
    };
    let result = execute_delete(params, df).unwrap().collect().unwrap();
    
    // No rows should be marked deleted
    assert!(result.column("_deleted").unwrap().bool().unwrap().none());
}

#[test]
fn test_delete_already_deleted_rows_no_op() {
    let df = df! {
        "_row_id" => &["r1"],
        "_deleted" => &[true],  // Already deleted
        "_modified_at" => &[timestamp(100)],
        "amount" => &[0],
    }.unwrap().lazy();
    
    let params = DeleteOperationParams {
        selector: Some("amount = 0".to_string()),
    };
    let result = execute_delete(params, df).unwrap().collect().unwrap();
    
    // Metadata should remain unchanged
    assert_eq!(result.column("_deleted").unwrap().bool().unwrap().get(0), Some(true));
    assert_eq!(result.column("_modified_at").unwrap().i64().unwrap().get(0), Some(timestamp(100)));
}
```

---

### Phase 2: Pipeline Integration (TDD)

**Step 2.1: Write integration test for multi-operation pipeline**

Location: `crates/core/tests/integration/test_pipeline_with_delete.rs`

```rust
#[test]
fn test_deleted_rows_excluded_from_subsequent_operations() {
    // ARRANGE: Pipeline with delete followed by aggregate
    let project = Project {
        operations: vec![
            OperationInstance {
                order: 1,
                kind: OperationKind::Delete,
                parameters: json!({"selector": "amount = 0"}),
                ..Default::default()
            },
            OperationInstance {
                order: 2,
                kind: OperationKind::Aggregate,
                parameters: json!({
                    "aggregations": [
                        {"function": "sum", "column": "amount", "alias": "total"}
                    ]
                }),
                ..Default::default()
            },
        ],
        ..Default::default()
    };
    
    let input_df = df! {
        "_row_id" => &["r1", "r2", "r3"],
        "_deleted" => &[false, false, false],
        "amount" => &[0, 100, 200],
    }.unwrap();
    
    // ACT: Execute pipeline
    let result = execute_pipeline(project, input_df).unwrap();
    
    // ASSERT: Aggregate only sums non-deleted rows (100 + 200 = 300)
    assert_eq!(result.column("total").unwrap().sum::<i64>(), Some(300));
    // r1 with amount=0 was deleted and excluded from aggregate
}
```

**Expected result**: Test fails (pipeline doesn't auto-filter deleted rows)

---

**Step 2.2: Implement automatic deleted row filtering**

Location: `crates/core/src/execution/pipeline.rs`

```rust
pub fn execute_pipeline(project: Project, mut working_df: DataFrame) -> Result<DataFrame> {
    let mut df = working_df.lazy();
    
    for op in project.operations.iter() {
        // Execute operation
        df = match op.kind {
            OperationKind::Delete => {
                let params: DeleteOperationParams = serde_json::from_value(op.parameters.clone())?;
                execute_delete(params, df)?
            },
            OperationKind::Update => execute_update(/* ... */),
            // ... other operation types
            OperationKind::Output => {
                // Output doesn't filter here (controlled by include_deleted param)
                execute_output(/* ... */);
                continue;  // Output doesn't modify working_df
            },
        };
        
        // CRITICAL: Filter deleted rows for non-output operations
        if op.kind != OperationKind::Output {
            df = df.filter(col("_deleted").eq(lit(false)));
        }
    }
    
    Ok(df.collect()?)
}
```

**Expected result**: Test passes (green)

---

### Phase 3: Output Operation Extension (TDD)

**Step 3.1: Test output with include_deleted flag**

```rust
#[test]
fn test_output_excludes_deleted_rows_by_default() {
    let df = /* 2 active, 1 deleted row */;
    let params = OutputOperationParams {
        destination: /* ... */,
        include_deleted: false,  // Default
    };
    
    let output_df = execute_output(params, df).unwrap();
    
    // Only 2 active rows should be written
    assert_eq!(output_df.height(), 2);
}

#[test]
fn test_output_includes_deleted_rows_when_requested() {
    let df = /* 2 active, 1 deleted row */;
    let params = OutputOperationParams {
        destination: /* ... */,
        include_deleted: true,
    };
    
    let output_df = execute_output(params, df).unwrap();
    
    // All 3 rows should be written
    assert_eq!(output_df.height(), 3);
}
```

---

**Step 3.2: Implement output filtering logic**

```rust
pub fn execute_output(params: OutputOperationParams, df: LazyFrame) -> Result<()> {
    let output_df = if !params.include_deleted {
        df.filter(col("_deleted").eq(lit(false)))
    } else {
        df
    };
    
    // Write to destination
    write_to_destination(&params.destination, output_df.collect()?)?;
    Ok(())
}
```

---

### Phase 4: Contract Tests (Acceptance Criteria)

**Step 4.1: Write YAML scenario for User Story 1**

Location: `crates/test-resolver/tests/scenarios/delete_selective.yaml`

```yaml
name: "Delete Operation - Selective Row Deletion"
description: "Verify delete operation marks only matching rows as deleted"

dataset:
  orders:
    schema:
      - name: id
        type: int64
      - name: amount
        type: int64
      - name: status
        type: string
    rows:
      - {id: 1, amount: 0, status: "pending"}
      - {id: 2, amount: 100, status: "active"}
      - {id: 3, amount: 200, status: "active"}

project:
  operations:
    - seq: 1
      type: delete
      selector: "amount = 0"

expected_output:
  rows:
    - {id: 2, amount: 100, status: "active", _deleted: false}
    - {id: 3, amount: 200, status: "active", _deleted: false}
  # Row with id=1 should be marked deleted and excluded from output
```

---

**Step 4.2: Run contract test suite**

```bash
cd /workspace
cargo test --package test-resolver --test scenarios
```

**Expected result**: All acceptance scenarios pass

---

## Testing Strategy

### Test Pyramid

```
        ^
       / \
      /   \     Contract Tests (End-to-End Scenarios)
     /     \    - YAML-based acceptance tests
    /_______\   - Verify all user stories from spec.md
   /         \
  /           \  Integration Tests
 /             \ - Multi-operation pipelines
/_____________\ - Deleted row filtering across operations

 _______________
|               | Unit Tests
|               | - Delete logic
|               | - Selector compilation
|               | - Metadata updates
|_______________|
```

### Test Coverage Requirements

**Unit Tests**: 100% coverage for:
- `execute_delete()` function
- Selector compilation edge cases
- Metadata update logic

**Integration Tests**: Cover:
- Delete -> Update pipeline
- Delete -> Aggregate pipeline
- Delete -> Output pipeline
- Multiple delete operations in sequence

**Contract Tests**: All acceptance scenarios from spec:
- User Story 1: Selective deletion
- User Story 2: Delete all (no selector)
- User Story 3: Output visibility control

---

## Running Tests

```bash
# Run all tests
cargo test

# Run unit tests only
cargo test --package dobo-core --lib operations::delete

# Run integration tests
cargo test --package dobo-core --test integration

# Run contract tests
cargo test --package test-resolver

# Run with coverage
cargo tarpaulin --out Html --output-dir coverage
```

---

## Common Pitfalls

### [X] Pitfall 1: Eager DataFrame Collection

```rust
// BAD: Breaks lazy evaluation
let df = df.collect()?;  // Materializes entire DataFrame
let filtered = df.lazy().filter(/* ... */);
```

```rust
// GOOD: Maintain lazy evaluation
let df = df.filter(/* ... */);  // Stays lazy
// Only collect when absolutely necessary (e.g., final output)
```

---

### [X] Pitfall 2: Forgetting to Update _modified_at

```rust
// BAD: Only updates _deleted
df.with_column(
    when(selector_expr).then(lit(true)).otherwise(col("_deleted")).alias("_deleted")
);
```

```rust
// GOOD: Updates both _deleted and _modified_at
df.with_column(
    when(selector_expr.clone()).then(lit(true)).otherwise(col("_deleted")).alias("_deleted")
)
.with_column(
    when(selector_expr).then(lit(current_time)).otherwise(col("_modified_at")).alias("_modified_at")
);
```

---

### [X] Pitfall 3: Not Filtering Deleted Rows in Pipeline

```rust
// BAD: Deleted rows leak into next operation
for op in operations {
    df = execute_operation(op, df)?;
    // Missing filter step!
}
```

```rust
// GOOD: Auto-filter after each non-output operation
for op in operations {
    df = execute_operation(op, df)?;
    if op.kind != OperationKind::Output {
        df = df.filter(col("_deleted").eq(lit(false)));
    }
}
```

---

## Debugging Tips

**Enable Polars query plan visualization**:
```rust
println!("{}", df.describe_optimized_plan()?);
```

**Check DataFrame state at each step**:
```rust
let df = df.with_column(/* delete logic */);
eprintln!("After delete: {:?}", df.clone().collect()?);
```

**Validate selector compilation**:
```rust
let expr = compile_selector("amount = 0")?;
eprintln!("Compiled expression: {:?}", expr);
```

---

## Performance Considerations

**Lazy Evaluation**:
- Polars optimizes entire query plan before execution
- Multiple `.with_column()` calls are fused into single pass
- Avoid `.collect()` until final result needed

**Memory Efficiency**:
- Soft delete adds 2 columns (`_deleted` + `_modified_at`): ~9 bytes/row overhead
- No row removal means no memory reallocation
- Filtering deleted rows is zero-copy (just metadata update)

**Benchmarks** (target performance):
- 10k rows: <10ms for delete operation
- 100k rows: <50ms
- 1M rows: <500ms

---

## Next Steps

After implementing delete operation:

1. **Run full test suite**: Ensure 100% pass rate
2. **Update documentation**: Add delete examples to main docs
3. **Integration testing**: Test with real-world pipelines
4. **Performance profiling**: Benchmark against targets above

---

## References

- **Research**: `./research.md` - Technical decisions and patterns
- **Data Model**: `./data-model.md` - Entity schemas and relationships
- **Contracts**: `./contracts/delete-operation-schema.md` - YAML schema specification
- **Feature Spec**: `./spec.md` - User stories and acceptance criteria
- **Polars Docs**: https://docs.pola.rs/ - DataFrame API reference
