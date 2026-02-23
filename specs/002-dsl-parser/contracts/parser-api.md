# Parser API Contract

**Module**: `dobo_core::dsl::parser`  
**Version**: 0.1.0  
**Date**: 2026-02-22

## Overview

The Parser API provides functions to parse expression strings into Abstract Syntax Trees (ASTs) using the pest parser generator with a PEG grammar.

---

## Public API

### `parse_expression`

**Signature**:
```rust
pub fn parse_expression(source: &str) -> Result<ExprAST, ParseError>
```

**Description**:
Parses an expression string into an ExprAST tree.

**Parameters**:
- `source: &str` - The expression string to parse (e.g., `"transactions.amount * 1.1"`)

**Returns**:
- `Ok(ExprAST)` - Successfully parsed expression tree
- `Err(ParseError)` - Parse failure with position information

**Errors**:
- `ParseError::SyntaxError` - Invalid grammar (e.g., unmatched parentheses, invalid tokens)
- `ParseError::UnexpectedToken` - Token doesn't match expected grammar rule

**Examples**:
```rust
// Success case
let ast = parse_expression("transactions.amount + 100")?;
assert!(matches!(ast, ExprAST::BinaryOp { op: BinaryOperator::Add, .. }));

// Error case
let result = parse_expression("transactions.amount +");
assert!(matches!(result, Err(ParseError::UnexpectedToken { .. })));
```

**Preconditions**:
- Input must be valid UTF-8
- No maximum length enforced (but performance degrades beyond ~10KB)

**Postconditions**:
- Returned ExprAST contains no unresolved references (semantic validation happens later)
- ParseError includes line and column numbers for error reporting

**Performance**:
- O(n) in expression length
- Target: <1ms for expressions <1KB
- Target: <100ms for batch of 1000 expressions

---

### `parse_expression_with_span`

**Signature**:
```rust
pub fn parse_expression_with_span(source: &str) -> Result<(ExprAST, Span), ParseError>
```

**Description**:
Parses an expression and returns the AST along with the source span for debugging.

**Parameters**:
- `source: &str` - The expression string to parse

**Returns**:
- `Ok((ExprAST, Span))` - AST and span covering entire expression
- `Err(ParseError)` - Parse failure

**Examples**:
```rust
let (ast, span) = parse_expression_with_span("SUM(orders.amount)")?;
assert_eq!(span.start, 0);
assert_eq!(span.end, 18);
```

**Use Case**:
- Debugging and error reporting with source context
- IDE integration (syntax highlighting, autocomplete)

---

## Data Types

### `ExprAST`

See `/workspace/specs/002-dsl-parser/data-model.md` for full definition.

**Summary**:
```rust
pub enum ExprAST {
    Literal(LiteralValue),
    ColumnRef { table: String, column: String },
    BinaryOp { op: BinaryOperator, left: Box<ExprAST>, right: Box<ExprAST> },
    UnaryOp { op: UnaryOperator, operand: Box<ExprAST> },
    FunctionCall { name: String, args: Vec<ExprAST> },
}
```

---

### `ParseError`

See `/workspace/specs/002-dsl-parser/data-model.md` for full definition.

**Summary**:
```rust
pub enum ParseError {
    SyntaxError { line: usize, col: usize, message: String },
    UnexpectedToken { expected: String, found: String, line: usize, col: usize },
}
```

---

### `Span`

**Definition**:
```rust
pub struct Span {
    pub start: usize,  // Byte offset in source
    pub end: usize,    // Byte offset in source
}
```

**Invariants**:
- `start <= end`
- Offsets are byte positions, not character positions

---

## Grammar

The parser implements the following grammar (simplified):

```pest
// Entry point
expression = _{ SOI ~ expr ~ EOI }

// Precedence (lowest to highest via PrattParser)
expr = { prefix* ~ primary ~ postfix* ~ (infix ~ prefix* ~ primary ~ postfix*)* }

// Operators
infix = _{ add | subtract | multiply | divide | eq | ne | lt | le | gt | ge | and | or }
prefix = _{ not | negate }

add = { "+" }
subtract = { "-" }
multiply = { "*" }
divide = { "/" }
eq = { "=" }
ne = { "<>" }
lt = { "<" }
le = { "<=" }
gt = { ">" }
ge = { ">=" }
and = { "AND" }
or = { "OR" }
not = { "NOT" }
negate = { "-" }

// Primaries
primary = _{ literal | column_ref | function_call | "(" ~ expr ~ ")" }

literal = { null | boolean | number | string | date }
null = { "NULL" }
boolean = { "TRUE" | "FALSE" }
number = @{ "-"? ~ ASCII_DIGIT+ ~ ("." ~ ASCII_DIGIT+)? }
string = @{ "\"" ~ (!"\"" ~ ANY)* ~ "\"" }
date = { "DATE(" ~ string ~ ")" }

column_ref = { identifier ~ "." ~ identifier }
function_call = { identifier ~ "(" ~ (expr ~ ("," ~ expr)*)? ~ ")" }

identifier = @{ ASCII_ALPHA ~ (ASCII_ALPHANUMERIC | "_")* }

WHITESPACE = _{ " " | "\t" | "\r" | "\n" }
```

**Precedence Levels** (PrattParser configuration):
1. Logical OR (lowest)
2. Logical AND
3. Comparison (=, <>, <, <=, >, >=)
4. Addition, Subtraction
5. Multiplication, Division (highest)

**Notes**:
- Full grammar is in `crates/core/src/dsl/grammar.pest`
- PrattParser handles precedence and associativity
- All operators are left-associative except NOT (prefix)

---

## Testing Contract

### Unit Tests Required

**Grammar Rules** (one test per rule):
- Literals: numbers, strings, booleans, dates, NULL
- Column references: valid table.column, invalid formats
- Binary operators: each operator, precedence validation
- Unary operators: NOT, negation
- Function calls: zero args, multiple args, nested calls
- Parentheses: grouping, nested grouping

**Edge Cases**:
- Empty string: ParseError
- Whitespace-only: ParseError
- Unclosed string: ParseError
- Unclosed parenthesis: ParseError
- Invalid identifiers: ParseError
- Case sensitivity: keywords uppercase, identifiers case-insensitive

**Error Cases**:
- Position tracking accuracy (line/column)
- Error message clarity
- Multiple errors in one expression (first error reported)

### Integration Tests Required

**Sample Expressions** (from feature spec):
```rust
// Arithmetic
"transactions.amount_local * fx.rate"

// Conditional
"IF(accounts.type = \"revenue\", transactions.amount_local * -1, transactions.amount_local)"

// Boolean selector
"transactions.source_system = \"ERP\" AND transactions.amount_local > 1000"

// String function
"CONCAT(accounts.code, \" - \", accounts.name)"

// Aggregate
"SUM(transactions.amount_local)"

// Date function
"posting_date >= TODAY() - 30"

// NULL handling
"IF(IS_NULL(transactions.amount_reporting), transactions.amount_local, transactions.amount_reporting)"
```

---

## Versioning & Compatibility

**Stability**: Unstable (pre-1.0)

**Breaking Changes**:
- Grammar modifications (add/remove operators, change precedence)
- AST structure changes (add/remove variants)
- Error type changes

**Non-Breaking Changes**:
- Performance improvements
- Error message improvements
- New helper functions

**Deprecation Policy**:
- No deprecations until 1.0 release
- Breaking changes allowed in 0.x versions

---

## References

- Feature spec: `/workspace/docs/specs/S01-dsl-parser/prompt.md`
- Data model: `/workspace/specs/002-dsl-parser/data-model.md`
- Pest documentation: https://pest.rs/
- Grammar file: `crates/core/src/dsl/grammar.pest` (to be created)
