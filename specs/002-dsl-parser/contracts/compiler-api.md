# Compiler API Contract

**Module**: `dobo_core::dsl::compiler`  
**Version**: 0.1.0  
**Date**: 2026-02-22

## Overview

The Compiler API transforms validated ExprAST trees into Polars `Expr` objects that can be attached to LazyFrame operations. It handles all DSL function mappings, operator conversions, and type-safe Polars expression construction.

---

## Public API

### `compile_expression`

**Signature**:
```rust
pub fn compile_expression(
    ast: &ExprAST,
    context: &CompilationContext,
) -> Result<CompiledExpression, CompilationError>
```

**Description**:
Compiles a validated ExprAST into a Polars Expr with full context awareness.

**Parameters**:
- `ast: &ExprAST` - The expression AST (must be validated: column resolution + type checking already done)
- `context: &CompilationContext` - Compilation context (schema, selectors, TODAY timestamp, aggregate flag)

**Returns**:
- `Ok(CompiledExpression)` - Successfully compiled expression with Polars Expr
- `Err(CompilationError)` - Compilation failure

**Errors**:
- `CompilationError::UnsupportedFunction` - Function not yet implemented (e.g., window functions deferred to later)
- `CompilationError::PolarsCompatibility` - Type mismatch between DSL and Polars representation

**Examples**:
```rust
// Simple arithmetic
let ast = parse_expression("orders.amount * 1.1")?;
let context = CompilationContext::new(schema, HashMap::new(), NaiveDate::default(), false);
let compiled = compile_expression(&ast, &context)?;
// compiled.expr is: col("orders.amount").mul(lit(1.1))

// Conditional with function
let ast = parse_expression("IF(orders.status = \"active\", 1, 0)")?;
let compiled = compile_expression(&ast, &context)?;
// compiled.expr is: when(col("orders.status").eq(lit("active"))).then(lit(1)).otherwise(lit(0))
```

**Preconditions**:
- AST must be validated (column resolution + type checking passed)
- CompilationContext must have valid schema
- For aggregate functions, context.allow_aggregates must be true (validated earlier)

**Postconditions**:
- Returned CompiledExpression contains a valid Polars Expr
- Expr can be attached to a LazyFrame without panic
- Return type matches inferred type from validation

**Performance**:
- O(n) in AST node count
- Target: <1ms for typical expressions (<100 nodes)
- Zero allocations for literals and column references (Polars handles Arc internally)

---

### `compile_with_interpolation`

**Signature**:
```rust
pub fn compile_with_interpolation(
    source: &str,
    context: &CompilationContext,
) -> Result<CompiledExpression, CompilationError>
```

**Description**:
End-to-end compilation: selector interpolation → parse → validate → compile.

**Parameters**:
- `source: &str` - Original expression string (may contain {{SELECTOR}} references)
- `context: &CompilationContext` - Full compilation context

**Returns**:
- `Ok(CompiledExpression)` - Successfully compiled expression
- `Err(CompilationError)` - Any phase failure (parse, validate, compile)

**Errors**:
- All ParseError variants (wrapped in CompilationError::ParseFailure)
- All ValidationError variants (wrapped in CompilationError::ValidationFailure)
- All CompilationError variants

**Examples**:
```rust
let mut selectors = HashMap::new();
selectors.insert("EMEA_ONLY".to_string(), "region = \"EMEA\"".to_string());

let context = CompilationContext::new(schema, selectors, NaiveDate::default(), false);
let compiled = compile_with_interpolation("{{EMEA_ONLY}} AND amount > 1000", &context)?;
// First expands to: "region = \"EMEA\" AND amount > 1000"
// Then compiles to: col("region").eq(lit("EMEA")).and(col("amount").gt(lit(1000)))
```

**Use Case**:
- Primary entry point for expression compilation from raw strings
- Handles full pipeline in one call

---

## Data Types

### `CompilationContext`

See `/workspace/specs/002-dsl-parser/data-model.md` for full definition.

**Summary**:
```rust
pub struct CompilationContext {
    pub schema: DatasetSchema,
    pub join_aliases: Vec<String>,
    pub selectors: HashMap<String, String>,
    pub today: chrono::NaiveDate,
    pub allow_aggregates: bool,
}
```

**Construction**:
```rust
impl CompilationContext {
    pub fn new(
        schema: DatasetSchema,
        selectors: HashMap<String, String>,
        today: chrono::NaiveDate,
        allow_aggregates: bool,
    ) -> Self;
    
    pub fn with_join_aliases(self, aliases: Vec<String>) -> Self;
}
```

---

### `CompiledExpression`

See `/workspace/specs/002-dsl-parser/data-model.md` for full definition.

**Summary**:
```rust
pub struct CompiledExpression {
    pub source: String,
    pub expr: polars::lazy::dsl::Expr,
    pub return_type: ExprType,
}
```

**Methods**:
```rust
impl CompiledExpression {
    /// Extract the Polars Expr (consumes self)
    pub fn into_expr(self) -> polars::lazy::dsl::Expr;
    
    /// Borrow the Polars Expr
    pub fn as_expr(&self) -> &polars::lazy::dsl::Expr;
    
    /// Get the inferred return type
    pub fn return_type(&self) -> ExprType;
}
```

---

### `CompilationError`

**Definition**:
```rust
#[derive(Debug, thiserror::Error)]
pub enum CompilationError {
    #[error("Parse failure: {0}")]
    ParseFailure(#[from] ParseError),
    
    #[error("Validation failure: {0}")]
    ValidationFailure(#[from] ValidationError),
    
    #[error("Unsupported function: {function}")]
    UnsupportedFunction { function: String },
    
    #[error("Polars compatibility issue: {message}")]
    PolarsCompatibility { message: String },
}
```

---

## DSL → Polars Function Mapping

### Arithmetic Functions

| DSL Function | Polars Expr |
|--------------|-------------|
| `ABS(x)` | `col(x).abs()` |
| `ROUND(x, n)` | `col(x).round(n)` |
| `FLOOR(x)` | `col(x).floor()` |
| `CEIL(x)` | `col(x).ceil()` |
| `MOD(x, y)` | `col(x) % lit(y)` |
| `MIN(a, b)` | `when(col(a).lt(col(b))).then(col(a)).otherwise(col(b))` |
| `MAX(a, b)` | `when(col(a).gt(col(b))).then(col(a)).otherwise(col(b))` |

### String Functions

| DSL Function | Polars Expr |
|--------------|-------------|
| `CONCAT(a, b, ...)` | `concat_str([col(a), col(b), ...], "")` |
| `UPPER(s)` | `col(s).str().to_uppercase()` |
| `LOWER(s)` | `col(s).str().to_lowercase()` |
| `TRIM(s)` | `col(s).str().strip_chars(None)` |
| `LEFT(s, n)` | `col(s).str().slice(0, Some(n))` |
| `RIGHT(s, n)` | `col(s).str().slice(-n, None)` |
| `LEN(s)` | `col(s).str().len_chars()` |
| `CONTAINS(s, substr)` | `col(s).str().contains(lit(substr), false)` |
| `REPLACE(s, old, new)` | `col(s).str().replace_all(lit(old), lit(new), false)` |

### Conditional Functions

| DSL Function | Polars Expr |
|--------------|-------------|
| `IF(cond, then, else)` | `when(cond).then(then).otherwise(else)` |
| `ISNULL(x)` | `col(x).is_null()` |
| `COALESCE(a, b, ...)` | `col(a).fill_null(col(b)).fill_null(...).fill_null(col(z))` |

### Date/Time Functions

| DSL Function | Polars Expr |
|--------------|-------------|
| `DATE(iso_str)` | `lit(NaiveDate::parse_from_str(...))` |
| `TODAY()` | `lit(context.today)` |
| `YEAR(date)` | `col(date).dt().year()` |
| `MONTH(date)` | `col(date).dt().month()` |
| `DAY(date)` | `col(date).dt().day()` |
| `DATEDIFF(end, start)` | `(col(end) - col(start)).dt().total_days()` |
| `DATEADD(date, n)` | `col(date) + lit(Duration::days(n))` |

### Aggregate Functions

| DSL Function | Polars Expr |
|--------------|-------------|
| `SUM(x)` | `col(x).sum()` |
| `COUNT(x)` | `col(x).count()` |
| `COUNT_ALL()` | `count()` |
| `AVG(x)` | `col(x).mean()` |
| `MIN_AGG(x)` | `col(x).min()` |
| `MAX_AGG(x)` | `col(x).max()` |

**Note**: Aggregate functions require `context.allow_aggregates = true` (validated in type checker).

### Operators

| DSL Operator | Polars Expr |
|--------------|-------------|
| `a + b` | `col(a).add(col(b))` or `col(a) + col(b)` |
| `a - b` | `col(a).sub(col(b))` or `col(a) - col(b)` |
| `a * b` | `col(a).mul(col(b))` or `col(a) * col(b)` |
| `a / b` | `col(a).div(col(b))` or `col(a) / col(b)` |
| `a = b` | `col(a).eq(col(b))` |
| `a <> b` | `col(a).neq(col(b))` |
| `a < b` | `col(a).lt(col(b))` |
| `a <= b` | `col(a).lt_eq(col(b))` |
| `a > b` | `col(a).gt(col(b))` |
| `a >= b` | `col(a).gt_eq(col(b))` |
| `a AND b` | `col(a).and(col(b))` |
| `a OR b` | `col(a).or(col(b))` |
| `NOT a` | `col(a).not()` |

---

## Testing Contract

### Unit Tests Required

**Function Mappings** (one test per function):
- Each DSL function compiles to correct Polars Expr
- Verify Expr structure (inspect Debug representation)
- Validate with dummy LazyFrame (attach Expr without panic)

**Operator Mappings**:
- Each binary operator
- Each unary operator
- Operator chaining (precedence preserved)

**Edge Cases**:
- Nested function calls
- Complex conditionals (nested IF)
- String concatenation with many arguments
- NULL literal handling

### Integration Tests Required

**Sample Expressions** (from feature spec):
```rust
// Each sample expression from spec must compile successfully
"transactions.amount_local * fx.rate"
"IF(accounts.type = \"revenue\", transactions.amount_local * -1, transactions.amount_local)"
"SUM(transactions.amount_local)"
"CONCAT(accounts.code, \" - \", accounts.name)"
```

**Contract Tests** (validate Polars compatibility):
```rust
// Create dummy LazyFrame, attach compiled Expr, verify no panic
let df = df! {
    "orders.amount" => [100, 200, 300],
}.unwrap().lazy();

let ast = parse_expression("orders.amount * 1.1")?;
let context = CompilationContext::new(schema, HashMap::new(), NaiveDate::default(), false);
let compiled = compile_expression(&ast, &context)?;

let result = df.select([compiled.into_expr()]);
assert!(result.collect().is_ok());
```

---

## Versioning & Compatibility

**Stability**: Unstable (pre-1.0)

**Polars Version Compatibility**:
- Tested against Polars 0.46
- Minor version changes (0.46 → 0.47) may require updates
- Major version changes (0.x → 1.x) will require significant work

**Breaking Changes**:
- New DSL functions added
- Function mapping changes
- CompilationContext structure changes

**Non-Breaking Changes**:
- Performance improvements
- Error message improvements
- Additional validation

---

## References

- Feature spec: `/workspace/docs/specs/S01-dsl-parser/prompt.md`
- Data model: `/workspace/specs/002-dsl-parser/data-model.md`
- Polars Expr API: https://docs.rs/polars/latest/polars/lazy/dsl/struct.Expr.html
- Polars lazy functions: https://docs.rs/polars/latest/polars/lazy/dsl/functions/
