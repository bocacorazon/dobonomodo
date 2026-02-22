# S01: DSL Parser & Expression Compiler

## Feature
Parse expression strings from the YAML DSL into an AST, perform type-checking and column resolution, interpolate `{{SELECTOR}}` references, and compile the AST into Polars `Expr` objects that can be attached to `LazyFrame` operations.

## Context
- Read: `docs/entities/expression.md` (expression syntax, function categories, type rules, NULL propagation)
- Read: `docs/entities/operation.md` (how expressions are used in selectors, assignments, join conditions, aggregations)
- Read: `docs/entities/dataset.md` (column schema for resolution — `ColumnDef`, `ColumnType`, system columns)
- Read: `docs/entities/project.md` (`selectors` map for `{{NAME}}` interpolation)
- Read: `docs/architecture/system-architecture.md` (DSL compilation pipeline section)
- Read: `docs/architecture/sample-datasets.md` (sample expressions to test against)

## Scope

### In Scope
- Expression grammar definition (pest or lalrpop — choose one and document rationale)
- Parser: `&str` → `ExprAST` (abstract syntax tree)
- AST node types: literals (string, integer, decimal, boolean, date), column references (`table.column`), binary operators (arithmetic, comparison, logical), function calls (all 5 categories from expression.md), `NULL`
- `{{SELECTOR}}` interpolation: detect `{{NAME}}` tokens, substitute from a `Map<String, String>`, then re-parse the expanded string
- Column resolution: given a `DatasetSchema` + optional `Vec<JoinAlias>`, validate that every column reference resolves. Return `UnresolvedColumnRef` errors
- Type checking: validate expression type compatibility (e.g., boolean selectors, assignment type matches, aggregate functions only in aggregate context). Return `TypeMismatch` errors
- Polars `Expr` generation: compile a validated AST into a `polars::lazy::dsl::Expr`
- Function mapping: map each DSL function to its Polars equivalent (e.g., `SUM` → `col().sum()`, `IF` → `when/then/otherwise`, `CONCAT` → `concat_str`)
- Error types: `ParseError`, `UnresolvedColumnRef`, `TypeMismatch`, `UnresolvedSelectorRef`, `InvalidAggregateContext`

### Out of Scope
- Executing expressions against actual data — that's S03+
- Window/ranking functions (deferred per expression.md OQ-001)
- Cross-table aggregates (deferred per expression.md OQ-002)
- Type coercion rules (deferred per expression.md OQ-003)

## Dependencies
- **S00** (Workspace Scaffold): entity model structs, `ColumnType` enum, `Expression` newtype

## Parallel Opportunities
This spec can run in parallel with **S02** (Test Harness), **S16** (DataSource Adapters), **S17** (Metadata Store).

## Key Design Decisions
- Excel-style function names (SUM, IF, CONCAT, etc.)
- Infix operators for arithmetic and comparison
- Column references as `logical_table.column_name`
- NULL propagation is explicit (no auto-coalesce)
- `TODAY()` resolves to a provided timestamp (Run's `started_at`), not live time
- Aggregate functions (`SUM`, `COUNT`, `AVG`, `MIN_AGG`, `MAX_AGG`) are valid only in aggregate/append-with-aggregation context — compile error elsewhere

## Sample Expressions for Testing

```
# Arithmetic
transactions.amount_local * fx.rate

# Conditional
IF(accounts.type = "revenue", transactions.amount_local * -1, transactions.amount_local)

# Boolean (selector)
transactions.source_system = "ERP" AND transactions.amount_local > 1000

# String function
CONCAT(accounts.code, " - ", accounts.name)

# Aggregate (valid only in aggregate context)
SUM(transactions.amount_local)
COUNT(transactions.journal_id)

# Named selector interpolation
{{EMEA_ONLY}}

# Date function
posting_date >= TODAY() - 30

# NULL handling
IF(IS_NULL(transactions.amount_reporting), transactions.amount_local, transactions.amount_reporting)
```

## Success Criteria
- All sample expressions above parse correctly
- Invalid expressions produce clear error messages with position info
- Column resolution catches references to non-existent columns
- Type checker catches boolean expressions used as assignments and vice versa
- Aggregate functions in non-aggregate context produce compile error
- `{{NAME}}` references to undefined selectors produce `UnresolvedSelectorRef`
- Generated Polars `Expr` objects are valid (can be attached to a dummy `LazyFrame` without panic)
