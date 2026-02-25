# Feature Specification: DSL Parser and Expression Compiler

**Feature Branch**: `002-dsl-parser`  
**Created**: 2026-02-22  
**Status**: Draft  
**Input**: User description: "using docs/specs/S01-dsl-parser"

## User Scenarios & Testing *(mandatory)*

### User Story 1 - Validate Expressions Before Activation (Priority: P1)

As a project author, I want expressions to be validated before a project is activated so invalid formulas do not fail during run execution.

**Why this priority**: Activation safety is a core gate for reliable execution and prevents broken projects from advancing.

**Independent Test**: Submit a project containing valid and invalid expressions and verify activation is blocked only for invalid expressions with clear failures.

**Acceptance Scenarios**:

1. **Given** a project with valid selectors and assignment formulas, **When** validation is triggered, **Then** all expressions are accepted as valid for their intended usage context.
2. **Given** a project with malformed expression syntax, **When** validation is triggered, **Then** the system rejects activation and reports parse failures with error location.

---

### User Story 2 - Resolve References Reliably (Priority: P2)

As a project author, I want selector names and column references to resolve consistently so formulas always point to defined data.

**Why this priority**: Reference correctness is required for predictable behavior and prevents silent data errors.

**Independent Test**: Validate expressions against a dataset schema and project selector map, including both valid and invalid references.

**Acceptance Scenarios**:

1. **Given** an expression containing a named selector token, **When** that selector exists in the project selector map, **Then** the expression is expanded and validated successfully.
2. **Given** an expression referencing a missing selector or undefined column, **When** validation is triggered, **Then** validation fails with an explicit unresolved reference error.

---

### User Story 3 - Enforce Expression Type and Context Rules (Priority: P3)

As a project reviewer, I want expression type and aggregate usage rules enforced so operation behavior remains logically correct.

**Why this priority**: Enforcing semantic rules early prevents invalid transformations and reduces downstream debugging effort.

**Independent Test**: Validate expressions across selector, assignment, and aggregation contexts to confirm allowed and disallowed patterns are enforced.

**Acceptance Scenarios**:

1. **Given** a selector that does not evaluate to a boolean condition, **When** validation is triggered, **Then** validation fails with a type mismatch error.
2. **Given** an aggregate expression used in a non-aggregate context, **When** validation is triggered, **Then** validation fails with an invalid aggregate context error.

### Edge Cases

- A named selector reference token is syntactically present but the referenced selector name is not defined.
- An expression references a join alias that is not available in the current operation scope.
- An expression is syntactically valid but mixes incompatible value types in a comparison or arithmetic operation.
- An expression includes nested function calls where one inner argument is invalid, and the error must still identify the precise failing location.
- A date-relative expression uses run-relative date functions and must resolve deterministically for the same run snapshot.

## Requirements *(mandatory)*

### Functional Requirements

- **FR-001**: The system MUST parse expression strings used in project selectors and operation parameters into a structured expression representation.
- **FR-002**: The system MUST support literals, column references, infix operators, function calls, NULL values, and parenthesized expressions defined by the expression DSL.
- **FR-003**: The system MUST support named selector interpolation using `{{NAME}}` tokens before final validation.
- **FR-004**: The system MUST reject any expression containing a selector reference token that does not resolve to a defined project selector.
- **FR-005**: The system MUST validate that every column reference resolves against the active dataset schema or operation-scoped join aliases.
- **FR-006**: The system MUST reject unresolved column or alias references with explicit validation errors.
- **FR-007**: The system MUST enforce expression type compatibility rules for each usage context, including boolean-only contexts for row filters.
- **FR-008**: The system MUST enforce aggregate-function context rules and reject aggregate functions used outside allowed aggregation contexts.
- **FR-009**: The system MUST produce a validated executable expression artifact for each accepted expression so downstream pipeline execution can consume it consistently.
- **FR-010**: The system MUST report expression validation failures with error category and source position information sufficient for user correction.
- **FR-011**: The system MUST treat run-relative date functions deterministically using the run snapshot time context.
- **FR-012**: The system MUST preserve defined NULL semantics for expression validation and compilation behavior.

### Key Entities *(include if feature involves data)*

- **Expression Source**: User-authored DSL string embedded in selectors, assignments, join conditions, and aggregations.
- **Named Selector Map**: Project-scoped map of reusable boolean expressions referenced by `{{NAME}}` tokens.
- **Dataset Schema Context**: Declared table and column contract used to resolve expression column references.
- **Join Alias Context**: Operation-scoped alias definitions that extend available column references within the same operation.
- **Validation Result**: Structured success/failure output containing error categories, positions, and unresolved reference details.
- **Executable Expression Artifact**: Validated expression form consumed by downstream operation execution.

## Assumptions

- Workspace scaffold baseline capabilities are available and provide required entity definitions and project structure.
- Expression validation is performed before project activation and reused by run execution paths.
- Window functions, cross-table aggregates beyond declared scope, and implicit coercion extensions remain out of scope for this feature.
- Existing project selector and dataset definitions are treated as source of truth during interpolation and resolution.

## Success Criteria *(mandatory)*

### Measurable Outcomes

- **SC-001**: 100% of curated reference expressions for arithmetic, conditional, boolean, string, date, aggregate, selector interpolation, and NULL handling validate successfully when inputs are valid.
- **SC-002**: 100% of invalid reference expressions in the validation suite are rejected with the correct error category.
- **SC-003**: 100% of unresolved selector and unresolved column test cases are detected before activation.
- **SC-004**: 100% of aggregate-context misuse cases are rejected in non-aggregate contexts.
- **SC-005**: At least 95% of expression error reports include precise source position information that enables first-attempt correction by reviewers.
