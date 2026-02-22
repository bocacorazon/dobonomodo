# Validation API Contract

**Module**: `dobo_core::dsl::validation`  
**Version**: 0.1.0  
**Date**: 2026-02-22

## Overview

The Validation API performs semantic validation on parsed ExprAST trees: column resolution, type checking, aggregate context validation, and selector reference validation.

---

## Public API

### `validate_expression`

**Signature**:
```rust
pub fn validate_expression(
    ast: &ExprAST,
    context: &CompilationContext,
) -> Result<TypedExprAST, ValidationError>
```

**Description**:
Validates an expression AST against the compilation context, performing column resolution and type checking.

**Parameters**:
- `ast: &ExprAST` - The expression AST to validate
- `context: &CompilationContext` - Validation context (schema, aggregate flag)

**Returns**:
- `Ok(TypedExprAST)` - Validated AST with type annotations
- `Err(ValidationError)` - Validation failure

**Errors**:
- `ValidationError::UnresolvedColumnRef` - Column not found in schema
- `ValidationError::TypeMismatch` - Incompatible types
- `ValidationError::InvalidAggregateContext` - Aggregate function outside aggregate operation

**Examples**:
```rust
// Success case
let ast = parse_expression("orders.amount + 100")?;
let context = CompilationContext::new(schema, HashMap::new(), NaiveDate::default(), false);
let typed_ast = validate_expression(&ast, &context)?;
assert_eq!(typed_ast.return_type(), ExprType::Number);

// Error case: unresolved column
let ast = parse_expression("nonexistent.column")?;
let result = validate_expression(&ast, &context);
assert!(matches!(result, Err(ValidationError::UnresolvedColumnRef { .. })));

// Error case: type mismatch
let ast = parse_expression("orders.amount + \"text\"")?;
let result = validate_expression(&ast, &context);
assert!(matches!(result, Err(ValidationError::TypeMismatch { .. })));

// Error case: aggregate outside context
let ast = parse_expression("SUM(orders.amount)")?;
let context_no_agg = CompilationContext::new(schema, HashMap::new(), NaiveDate::default(), false);
let result = validate_expression(&ast, &context_no_agg);
assert!(matches!(result, Err(ValidationError::InvalidAggregateContext { .. })));
```

**Preconditions**:
- AST must be syntactically valid (from parser)
- CompilationContext must have valid schema

**Postconditions**:
- If validation succeeds, all column references are resolvable
- All type constraints are satisfied
- Aggregate functions only present if allow_aggregates=true

**Performance**:
- O(n) in AST node count
- Target: <1ms for typical expressions

---

### `resolve_column`

**Signature**:
```rust
pub fn resolve_column(
    table: &str,
    column: &str,
    schema: &DatasetSchema,
) -> Result<ColumnDef, ValidationError>
```

**Description**:
Resolves a column reference to its ColumnDef in the schema.

**Parameters**:
- `table: &str` - Logical table name
- `column: &str` - Column name
- `schema: &DatasetSchema` - Dataset schema

**Returns**:
- `Ok(ColumnDef)` - Column definition if found
- `Err(ValidationError::UnresolvedColumnRef)` - Column not found

**Examples**:
```rust
let column_def = resolve_column("orders", "amount", &schema)?;
assert_eq!(column_def.column_type, ColumnType::Number);
```

**Use Case**:
- Called internally during validation
- Can be used standalone for schema introspection

---

### `infer_type`

**Signature**:
```rust
pub fn infer_type(
    ast: &ExprAST,
    schema: &DatasetSchema,
) -> Result<ExprType, ValidationError>
```

**Description**:
Infers the return type of an expression AST.

**Parameters**:
- `ast: &ExprAST` - Expression AST
- `schema: &DatasetSchema` - Schema for column type lookup

**Returns**:
- `Ok(ExprType)` - Inferred type
- `Err(ValidationError)` - Type inference failure (unresolved column or type mismatch)

**Type Inference Rules**:
- **Literals**: Type from LiteralValue variant
- **Column references**: Type from ColumnDef.column_type
- **Arithmetic operators**: Number × Number → Number
- **Comparison operators**: T × T → Boolean
- **Logical operators**: Boolean × Boolean → Boolean
- **Functions**: Return type from function signature

**Examples**:
```rust
// Literal
let ast = ExprAST::Literal(LiteralValue::Number(42.0));
assert_eq!(infer_type(&ast, &schema)?, ExprType::Number);

// Column reference
let ast = ExprAST::ColumnRef { table: "orders".into(), column: "amount".into() };
assert_eq!(infer_type(&ast, &schema)?, ExprType::Number);

// Arithmetic
let ast = parse_expression("orders.amount + 100")?;
assert_eq!(infer_type(&ast, &schema)?, ExprType::Number);

// Comparison
let ast = parse_expression("orders.amount > 100")?;
assert_eq!(infer_type(&ast, &schema)?, ExprType::Boolean);
```

---

### `interpolate_selectors`

**Signature**:
```rust
pub fn interpolate_selectors(
    source: &str,
    selectors: &HashMap<String, String>,
) -> Result<String, ValidationError>
```

**Description**:
Expands {{SELECTOR}} references in an expression string.

**Parameters**:
- `source: &str` - Expression string with potential {{NAME}} tokens
- `selectors: &HashMap<String, String>` - Named selectors from Project

**Returns**:
- `Ok(String)` - Expanded expression string
- `Err(ValidationError)` - Unresolved or circular selector reference

**Errors**:
- `ValidationError::UnresolvedSelectorRef` - {{NAME}} not in selectors map
- `ValidationError::CircularSelectorRef` - Circular reference detected

**Examples**:
```rust
let mut selectors = HashMap::new();
selectors.insert("EMEA_ONLY".into(), "region = \"EMEA\"".into());
selectors.insert("LARGE_AMOUNTS".into(), "amount > 1000".into());

// Simple expansion
let expanded = interpolate_selectors("{{EMEA_ONLY}}", &selectors)?;
assert_eq!(expanded, "region = \"EMEA\"");

// Multiple selectors
let expanded = interpolate_selectors("{{EMEA_ONLY}} AND {{LARGE_AMOUNTS}}", &selectors)?;
assert_eq!(expanded, "region = \"EMEA\" AND amount > 1000");

// Nested selectors
selectors.insert("COMPLEX".into(), "{{EMEA_ONLY}} OR status = \"active\"".into());
let expanded = interpolate_selectors("{{COMPLEX}}", &selectors)?;
assert_eq!(expanded, "region = \"EMEA\" OR status = \"active\"");

// Unresolved selector
let result = interpolate_selectors("{{NONEXISTENT}}", &selectors);
assert!(matches!(result, Err(ValidationError::UnresolvedSelectorRef { .. })));

// Circular reference
selectors.insert("A".into(), "{{B}}".into());
selectors.insert("B".into(), "{{A}}".into());
let result = interpolate_selectors("{{A}}", &selectors);
assert!(matches!(result, Err(ValidationError::CircularSelectorRef { .. })));
```

**Preconditions**:
- Selectors map contains valid expression strings (no syntax errors)

**Postconditions**:
- Returned string contains no {{}} tokens
- Expansion is deterministic and complete

**Performance**:
- O(n × m) where n is source length, m is selector expansion depth
- Target: <1ms for typical cases (1-3 levels of nesting)
- Maximum depth: 10 levels (prevents infinite loops from complex circular refs)

---

## Data Types

### `TypedExprAST`

**Definition**:
```rust
pub struct TypedExprAST {
    ast: ExprAST,
    return_type: ExprType,
}

impl TypedExprAST {
    pub fn ast(&self) -> &ExprAST;
    pub fn return_type(&self) -> ExprType;
    pub fn into_ast(self) -> ExprAST;
}
```

**Purpose**:
- Carries both the AST and its inferred type
- Guarantees type validity (cannot construct with mismatched type)
- Used as input to compiler

---

### `ValidationError`

See `/workspace/specs/002-dsl-parser/data-model.md` for full definition.

**Summary**:
```rust
pub enum ValidationError {
    UnresolvedColumnRef { table: String, column: String },
    TypeMismatch { expected: ExprType, found: ExprType },
    UnresolvedSelectorRef { selector: String },
    InvalidAggregateContext { function: String },
    CircularSelectorRef { chain: String },
}
```

---

## Validation Rules

### Column Resolution (BR-001)

**Rule**: All ColumnRef nodes must match a table.column in DatasetSchema.

**Implementation**:
```rust
fn validate_column_ref(table: &str, column: &str, schema: &DatasetSchema) -> Result<(), ValidationError> {
    // 1. Find table in schema.tables
    let table_ref = schema.tables.iter()
        .find(|t| t.logical_name == table)
        .ok_or_else(|| ValidationError::UnresolvedColumnRef {
            table: table.to_string(),
            column: column.to_string(),
        })?;
    
    // 2. Find column in table.columns
    table_ref.columns.iter()
        .find(|c| c.name == column)
        .ok_or_else(|| ValidationError::UnresolvedColumnRef {
            table: table.to_string(),
            column: column.to_string(),
        })?;
    
    Ok(())
}
```

---

### Type Compatibility (BR-002)

**Rule**: Operators and functions must have compatible operand/argument types.

**Binary Operator Type Rules**:
```rust
fn validate_binary_op(op: BinaryOperator, left_type: ExprType, right_type: ExprType) -> Result<ExprType, ValidationError> {
    match op {
        BinaryOperator::Add | BinaryOperator::Subtract | BinaryOperator::Multiply | BinaryOperator::Divide => {
            if matches!(left_type, ExprType::Number) && matches!(right_type, ExprType::Number) {
                Ok(ExprType::Number)
            } else {
                Err(ValidationError::TypeMismatch {
                    expected: ExprType::Number,
                    found: if !matches!(left_type, ExprType::Number) { left_type } else { right_type },
                })
            }
        }
        BinaryOperator::Equal | BinaryOperator::NotEqual | BinaryOperator::LessThan | /* ... */ => {
            if left_type == right_type {
                Ok(ExprType::Boolean)
            } else {
                Err(ValidationError::TypeMismatch { expected: left_type, found: right_type })
            }
        }
        BinaryOperator::And | BinaryOperator::Or => {
            if matches!(left_type, ExprType::Boolean) && matches!(right_type, ExprType::Boolean) {
                Ok(ExprType::Boolean)
            } else {
                Err(ValidationError::TypeMismatch { expected: ExprType::Boolean, found: /* ... */ })
            }
        }
    }
}
```

**Function Signature Table** (partial):
```rust
fn function_signature(name: &str) -> Option<FunctionSignature> {
    match name {
        "SUM" => Some(FunctionSignature { args: vec![ExprType::Number], returns: ExprType::Number }),
        "IF" => Some(FunctionSignature { args: vec![ExprType::Boolean, ExprType::Unknown, ExprType::Unknown], returns: ExprType::Unknown }),
        "CONCAT" => Some(FunctionSignature { args: vec![ExprType::String /* variadic */], returns: ExprType::String }),
        // ...
    }
}
```

---

### Aggregate Context (BR-003)

**Rule**: Aggregate functions only valid when `context.allow_aggregates = true`.

**Implementation**:
```rust
fn validate_aggregate_function(name: &str, allow_aggregates: bool) -> Result<(), ValidationError> {
    const AGGREGATE_FUNCTIONS: &[&str] = &["SUM", "COUNT", "COUNT_ALL", "AVG", "MIN_AGG", "MAX_AGG"];
    
    if AGGREGATE_FUNCTIONS.contains(&name) && !allow_aggregates {
        Err(ValidationError::InvalidAggregateContext { function: name.to_string() })
    } else {
        Ok(())
    }
}
```

---

## Testing Contract

### Unit Tests Required

**Column Resolution**:
- Valid column references resolve successfully
- Invalid table name fails with UnresolvedColumnRef
- Invalid column name fails with UnresolvedColumnRef
- Case sensitivity handling

**Type Inference**:
- Each literal type inferred correctly
- Column references infer from schema
- Binary operators infer correct return type
- Function calls infer from signatures
- Type mismatch detected and reported

**Aggregate Validation**:
- Aggregate functions allowed when context.allow_aggregates=true
- Aggregate functions rejected when context.allow_aggregates=false
- Non-aggregate functions allowed regardless of flag

**Selector Interpolation**:
- Simple selector expansion
- Multiple selectors in one expression
- Nested selectors (up to max depth)
- Unresolved selector error
- Circular reference detection
- No {{}} tokens in result

### Integration Tests Required

**End-to-End Validation**:
```rust
// Valid expression passes all checks
let ast = parse_expression("orders.amount > 100 AND orders.status = \"active\"")?;
let context = CompilationContext::new(schema, HashMap::new(), NaiveDate::default(), false);
let typed_ast = validate_expression(&ast, &context)?;
assert_eq!(typed_ast.return_type(), ExprType::Boolean);

// Invalid column fails
let ast = parse_expression("nonexistent.column")?;
assert!(validate_expression(&ast, &context).is_err());

// Type mismatch fails
let ast = parse_expression("orders.amount + \"text\"")?;
assert!(validate_expression(&ast, &context).is_err());

// Aggregate outside context fails
let ast = parse_expression("SUM(orders.amount)")?;
assert!(validate_expression(&ast, &context).is_err());
```

---

## Versioning & Compatibility

**Stability**: Unstable (pre-1.0)

**Breaking Changes**:
- New validation rules added
- ValidationError variants added
- Type inference rules modified

**Non-Breaking Changes**:
- Error message improvements
- Performance optimizations

---

## References

- Feature spec: `/workspace/docs/specs/S01-dsl-parser/prompt.md`
- Data model: `/workspace/specs/002-dsl-parser/data-model.md`
- Entity documentation: `/workspace/docs/entities/expression.md`
