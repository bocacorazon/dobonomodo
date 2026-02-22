# Quickstart: DSL Parser & Expression Compiler

**Feature**: 002-dsl-parser  
**Version**: 0.1.0  
**Date**: 2026-02-22

## Overview

This quickstart guide shows how to use the DobONoMoDo DSL parser and expression compiler to transform expression strings into Polars `Expr` objects.

---

## Installation

The DSL module is part of the `dobo-core` crate in the DobONoMoDo workspace.

**Add to your Cargo.toml**:
```toml
[dependencies]
dobo-core = { path = "../crates/core" }
polars = { version = "0.46", default-features = false, features = ["lazy"] }
chrono = "0.4"
```

---

## Basic Usage

### 1. Parse an Expression

```rust
use dobo_core::dsl::parser::parse_expression;

fn main() -> Result<(), Box<dyn std::error::Error>> {
    // Parse a simple arithmetic expression
    let ast = parse_expression("orders.amount * 1.1")?;
    println!("Parsed AST: {:#?}", ast);
    
    Ok(())
}
```

**Output**:
```
Parsed AST: BinaryOp {
    op: Multiply,
    left: ColumnRef { table: "orders", column: "amount" },
    right: Literal(Number(1.1)),
}
```

---

### 2. Validate an Expression

```rust
use dobo_core::dsl::{parser::parse_expression, validation::validate_expression};
use dobo_core::model::{DatasetSchema, ColumnDef, ColumnType};
use std::collections::HashMap;
use chrono::NaiveDate;

fn main() -> Result<(), Box<dyn std::error::Error>> {
    // Build a simple schema
    let schema = DatasetSchema {
        tables: vec![
            TableRef {
                logical_name: "orders".into(),
                columns: vec![
                    ColumnDef { name: "amount".into(), column_type: ColumnType::Number },
                    ColumnDef { name: "status".into(), column_type: ColumnType::String },
                ],
            },
        ],
    };
    
    // Parse expression
    let ast = parse_expression("orders.amount > 100")?;
    
    // Validate (column resolution + type checking)
    let context = CompilationContext::new(schema, HashMap::new(), NaiveDate::default(), false);
    let typed_ast = validate_expression(&ast, &context)?;
    
    println!("Return type: {:?}", typed_ast.return_type());
    // Output: Return type: Boolean
    
    Ok(())
}
```

---

### 3. Compile to Polars Expr

```rust
use dobo_core::dsl::compiler::compile_expression;
use polars::prelude::*;

fn main() -> Result<(), Box<dyn std::error::Error>> {
    // Parse and validate (as above)
    let ast = parse_expression("orders.amount > 100")?;
    let context = CompilationContext::new(schema, HashMap::new(), NaiveDate::default(), false);
    
    // Compile to Polars Expr
    let compiled = compile_expression(&ast, &context)?;
    
    // Use in a LazyFrame
    let df = df! {
        "orders.amount" => [50, 150, 200],
    }?.lazy();
    
    let filtered = df.filter(compiled.into_expr());
    let result = filtered.collect()?;
    
    println!("{}", result);
    // Output: DataFrame with rows where amount > 100
    
    Ok(())
}
```

---

### 4. End-to-End: String to Polars Expr

```rust
use dobo_core::dsl::compiler::compile_with_interpolation;

fn main() -> Result<(), Box<dyn std::error::Error>> {
    // Build context with schema and selectors
    let mut selectors = HashMap::new();
    selectors.insert("ACTIVE_ONLY".into(), "orders.status = \"active\"".into());
    
    let context = CompilationContext::new(schema, selectors, NaiveDate::default(), false);
    
    // Compile directly from string (with selector interpolation)
    let compiled = compile_with_interpolation("{{ACTIVE_ONLY}} AND orders.amount > 100", &context)?;
    
    // Use the compiled expression
    let df = df! {
        "orders.status" => ["active", "closed", "active"],
        "orders.amount" => [50, 150, 200],
    }?.lazy();
    
    let filtered = df.filter(compiled.into_expr());
    let result = filtered.collect()?;
    
    println!("{}", result);
    // Output: DataFrame with active orders where amount > 100
    
    Ok(())
}
```

---

## Expression Syntax

### Literals

```rust
// Number
parse_expression("42")?;
parse_expression("3.14")?;
parse_expression("-7")?;

// String
parse_expression("\"active\"")?;
parse_expression("\"USD\"")?;

// Boolean
parse_expression("TRUE")?;
parse_expression("FALSE")?;

// Date
parse_expression("DATE(\"2026-01-01\")")?;

// NULL
parse_expression("NULL")?;
```

---

### Column References

```rust
// Table.column syntax
parse_expression("orders.amount")?;
parse_expression("customers.country_code")?;
parse_expression("products.unit_price")?;
```

**Requirements**:
- Table name must match a logical table in the Dataset
- Column name must exist in that table's schema
- Case-sensitive (both table and column names)

---

### Operators

#### Arithmetic
```rust
parse_expression("orders.amount + 100")?;
parse_expression("orders.amount - discount")?;
parse_expression("orders.quantity * products.unit_price")?;
parse_expression("orders.total / orders.quantity")?;
```

#### Comparison
```rust
parse_expression("orders.amount = 100")?;
parse_expression("orders.status <> \"cancelled\"")?;
parse_expression("orders.amount > 1000")?;
parse_expression("orders.amount >= 1000")?;
parse_expression("orders.amount < 100")?;
parse_expression("orders.amount <= 100")?;
```

#### Logical
```rust
parse_expression("orders.amount > 100 AND orders.status = \"active\"")?;
parse_expression("orders.status = \"pending\" OR orders.status = \"processing\"")?;
parse_expression("NOT orders.cancelled")?;
```

**Precedence** (highest to lowest):
1. Multiplication, Division
2. Addition, Subtraction
3. Comparison (=, <>, <, <=, >, >=)
4. Logical AND
5. Logical OR

Use parentheses to override: `(orders.amount + 100) * 1.1`

---

### Functions

#### Conditional
```rust
// IF(condition, then, else)
parse_expression("IF(orders.amount > 1000, \"large\", \"small\")")?;

// ISNULL(expr)
parse_expression("ISNULL(orders.discount)")?;

// COALESCE(expr1, expr2, ...)
parse_expression("COALESCE(orders.discount, 0)")?;
```

#### Arithmetic
```rust
parse_expression("ABS(orders.amount)")?;
parse_expression("ROUND(orders.amount, 2)")?;
parse_expression("FLOOR(orders.amount)")?;
parse_expression("CEIL(orders.amount)")?;
parse_expression("MOD(orders.amount, 10)")?;
parse_expression("MIN(orders.amount, orders.limit)")?;
parse_expression("MAX(orders.amount, 100)")?;
```

#### String
```rust
parse_expression("CONCAT(accounts.code, \" - \", accounts.name)")?;
parse_expression("UPPER(customers.country_code)")?;
parse_expression("LOWER(products.name)")?;
parse_expression("TRIM(customers.email)")?;
parse_expression("LEFT(products.sku, 3)")?;
parse_expression("RIGHT(products.sku, 4)")?;
parse_expression("LEN(customers.name)")?;
parse_expression("CONTAINS(products.description, \"organic\")")?;
parse_expression("REPLACE(products.name, \"old\", \"new\")")?;
```

#### Date/Time
```rust
parse_expression("DATE(\"2026-01-01\")")?;
parse_expression("TODAY()")?;  // Resolves to Run.started_at
parse_expression("YEAR(orders.posting_date)")?;
parse_expression("MONTH(orders.posting_date)")?;
parse_expression("DAY(orders.posting_date)")?;
parse_expression("DATEDIFF(orders.end_date, orders.start_date)")?;
parse_expression("DATEADD(orders.posting_date, 30)")?;
parse_expression("orders.posting_date >= TODAY() - 30")?;
```

#### Aggregate (requires `allow_aggregates=true`)
```rust
// Only valid in aggregate/rollup operations
let context_with_agg = CompilationContext::new(schema, HashMap::new(), NaiveDate::default(), true);

parse_expression("SUM(orders.amount)")?;
parse_expression("COUNT(orders.order_id)")?;
parse_expression("COUNT_ALL()")?;
parse_expression("AVG(orders.amount)")?;
parse_expression("MIN_AGG(orders.amount)")?;
parse_expression("MAX_AGG(orders.amount)")?;
```

**Note**: Attempting to use aggregate functions with `allow_aggregates=false` will fail validation.

---

### Selector Interpolation

**Define selectors** in Project:
```rust
let mut selectors = HashMap::new();
selectors.insert("EMEA_ONLY".into(), "customers.region = \"EMEA\"".into());
selectors.insert("LARGE_ORDERS".into(), "orders.amount > 1000".into());
```

**Use in expressions**:
```rust
// Simple selector
compile_with_interpolation("{{EMEA_ONLY}}", &context)?;
// Expands to: customers.region = "EMEA"

// Combine selectors
compile_with_interpolation("{{EMEA_ONLY}} AND {{LARGE_ORDERS}}", &context)?;
// Expands to: customers.region = "EMEA" AND orders.amount > 1000

// Nested selectors
selectors.insert("COMPLEX".into(), "{{EMEA_ONLY}} OR status = \"premium\"".into());
compile_with_interpolation("{{COMPLEX}}", &context)?;
// Expands to: customers.region = "EMEA" OR status = "premium"
```

---

## Error Handling

### Parse Errors

```rust
let result = parse_expression("orders.amount +");
match result {
    Err(ParseError::UnexpectedToken { expected, found, line, col }) => {
        println!("Parse error at line {}, column {}: expected {}, found {}", line, col, expected, found);
    }
    _ => {}
}
```

### Validation Errors

```rust
let ast = parse_expression("nonexistent.column")?;
let result = validate_expression(&ast, &context);

match result {
    Err(ValidationError::UnresolvedColumnRef { table, column }) => {
        println!("Column {}.{} not found in schema", table, column);
    }
    Err(ValidationError::TypeMismatch { expected, found }) => {
        println!("Type mismatch: expected {:?}, found {:?}", expected, found);
    }
    Err(ValidationError::InvalidAggregateContext { function }) => {
        println!("Aggregate function {} used outside aggregate context", function);
    }
    _ => {}
}
```

---

## Testing Your Expressions

### Unit Test Template

```rust
#[cfg(test)]
mod tests {
    use super::*;
    
    #[test]
    fn test_simple_arithmetic() {
        let ast = parse_expression("orders.amount * 1.1").unwrap();
        
        let schema = build_test_schema();
        let context = CompilationContext::new(schema, HashMap::new(), NaiveDate::default(), false);
        
        let typed_ast = validate_expression(&ast, &context).unwrap();
        assert_eq!(typed_ast.return_type(), ExprType::Number);
        
        let compiled = compile_expression(&ast, &context).unwrap();
        // Validate with dummy LazyFrame
        let df = df! { "orders.amount" => [100, 200, 300] }.unwrap().lazy();
        let result = df.select([compiled.into_expr()]).collect();
        assert!(result.is_ok());
    }
}
```

---

## Common Patterns

### Building a CompilationContext

```rust
fn build_context(
    schema: DatasetSchema,
    project: &Project,
    run: &Run,
    allow_aggregates: bool,
) -> CompilationContext {
    let selectors = project.selectors.clone();
    let today = run.started_at.date();
    
    CompilationContext::new(schema, selectors, today, allow_aggregates)
}
```

### Batch Compilation

```rust
fn compile_all_expressions(
    expressions: &[String],
    context: &CompilationContext,
) -> Result<Vec<CompiledExpression>, CompilationError> {
    expressions.iter()
        .map(|expr| compile_with_interpolation(expr, context))
        .collect()
}
```

### Type-Safe Expression Builder

```rust
fn build_selector_expression(conditions: &[String]) -> String {
    if conditions.is_empty() {
        "TRUE".to_string()
    } else {
        conditions.join(" AND ")
    }
}

// Usage
let conditions = vec![
    "orders.status = \"active\"".to_string(),
    "orders.amount > 100".to_string(),
    "customers.region = \"EMEA\"".to_string(),
];
let expr_string = build_selector_expression(&conditions);
let compiled = compile_with_interpolation(&expr_string, &context)?;
```

---

## Next Steps

- **Read the API contracts**: See `/workspace/specs/002-dsl-parser/contracts/` for detailed API documentation
- **Explore the data model**: See `/workspace/specs/002-dsl-parser/data-model.md` for AST structure and validation rules
- **Check the feature spec**: See `/workspace/docs/specs/S01-dsl-parser/prompt.md` for complete feature requirements
- **Implement operations**: See S03+ specs for using compiled expressions in execution pipeline

---

## Troubleshooting

### "Unresolved column reference" error

**Problem**: Column not found in schema.

**Solution**:
1. Verify table name matches logical table in Dataset
2. Verify column name exists in that table's ColumnDef list
3. Check for case sensitivity (both table and column names are case-sensitive)

### "Type mismatch" error

**Problem**: Incompatible types in operation.

**Solution**:
1. Check operator requirements (arithmetic requires Number, logical requires Boolean)
2. Verify function argument types match signature
3. Use IF() or COALESCE() to handle type conversions

### "Aggregate function used outside aggregate context" error

**Problem**: SUM/COUNT/AVG used with `allow_aggregates=false`.

**Solution**:
1. Set `allow_aggregates=true` in CompilationContext
2. Ensure expression is used in aggregate/rollup operation (not filter/assignment)

### "Circular selector reference" error

**Problem**: Selector references itself directly or indirectly.

**Solution**:
1. Review selector definitions in Project
2. Break circular chain by redefining selectors
3. Use base expressions instead of {{}} references in problematic selectors

---

## Performance Tips

1. **Reuse CompilationContext**: Build once, use for all expressions in a Project/Run
2. **Batch compile**: Use `compile_all_expressions()` pattern for multiple expressions
3. **Validate early**: Run validation during Project activation, not during Run execution
4. **Cache compiled expressions**: Store CompiledExpression results, avoid re-compilation

---

## References

- Parser API: `/workspace/specs/002-dsl-parser/contracts/parser-api.md`
- Compiler API: `/workspace/specs/002-dsl-parser/contracts/compiler-api.md`
- Validation API: `/workspace/specs/002-dsl-parser/contracts/validation-api.md`
- Data Model: `/workspace/specs/002-dsl-parser/data-model.md`
- Feature Spec: `/workspace/docs/specs/S01-dsl-parser/prompt.md`
