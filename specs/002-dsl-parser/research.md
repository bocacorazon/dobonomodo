# Research: DSL Parser & Expression Compiler

**Feature**: 002-dsl-parser  
**Date**: 2026-02-22  
**Status**: Complete

## Overview

This document captures research findings and technical decisions made during Phase 0 of the DSL Parser & Expression Compiler feature. The primary research question was: **Which parser generator should we use: pest or lalrpop?**

## Research Questions & Findings

### 1. Parser Generator Choice: pest vs lalrpop

**Context**: We need to parse DobONoMoDo expression strings with:
- Excel-style function calls (SUM, IF, CONCAT, etc.)
- Infix operators with precedence (arithmetic, comparison, logical)
- Column references (table.column syntax)
- Literals (numbers, strings, booleans, dates, NULL)
- Selector interpolation ({{NAME}} references)

#### Pest Analysis

**Characteristics**:
- PEG-based (Parsing Expression Grammars) - deterministic, no backtracking conflicts
- Automatic position-aware error reporting (line/column tracking)
- 5,287 GitHub stars, actively maintained, Rust 1.83+ support
- External grammar files (.pest) separate from Rust code
- Built-in PrattParser for elegant operator precedence handling

**Strengths for this use case**:
- PrattParser makes operator precedence trivial to define
- Intuitive PEG-like syntax (similar to regex)
- Macro-driven: pest_derive auto-generates parser from grammar
- Zero Polars compatibility concerns (generates AST iterators, compiler maps to Polars)
- Excellent for configuration DSLs and expression parsers
- Project already has guidance at `.github/instructions/rust/pest.instructions.md`

**Weaknesses**:
- PEG can have subtle left-recursion issues (not a concern for our expression grammar)
- Less conventional than LR parsers for some domains

#### LALRPOP Analysis

**Characteristics**:
- LR(1)/LALR(1) parser generator
- 3,400 GitHub stars, active development (0.23.0 latest)
- Native operator precedence declarations (%left, %right, %nonassoc)
- Rust-first design with compile-time code generation
- Strong conflict detection and resolution guidance

**Strengths for this use case**:
- Native precedence declarations handle arbitrary hierarchies elegantly
- Excellent error messages with grammar conflict detection
- Well-suited for SQL-like expression grammars
- Widely used in production (RustPython, Gluon, SQL parsers)
- Good documentation with tutorial examples

**Weaknesses**:
- Moderate learning curve (LR theory less intuitive than PEG)
- Requires understanding of shift/reduce conflicts
- More verbose for simple grammars
- No error recovery during partial parsing

### Decision: **pest**

**Rationale**:
1. **Simplicity**: Pest's PEG approach is more natural for Excel-like expression syntax without requiring shift/reduce conflict resolution
2. **Precedence handling**: PrattParser provides elegant solution for our operator precedence needs (arithmetic < comparison < logical)
3. **Existing guidance**: Project already has pest instructions at `.github/instructions/rust/pest.instructions.md`
4. **Ergonomics**: Separated .pest grammar files keep concerns clean; macro-driven generation reduces boilerplate
5. **Performance**: Meets our target (<100ms for 1000 expressions) with deterministic PEG parsing
6. **Zero integration risk**: No Polars compatibility concerns; pest output is AST that our compiler transforms

**Alternative considered**: lalrpop
- **Why rejected**: While lalrpop has excellent precedence handling and is production-proven, its LR-based approach adds unnecessary complexity for our expression grammar. Pest's PEG model is more intuitive for the team and better matches the DSL's structure. The learning curve difference favors pest for a team new to parser generators.

### 2. Best Practices for Rust DSL Parsing

**Findings from research**:

**Error Handling**:
- Use thiserror for custom error types (already in workspace dependencies)
- Preserve position information (line, column, span) in all errors
- Provide clear error messages with context snippets
- Error types: ParseError, UnresolvedColumnRef, TypeMismatch, UnresolvedSelectorRef, InvalidAggregateContext

**Type Checking Strategy**:
- Infer return types bottom-up during AST traversal
- Column types come from DatasetSchema (ColumnDef)
- Function return types are predefined (e.g., SUM → number, IF → matches branch types)
- Validate type compatibility at usage sites (assignments, selectors, function arguments)

**Testing Strategy**:
- Unit tests for parser: each grammar rule, edge cases, invalid syntax
- Unit tests for type checker: type mismatches, aggregate context violations
- Unit tests for compiler: each DSL function → Polars Expr mapping
- Integration tests: end-to-end with sample expressions from spec
- Contract tests: validate generated Polars Expr objects (attach to dummy LazyFrame)

### 3. Polars Integration Patterns

**Findings**:

**Expr Compilation Mapping**:
- Literals: `lit(value)` for constants
- Column references: `col("table.column")` with qualified names
- Arithmetic: `col(...).add(...)`, `col(...).mul(...)`, etc.
- Comparison: `col(...).eq(...)`, `col(...).gt(...)`, etc.
- Logical: `col(...).and(...)`, `col(...).or(...)`, `col(...).not()`
- Functions:
  - SUM: `col(...).sum()`
  - IF: `when(...).then(...).otherwise(...)`
  - CONCAT: `concat_str([col(...), col(...), ...], "")`
  - UPPER: `col(...).str().to_uppercase()`
  - DATE functions: use chrono integration with Polars temporal types

**Performance Considerations**:
- Use lazy API throughout (no eager evaluation)
- Polars Expr is immutable and composable
- No runtime overhead from AST → Expr compilation (happens once at Project activation)

**Validation**:
- Generated Expr objects can be validated by attaching to a dummy LazyFrame
- Type mismatches will panic at LazyFrame evaluation (caught during compile phase)

### 4. Selector Interpolation Strategy

**Decision**: Two-phase approach

**Rationale**:
1. **Detection**: Scan expression string for `{{NAME}}` tokens using regex
2. **Substitution**: Replace each `{{NAME}}` with the corresponding expression from Project.selectors map
3. **Re-parse**: Parse the expanded string to produce the final AST
4. **Validation**: If selector not found in map, return UnresolvedSelectorRef error

**Alternative considered**: AST-level interpolation
- **Why rejected**: String-level substitution is simpler and allows selectors to contain arbitrary expression syntax without special AST node types. Re-parsing cost is negligible (selectors are expanded once during Project activation).

**Edge cases**:
- Circular selector references: detect during substitution (track expansion stack)
- Nested selectors: recursive expansion until no more {{}} tokens
- Selector containing {{}} itself: disallowed (validation error)

## Technology Choices Summary

| Component | Technology | Rationale |
|-----------|-----------|-----------|
| Parser Generator | **pest** | PEG simplicity, PrattParser for precedence, existing project guidance |
| Error Handling | **thiserror** | Already in workspace, ergonomic derive macros |
| AST Representation | Custom Rust enums | Type-safe, pattern matching, clear semantics |
| Type System | Bottom-up inference | Natural for expression trees, validates constraints early |
| Polars Integration | Direct Expr mapping | Lazy API, immutable composition, zero overhead |
| Selector Interpolation | String substitution + re-parse | Simple, flexible, supports arbitrary selector expressions |

## Open Questions (None)

All research questions have been resolved. No blocking unknowns remain.

## References

- Pest documentation: https://pest.rs/
- LALRPOP documentation: https://lalrpop.github.io/lalrpop/
- Polars lazy API: https://docs.rs/polars/latest/polars/lazy/
- Project entity docs: /workspace/docs/entities/expression.md
- Feature spec: /workspace/docs/specs/S01-dsl-parser/prompt.md
