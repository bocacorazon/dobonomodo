# Feature Specification: DSL Parser

**Feature Branch**: `[003-dsl-parser]`  
**Created**: 2026-02-22  
**Status**: Draft  
**Input**: User description: "Create DSL parser"

## User Scenarios & Testing *(mandatory)*

### User Story 1 - Parse valid expressions (Priority: P1)

As a pipeline author, I want valid DSL expressions to be accepted consistently so I can define transformation logic without manual rewrites.

**Why this priority**: Parsing valid expressions is the core value of the feature and enables all downstream behavior.

**Independent Test**: Submit a set of valid expressions from project examples and confirm each expression is accepted and represented in a normalized parsed form.

**Acceptance Scenarios**:

1. **Given** a valid arithmetic expression, **When** the user validates the project, **Then** the expression is accepted without errors.
2. **Given** a valid conditional expression using supported functions, **When** the user validates the project, **Then** the expression is accepted and available for downstream operation checks.

---

### User Story 2 - Receive actionable validation feedback (Priority: P2)

As a pipeline author, I want precise parser and validation errors so I can quickly fix invalid expressions.

**Why this priority**: Fast correction of invalid expressions reduces rework and avoids run failures.

**Independent Test**: Submit expressions with syntax mistakes, unknown selectors, and unknown columns, then confirm each failure includes clear issue type and location guidance.

**Acceptance Scenarios**:

1. **Given** an expression with invalid syntax, **When** validation runs, **Then** the user receives an error that identifies the failing position and reason.
2. **Given** an expression with an unresolved reference, **When** validation runs, **Then** the user receives an error that names the unresolved reference.

---

### User Story 3 - Enforce expression context rules (Priority: P3)

As a project owner, I want expression usage rules enforced by context so invalid operation definitions are rejected before execution.

**Why this priority**: Context validation prevents incorrect project configurations from reaching run-time.

**Independent Test**: Validate operation definitions that intentionally misuse expression types or aggregate expressions and confirm each invalid case is rejected with clear guidance.

**Acceptance Scenarios**:

1. **Given** an aggregate expression in a non-aggregate context, **When** validation runs, **Then** validation rejects the expression with a context-specific error.
2. **Given** a non-boolean selector expression, **When** validation runs, **Then** validation rejects it and explains the expected expression type.

---

### Edge Cases

- Empty or whitespace-only expressions are rejected with a clear "expression required" message.
- Very long expressions (up to 500 characters) remain parseable and return deterministic results.
- Nested function calls that are syntactically valid but semantically invalid are rejected with semantic guidance.
- Unknown selector placeholders are reported without masking additional syntax errors in the same expression.
- Ambiguous references that match multiple candidate columns are rejected with disambiguation guidance.

## Scope Boundaries

### In Scope

- Parsing and validating DSL expressions used in project operation definitions.
- Resolving selector and schema references needed to validate expression correctness.
- Returning structured, actionable parse and validation feedback before execution.

### Out of Scope

- Executing expressions against runtime datasets.
- Defining new expression language constructs outside current documented rules.
- Changing dataset schema modeling behavior outside expression validation needs.

## Requirements *(mandatory)*

### Functional Requirements

- **FR-001**: The system MUST accept DSL expression input used by project operations and evaluate it during validation.
- **FR-002**: The system MUST support literals, column references, operators, function calls, and selector placeholders defined in project expression rules.
- **FR-003**: The system MUST produce a normalized parsed representation for every valid expression.
- **FR-004**: The system MUST validate that every selector placeholder resolves to a defined selector.
- **FR-005**: The system MUST validate that every referenced column resolves against the relevant dataset schema and aliases.
- **FR-006**: The system MUST enforce expression context rules, including aggregate-only and selector-type constraints.
- **FR-007**: The system MUST return structured validation errors that include issue category, expression location, and human-readable remediation guidance.
- **FR-008**: The system MUST produce deterministic parsing and validation outcomes for the same input and schema context.
- **FR-009**: The system MUST allow users to validate expressions independently of execution so errors are surfaced before a run starts.
- **FR-010**: The system MUST preserve expression intent when normalized so downstream consumers can evaluate equivalent logic without semantic drift.

### Key Entities *(include if feature involves data)*

- **Expression Input**: User-authored DSL text attached to an operation definition.
- **Parsed Expression**: Canonical representation of expression structure used for validation and downstream consumption.
- **Expression Validation Issue**: Structured error record containing issue type, location, and remediation guidance.
- **Selector Definition**: Named reusable condition that can be referenced from expressions.
- **Schema Reference Context**: Available logical tables, aliases, and column names used to resolve expression references.

## Assumptions

- Expression syntax and supported function families follow current project entity documentation.
- Dataset schema metadata is available at validation time for reference resolution.
- Expression validation runs before pipeline execution and blocks invalid project definitions from proceeding.
- Function names are treated consistently regardless of input letter case.

## Success Criteria *(mandatory)*

### Measurable Outcomes

- **SC-001**: 100% of approved sample valid expressions in the feature acceptance suite are accepted during validation.
- **SC-002**: 100% of approved sample invalid expressions in the feature acceptance suite are rejected with an issue category and location.
- **SC-003**: Users can identify and correct expression errors in one edit cycle for at least 90% of invalid sample cases during acceptance testing.
- **SC-004**: Validation feedback for an individual expression up to 500 characters is returned in under 2 seconds in 95% of validation attempts.
