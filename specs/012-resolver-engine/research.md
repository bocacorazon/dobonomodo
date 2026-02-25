# Research: Resolver Rule Evaluation Engine

**Feature**: Resolver Rule Evaluation Engine  
**Branch**: 012-resolver-engine  
**Date**: 2026-02-22

## Research Tasks

This document captures research findings and technical decisions made to resolve unknowns identified during planning.

---

## 1. Template Rendering Approach

### Decision
Custom lightweight template renderer using regex-based token substitution.

### Rationale
1. **Simplicity**: Token vocabulary is fixed and small (period identifier, table name, datasource properties). No need for complex logic, loops, or conditionals in templates.
2. **Zero dependencies**: Avoids adding external template engines (handlebars, tera, liquid) for a simple use case.
3. **Performance**: Direct string replacement with pre-compiled regex patterns is faster than template engine parsing/compilation for simple substitutions.
4. **Predictability**: Template failures are explicit parse-time errors (unknown token) rather than runtime evaluation errors.
5. **Alignment with existing code**: Project uses minimal dependencies; custom solution fits the philosophy (see Cargo.toml workspace deps).

### Alternatives Considered
- **Handlebars/Tera template engine**: Rejected because it adds significant dependency weight for simple string interpolation. These engines support complex logic (loops, conditionals, helpers) that we don't need.
- **format! macro with named arguments**: Rejected because templates are stored as strings in YAML (user-defined), not in Rust source code. Would require runtime code generation.
- **String literal replacement**: Rejected because it lacks structure and error reporting for unknown tokens.

### Implementation Approach
```rust
// Context tokens available:
// {period_id}, {period_name}, {table_name}, {dataset_id}, {datasource_id}
// Custom renderer will:
// 1. Pre-compile regex for each token pattern
// 2. Validate template contains only known tokens
// 3. Replace tokens with values from ResolutionContext
// 4. Return error if token cannot be resolved
```

---

## 2. Expression Evaluation for `when` Conditions

### Decision
Implement simple boolean expression evaluator supporting comparison operators (==, !=, <, >, <=, >=) and logical operators (AND, OR, NOT) using recursive descent parser.

### Rationale
1. **Scope fit**: Spec requires evaluating conditions like `period >= "2024-Q1"` or `table == "sales"`. This is simple comparison logic, not full expression language.
2. **No external evaluator needed**: Libraries like `evalexpr` or `rhai` provide full scripting, which is overkill and introduces security concerns (arbitrary code execution).
3. **Type safety**: Custom evaluator can enforce type constraints (string vs period comparison) at parse time.
4. **Error messages**: Custom implementation provides domain-specific error messages ("period comparison requires ISO format" vs generic "parse error").
5. **Future extensibility**: Can add domain-specific functions (e.g., `in_list()`, `matches_pattern()`) as needed without dependency constraints.

### Alternatives Considered
- **CEL (Common Expression Language)**: Rejected because no mature Rust implementation exists, and it's designed for policy engines (more complex than needed).
- **Rhai scripting language**: Rejected because it's a full scripting language with loops, functions, closures - massive overkill for boolean conditions.
- **evalexpr crate**: Rejected because it's a general math expression evaluator without domain-specific period/calendar awareness.

### Implementation Approach
```rust
// Expression AST:
enum Expr {
    Comparison { left: Value, op: CompOp, right: Value },
    Logical { left: Box<Expr>, op: LogicalOp, right: Box<Expr> },
    Not(Box<Expr>),
    Literal(bool),
}

// Parser produces Expr from when_expression string
// Evaluator walks Expr tree with ResolutionContext values
```

---

## 3. Period Expansion Algorithm

### Decision
Graph traversal using parent-child relationships from Calendar hierarchy with pre-computed level ordering.

### Rationale
1. **Existing data model**: `LevelDef` already has `parent_level`, `Period` has `parent_id`. This is a natural tree structure.
2. **Correct semantics**: Spec requires "using calendar hierarchy (not inferred arithmetic)" - graph traversal ensures we use actual defined relationships.
3. **Deterministic ordering**: Tree traversal with stable child ordering (by `sequence` field in Period) produces deterministic output order.
4. **Flexibility**: Works for any calendar structure (fiscal, weekly, custom hierarchies) without hardcoded assumptions.

### Alternatives Considered
- **Arithmetic date calculation**: Rejected because spec explicitly requires hierarchy traversal, and not all calendars follow arithmetic rules (e.g., 4-4-5 retail calendar).
- **SQL query for descendants**: Rejected because resolver engine is in core library (no database access); metadata loading is caller's responsibility.
- **Lazy expansion on-demand**: Rejected because all child periods must be known upfront to generate complete location list.

### Implementation Approach
```rust
// 1. Load requested Period from metadata store (caller responsibility)
// 2. Determine target level from rule's data_level
// 3. If data_level == "any", return single period (no expansion)
// 4. If requested level == data_level, return single period
// 5. Otherwise: traverse Period.parent_id chain until reaching data_level
// 6. Collect all descendants at data_level
// 7. Sort by sequence for deterministic ordering
```

---

## 4. Resolver Precedence Strategy

### Decision
Implement precedence selection as a three-level fallback: project override → dataset resolver reference → system default.

### Rationale
1. **Spec requirement**: FR-005 explicitly defines the precedence order.
2. **Flexibility**: Allows project-level customization while maintaining dataset-specific and system-wide defaults.
3. **Clear semantics**: Precedence is evaluated at resolution time, not at configuration time, so updates take effect immediately.

### Alternatives Considered
- **Configuration merge**: Rejected because merging rules from multiple resolvers creates ambiguity (which rule wins? how to order merged list?).
- **Resolver inheritance**: Rejected because it adds complexity without clear benefit (would need to define inheritance semantics for rule lists).

### Implementation Approach
```rust
// Resolution request includes optional project_id and dataset_id
// Precedence check:
// 1. If project_id provided, load project.resolver_override_id → use if exists
// 2. Else if dataset_id provided, load dataset.resolver_id → use if exists
// 3. Else load system default resolver (status=Active, is_default=true)
// 4. If none found, return error with resolver-selection diagnostic
```

---

## 5. Diagnostic Structure

### Decision
Structured diagnostic output containing: resolver selection trace, evaluated rules with match/no-match reasons, and final resolution outcome.

### Rationale
1. **Troubleshooting**: Spec requires diagnostics for no-match scenarios and precedence decisions (FR-004, US3).
2. **Observability**: Operators need to understand why specific resolver/rule was selected.
3. **Structured format**: JSON-serializable diagnostic allows logging, monitoring, and tooling integration.

### Alternatives Considered
- **String-based error messages**: Rejected because unstructured text is hard to parse for tooling and doesn't support filtering/querying.
- **Minimal error codes**: Rejected because operators need detailed context (which rules evaluated, what values compared) for troubleshooting.

### Implementation Approach
```rust
pub struct ResolutionDiagnostic {
    pub resolver_id: String,
    pub resolver_source: ResolverSource, // ProjectOverride | DatasetReference | SystemDefault
    pub evaluated_rules: Vec<RuleDiagnostic>,
    pub outcome: DiagnosticOutcome, // Success | NoMatch | ExpansionFailure | TemplateError
}

pub struct RuleDiagnostic {
    pub rule_name: String,
    pub matched: bool,
    pub reason: String, // "condition evaluated to true" | "when: period < '2024' failed"
}
```

---

## 6. Integration with Existing Pipeline

### Decision
Resolver engine is invoked by engine-worker during Run execution, before data loading phase.

### Rationale
1. **Architecture alignment**: System architecture doc shows engine-worker executes Polars pipeline. Resolver provides input locations.
2. **Separation of concerns**: Resolver engine is pure logic (no I/O). Engine-worker handles metadata loading (PostgreSQL) and provides Calendar/Period data.
3. **Testability**: Core library function can be tested with in-memory fixtures (no database required).

### Implementation Approach
```rust
// In crates/core/src/resolver/engine.rs:
pub fn resolve(
    request: ResolutionRequest,
    resolver: Resolver,
    calendar: Calendar,
    periods: Vec<Period>, // Pre-loaded by caller
) -> Result<ResolutionResult, ResolutionError> {
    // 1. Select first matching rule
    // 2. Expand periods if needed
    // 3. Render templates for each period
    // 4. Return Vec<ResolvedLocation> + diagnostics
}

// Engine-worker calls:
// let result = resolver::engine::resolve(req, resolver, calendar, periods)?;
```

---

## Summary

All technical clarifications resolved. Key decisions:
- **Template rendering**: Custom regex-based substitution (simple, fast, zero deps)
- **Expression evaluation**: Custom recursive descent parser (domain-specific, type-safe)
- **Period expansion**: Graph traversal using existing Calendar hierarchy (correct semantics)
- **Resolver precedence**: Three-level fallback (project → dataset → system default)
- **Diagnostics**: Structured output with full evaluation trace (troubleshooting support)
- **Integration**: Pure function in core library, called by engine-worker (testable, clean separation)

Ready to proceed to Phase 1 (Design & Contracts).
