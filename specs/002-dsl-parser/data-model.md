# Data Model: DSL Parser & Expression Compiler

**Feature**: 002-dsl-parser  
**Date**: 2026-02-22  
**Status**: Draft

## Overview

This document defines the data structures and relationships for the DSL parser and expression compiler module. The parser transforms expression strings into validated Abstract Syntax Trees (ASTs), performs type checking and column resolution, and compiles ASTs into Polars Expr objects.

## Core Entities

### 1. ExprAST (Abstract Syntax Tree Node)

**Purpose**: Represents a parsed expression as a typed tree structure.

**Variants**:

```rust
pub enum ExprAST {
    // Literals
    Literal(LiteralValue),
    
    // Column reference
    ColumnRef {
        table: String,      // Logical table name
        column: String,     // Column name
    },
    
    // Binary operations
    BinaryOp {
        op: BinaryOperator,
        left: Box<ExprAST>,
        right: Box<ExprAST>,
    },
    
    // Unary operations
    UnaryOp {
        op: UnaryOperator,
        operand: Box<ExprAST>,
    },
    
    // Function calls
    FunctionCall {
        name: String,
        args: Vec<ExprAST>,
    },
}

pub enum LiteralValue {
    Number(f64),
    String(String),
    Boolean(bool),
    Date(chrono::NaiveDate),
    Null,
}

pub enum BinaryOperator {
    // Arithmetic
    Add, Subtract, Multiply, Divide,
    
    // Comparison
    Equal, NotEqual, LessThan, LessThanOrEqual, GreaterThan, GreaterThanOrEqual,
    
    // Logical
    And, Or,
}

pub enum UnaryOperator {
    Not,
    Negate,
}
```

**Attributes**:
- Immutable after construction
- No lifetime dependencies (owned strings)
- Recursive structure via Box<> for child nodes

**Relationships**:
- Parent: None (root of expression tree)
- Children: Nested ExprAST nodes for operands and arguments

**Validation Rules**:
- All column references must resolve to a ColumnDef in the provided DatasetSchema
- Binary operators must have compatible operand types
- Function calls must have correct argument count and types
- Aggregate functions only valid in aggregate context

---

### 2. ExprType (Type Annotation)

**Purpose**: Represents the inferred or validated type of an expression.

**Definition**:

```rust
pub enum ExprType {
    Number,
    String,
    Boolean,
    Date,
    Null,
    Unknown,  // Placeholder during type inference
}
```

**Type Inference Rules**:
- Literals: type is explicit from LiteralValue variant
- Column references: type from ColumnDef.column_type
- Arithmetic operators: Number × Number → Number
- Comparison operators: T × T → Boolean (where T matches)
- Logical operators: Boolean × Boolean → Boolean
- Function calls: return type from function signature

**Relationships**:
- Associated with each ExprAST node during type checking
- Derived from ColumnType (in model/dataset.rs) for column references

---

### 3. ParseError

**Purpose**: Represents errors during expression parsing.

**Definition**:

```rust
#[derive(Debug, thiserror::Error)]
pub enum ParseError {
    #[error("Syntax error at line {line}, column {col}: {message}")]
    SyntaxError {
        line: usize,
        col: usize,
        message: String,
    },
    
    #[error("Expected {expected}, found {found} at line {line}, column {col}")]
    UnexpectedToken {
        expected: String,
        found: String,
        line: usize,
        col: usize,
    },
}
```

**Attributes**:
- Includes position information (line, column) from pest parser
- Human-readable error messages
- Implements std::error::Error via thiserror

**State Transitions**:
- Created during parse phase if input violates grammar
- Propagated up call stack (no recovery)

---

### 4. ValidationError

**Purpose**: Represents errors during semantic validation (type checking, column resolution).

**Definition**:

```rust
#[derive(Debug, thiserror::Error)]
pub enum ValidationError {
    #[error("Unresolved column reference: {table}.{column}")]
    UnresolvedColumnRef {
        table: String,
        column: String,
    },
    
    #[error("Type mismatch: expected {expected:?}, found {found:?}")]
    TypeMismatch {
        expected: ExprType,
        found: ExprType,
    },
    
    #[error("Unresolved selector reference: {{{selector}}}")]
    UnresolvedSelectorRef {
        selector: String,
    },
    
    #[error("Aggregate function {function} used outside aggregate context")]
    InvalidAggregateContext {
        function: String,
    },
    
    #[error("Circular selector reference: {chain}")]
    CircularSelectorRef {
        chain: String,
    },
}
```

**Attributes**:
- Specific error variants for each validation failure type
- Context-rich messages for debugging
- Implements std::error::Error via thiserror

**State Transitions**:
- Created during type checking or column resolution phases
- Collected and reported (may accumulate multiple errors)

---

### 5. CompilationContext

**Purpose**: Carries contextual information needed during expression compilation.

**Definition**:

```rust
pub struct CompilationContext {
    /// Schema for column resolution
    pub schema: DatasetSchema,
    
    /// Join aliases (if operation has joins)
    pub join_aliases: Vec<String>,
    
    /// Named selectors from Project for {{NAME}} interpolation
    pub selectors: HashMap<String, String>,
    
    /// Current date/time for TODAY() resolution (from Run.started_at)
    pub today: chrono::NaiveDate,
    
    /// Whether aggregate functions are allowed (true for aggregate/rollup operations)
    pub allow_aggregates: bool,
}
```

**Attributes**:
- Immutable during compilation of a single expression
- Constructed from Project + Operation + Run snapshots
- Passed by reference through compilation pipeline

**Relationships**:
- References DatasetSchema (from model/dataset.rs)
- References Project.selectors (from model/project.rs)
- References Run.started_at (from model/run.rs)

---

### 6. CompiledExpression

**Purpose**: Result of successful expression compilation.

**Definition**:

```rust
pub struct CompiledExpression {
    /// Original expression source
    pub source: String,
    
    /// Compiled Polars expression
    pub expr: polars::lazy::dsl::Expr,
    
    /// Inferred return type
    pub return_type: ExprType,
}
```

**Attributes**:
- Immutable after construction
- Contains both original source (for debugging) and compiled target
- Return type cached for validation

**Relationships**:
- Created from ExprAST + CompilationContext
- Consumed by execution engine (S03+)

---

## Entity Relationships

```
Expression (model/expression.rs)
    ↓ parse
ExprAST (dsl/ast.rs)
    ↓ validate (with CompilationContext)
TypedExprAST (ExprAST + ExprType annotations)
    ↓ compile
CompiledExpression (contains polars::Expr)
```

**Dependencies**:
- ExprAST → ParseError (error case)
- TypedExprAST → ValidationError (error case)
- CompilationContext → DatasetSchema, Project, Run
- CompiledExpression → polars::lazy::dsl::Expr

---

## State Transitions

### Expression Compilation Pipeline

```
[Expression.source: String]
    ↓ (1) Selector interpolation
[Expanded source: String]
    ↓ (2) Parse
[ExprAST] or [ParseError]
    ↓ (3) Column resolution
[ExprAST] or [ValidationError::UnresolvedColumnRef]
    ↓ (4) Type checking
[TypedExprAST] or [ValidationError::TypeMismatch/InvalidAggregateContext]
    ↓ (5) Polars compilation
[CompiledExpression]
```

**Phase 1: Selector Interpolation**
- Input: Expression.source + CompilationContext.selectors
- Process: Detect {{NAME}} tokens, substitute from selectors map, re-parse
- Output: Expanded expression string
- Errors: UnresolvedSelectorRef, CircularSelectorRef

**Phase 2: Parse**
- Input: Expanded expression string
- Process: pest parser applies grammar, builds ExprAST
- Output: ExprAST tree
- Errors: ParseError (SyntaxError, UnexpectedToken)

**Phase 3: Column Resolution**
- Input: ExprAST + CompilationContext.schema
- Process: Traverse AST, validate all ColumnRef nodes exist in schema
- Output: ExprAST (unchanged)
- Errors: ValidationError::UnresolvedColumnRef

**Phase 4: Type Checking**
- Input: ExprAST + CompilationContext.schema
- Process: Bottom-up type inference, validate type constraints
- Output: TypedExprAST (ExprAST + ExprType annotations)
- Errors: ValidationError::TypeMismatch, ValidationError::InvalidAggregateContext

**Phase 5: Polars Compilation**
- Input: TypedExprAST + CompilationContext
- Process: Map AST nodes to Polars Expr constructors
- Output: CompiledExpression
- Errors: None (validation complete)

---

## Validation Rules

### Column Resolution (BR-001)
- All ColumnRef nodes must match a table.column in DatasetSchema
- Table name must match a logical table in the Dataset
- Column name must exist in that table's ColumnDef list
- Failure: UnresolvedColumnRef error

### Type Compatibility (BR-002)
- Binary operators require compatible operand types
  - Arithmetic: both Number
  - Comparison: both same type (Number, String, Date, Boolean)
  - Logical: both Boolean
- Function arguments must match signature types
- Assignment expressions must match target column type
- Selector expressions must return Boolean
- Failure: TypeMismatch error

### Aggregate Context (BR-003)
- Aggregate functions (SUM, COUNT, AVG, MIN_AGG, MAX_AGG) require allow_aggregates=true
- Only valid in aggregate/rollup operations (determined by Operation.kind)
- Failure: InvalidAggregateContext error

### Selector References (BR-004)
- {{NAME}} tokens must resolve to a key in Project.selectors
- Circular references detected via expansion stack
- Failure: UnresolvedSelectorRef or CircularSelectorRef error

---

## Domain Invariants

1. **Immutability**: All AST nodes are immutable after construction
2. **Type Safety**: TypedExprAST nodes have valid ExprType annotations
3. **Validation Order**: Type checking cannot succeed if column resolution fails
4. **Compilation Guarantee**: CompiledExpression only exists if all validation passes
5. **Error Position Tracking**: All ParseError instances include line/column information
6. **No Partial State**: Expression compilation is all-or-nothing (no partial results)

---

## References

- Entity documentation: /workspace/docs/entities/expression.md
- Feature spec: /workspace/docs/specs/S01-dsl-parser/prompt.md
- Existing model: /workspace/crates/core/src/model/expression.rs
- Polars Expr API: https://docs.rs/polars/latest/polars/lazy/dsl/struct.Expr.html
