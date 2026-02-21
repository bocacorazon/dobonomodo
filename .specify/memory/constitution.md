<!--
SYNC IMPACT REPORT
==================
Version Change: [none] → 1.0.0 (initial ratification)
Modified Principles: N/A (initial creation)
Added Sections:
  - Core Principles (4 principles defined: TDD, Quality Gates, Completion Bias, Comprehensive Testing)
  - Quality Assurance (Code Quality Standards, Test Coverage Requirements)
  - Development Workflow (Feature Development Process, Commit Standards, Issue Resolution Protocol)
  - Governance (Constitutional Authority, Amendment Process, Version Control, Compliance Verification, Continuous Improvement)
Removed Sections: N/A
Templates Requiring Updates:
  ✅ plan-template.md - Constitution Check section updated with specific principle verification checklist
  ✅ spec-template.md - User scenarios & acceptance criteria already align with TDD principle
  ✅ tasks-template.md - Updated to enforce MANDATORY tests (was "optional"), aligned with TDD Principle I
  ✅ Agent files - Verified no agent-specific references (generic guidance maintained)
Follow-up TODOs: None
-->

# DobONoMoDo Constitution

## Core Principles

### I. Test-Driven Development (NON-NEGOTIABLE)

**MUST**: All code MUST be developed using strict Test-Driven Development (TDD).

- Tests MUST be written before implementation code
- Tests MUST fail initially (Red phase)
- Implementation MUST make tests pass (Green phase)
- Code MUST be refactored only after tests pass (Refactor phase)
- No production code may be committed without corresponding tests
- Red-Green-Refactor cycle is strictly enforced

**Rationale**: TDD ensures code correctness from the outset, provides living documentation, enables fearless refactoring, and catches regressions immediately. It is the foundation of code quality and maintainability.

### II. Strong Quality Gates (NON-NEGOTIABLE)

**MUST**: All code changes MUST pass comprehensive quality gates before merge.

- All test suites MUST pass (unit, integration, contract tests)
- Code coverage MUST NOT decrease from baseline
- Linting and formatting checks MUST pass without warnings
- Build MUST succeed on all target platforms
- All preexisting test failures MUST be fixed, not ignored
- Breaking tests discovered during feature work MUST be resolved before proceeding
- No "skip test" or "TODO: fix later" comments permitted without explicit justification

**Rationale**: Quality gates prevent technical debt accumulation. Fixing preexisting issues during feature work maintains codebase health and prevents the "broken windows" effect where one failure leads to acceptance of more failures.

### III. Completion Bias (NON-NEGOTIABLE)

**MUST**: Development agents MUST bias towards autonomous completion.

- Agents MUST make reasonable technical decisions independently
- Agents MUST ask clarifying questions ONLY when truly blocked
- "Truly blocked" means: ambiguous requirements, conflicting constraints, or missing credentials/access
- NOT blocked: technology choice, implementation approach, file naming, code structure
- Agents MUST propose solutions with rationale rather than ask for permission
- When multiple valid approaches exist, agents MUST choose one and document the decision

**Rationale**: Excessive back-and-forth interrupts flow, delays delivery, and wastes human bandwidth. Autonomous agents that make sound technical decisions and complete features enable faster iteration and higher throughput.

### IV. Comprehensive Test Execution (NON-NEGOTIABLE)

**MUST**: All test suites MUST be executed for every change.

- Run ALL test suites (unit, integration, contract, end-to-end) before committing
- Investigate and fix ALL test failures, including preexisting ones
- Test suite execution MUST NOT be skipped or abbreviated
- Test suite failures MUST be treated as blocking issues
- Newly discovered test failures MUST be fixed in the same work session
- Test infrastructure issues (flaky tests, broken setup) MUST be resolved immediately

**Rationale**: Partial test execution hides regressions. Preexisting failures mask new failures. Comprehensive test execution ensures system integrity and prevents degradation over time.

## Quality Assurance

### Code Quality Standards

- **Static Analysis**: Code MUST pass all configured linters and static analyzers
- **Code Review**: All changes require review against constitutional principles
- **Documentation**: Public APIs MUST have complete documentation
- **Error Handling**: All error paths MUST be tested and logged appropriately

### Test Coverage Requirements

- **Minimum Coverage**: 80% line coverage for new code (exception: exploratory prototypes)
- **Critical Paths**: 100% coverage for authentication, authorization, data integrity, security
- **Edge Cases**: Tests MUST cover boundary conditions, error scenarios, and failure modes

## Development Workflow

### Feature Development Process

1. **Specification**: Define user scenarios and acceptance criteria (spec.md)
2. **Planning**: Technical design and architecture decisions (plan.md)
3. **Task Breakdown**: Ordered, actionable tasks (tasks.md)
4. **Implementation**: TDD cycle for each task
   - Write failing tests
   - Implement minimal code to pass tests
   - Refactor while maintaining green tests
   - Run full test suite
   - Fix any preexisting failures discovered
   - Commit only when all tests pass
5. **Validation**: Verify against acceptance criteria

### Commit Standards

- **Atomicity**: Each commit MUST represent a single logical change
- **Test Status**: All tests MUST pass before committing
- **Messages**: Follow conventional commits format (type: description)
- **Verification**: Pre-commit hooks MUST verify test passage and linting

### Issue Resolution Protocol

When discovering preexisting issues during feature work:

1. **Document**: Note the issue and its scope
2. **Assess**: Determine if it blocks current feature
3. **Fix**: Resolve the issue in the same work session
4. **Test**: Verify fix with appropriate tests
5. **Continue**: Resume feature work only after issue resolution

## Governance

### Constitutional Authority

- This constitution supersedes all other development practices and conventions
- All feature specifications, plans, and implementations MUST comply with these principles
- Violations MUST be explicitly justified with documented rationale
- Unjustified violations constitute grounds for change rejection

### Amendment Process

- **Proposal**: Amendments MUST include clear rationale and impact analysis
- **Review**: Proposed changes reviewed against project goals
- **Approval**: Amendments require explicit approval
- **Migration**: Amendment adoption MUST include migration plan for existing code
- **Versioning**: Version incremented per semantic versioning rules (MAJOR.MINOR.PATCH)

### Version Control

- **MAJOR**: Backward-incompatible governance changes, principle removal/redefinition
- **MINOR**: New principle additions, materially expanded guidance
- **PATCH**: Clarifications, wording improvements, non-semantic refinements

### Compliance Verification

- All pull requests MUST verify constitutional compliance
- Automated checks MUST enforce quality gates
- Code reviews MUST reference specific constitutional principles
- Non-compliance MUST be addressed before merge

### Continuous Improvement

- Constitution MUST be reviewed quarterly for relevance
- Principles MUST evolve with project maturity
- Feedback loops MUST inform constitutional amendments
- Retrospectives MUST identify principle violations and root causes

**Version**: 1.0.0 | **Ratified**: 2026-02-21 | **Last Amended**: 2026-02-21
