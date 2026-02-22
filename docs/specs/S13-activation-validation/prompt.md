# S13: Activation Validation

## Feature
Implement the activation validation rules (VAL-001 through VAL-009) that gate the `draft → active` Project transition. Validate expressions, column references, type compatibility, selector references, and Resolver availability. Return structured `ActivationError` on failure.

## Context
- Read: `docs/capabilities/activate-project.md` (validation rules VAL-001–009, ActivationError, ValidationFailure)
- Read: `docs/entities/project.md` (status transitions, BR-012/012a)
- Read: `docs/architecture/sample-datasets.md` (TS-11 activation validation failures)

## Scope

### In Scope
- `core::validation` module
- VAL-001: Expression syntax validation (all expressions in all operations parse)
- VAL-002: Column reference resolution (against Dataset schema + join aliases)
- VAL-003: Type compatibility (selector = boolean, assignment types match)
- VAL-004: `{{NAME}}` references resolve to `project.selectors` keys
- VAL-005: Named selectors parse as valid boolean expressions
- VAL-006: Resolver reachable for the Dataset (override → dataset → default)
- VAL-007: Resolved Resolver is active
- VAL-008: RuntimeJoin `dataset_id` references exist and are active
- VAL-009: Operation `order` values are unique
- `ActivationError` with `Vec<ValidationFailure>` — all failures collected, not fail-fast
- Test scenario TS-11: intentionally broken Project with multiple validation errors

### Out of Scope
- Actually transitioning Project status (S14)
- DataSource reachability validation (deferred per activate-project.md OQ-001)

## Dependencies
- **S01** (DSL Parser): expression parsing and type-checking
- **S11** (Resolver Engine): Resolver lookup for VAL-006/007

## Parallel Opportunities
Can run in parallel with **S15** (Run Lifecycle).

## Success Criteria
- All 9 validation rules are implemented and independently testable
- Multiple failures are collected and returned together
- Each `ValidationFailure` includes rule ID, type, operation_order, and detail
- Valid Projects produce empty failure list
