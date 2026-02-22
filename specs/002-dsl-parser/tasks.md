# Tasks: DSL Parser & Expression Compiler

**Input**: Design documents from `/workspace/specs/002-dsl-parser/`
**Prerequisites**: plan.md (tech stack, libraries, structure), spec.md (success criteria), research.md (parser choice), data-model.md (entities), contracts/ (API specs)

**Tests**: Per constitutional principle I (TDD), all tasks MUST include tests written BEFORE implementation. Tests are MANDATORY and NON-NEGOTIABLE.

**Organization**: Tasks are grouped by user story to enable independent implementation and testing of each story.

## Format: `- [ ] [ID] [P?] [Story] Description`

- **[P]**: Can run in parallel (different files, no dependencies)
- **[Story]**: Which user story this task belongs to (e.g., US1, US2, US3)
- Include exact file paths in descriptions

## Path Conventions

- Rust workspace structure: `/workspace/crates/core/src/dsl/`
- Tests: `/workspace/crates/core/tests/`
- Grammar: `/workspace/crates/core/src/dsl/grammar.pest`

---

## Phase 1: Setup (Shared Infrastructure)

**Purpose**: Project initialization and basic structure

- [X] T001 Add pest and pest_derive dependencies to /workspace/crates/core/Cargo.toml
- [X] T002 [P] Add thiserror dependency to /workspace/crates/core/Cargo.toml
- [X] T003 [P] Add chrono dependency to /workspace/crates/core/Cargo.toml
- [X] T004 Create dsl module directory at /workspace/crates/core/src/dsl/
- [X] T005 Create dsl module entry point /workspace/crates/core/src/dsl/mod.rs with public exports
- [X] T006 [P] Register dsl module in /workspace/crates/core/src/lib.rs

---

## Phase 2: Foundational (Blocking Prerequisites)

**Purpose**: Core infrastructure that MUST be complete before ANY user story can be implemented

**⚠️ CRITICAL**: No user story work can begin until this phase is complete

- [X] T007 Create error types module /workspace/crates/core/src/dsl/error.rs with ParseError and ValidationError enums
- [X] T008 [P] Create AST definition module /workspace/crates/core/src/dsl/ast.rs with ExprAST, LiteralValue, BinaryOperator, UnaryOperator enums
- [X] T009 [P] Create type system module /workspace/crates/core/src/dsl/types.rs with ExprType enum
- [X] T010 Create CompilationContext struct in /workspace/crates/core/src/dsl/context.rs
- [X] T011 [P] Create pest grammar file /workspace/crates/core/src/dsl/grammar.pest with basic structure (SOI/EOI rules only)

**Checkpoint**: Foundation ready - user story implementation can now begin in parallel

---

## Phase 3: User Story 1 - Parse Expression Strings into AST (Priority: P1) 🎯 MVP

**Goal**: Parse expression strings into validated AST with clear parse errors and position info

**Independent Test**: Given a valid expression string like `"transactions.amount * 1.1"`, the parser produces an ExprAST tree. Invalid syntax like `"amount +"` produces ParseError with line/column info.

### Tests for User Story 1 (MANDATORY - TDD Principle I) ⚠️

> **NOTE: Write these tests FIRST, ensure they FAIL before implementation**

- [X] T012 [P] [US1] Unit test for literal parsing in /workspace/crates/core/tests/dsl_parser_tests.rs (numbers, strings, booleans, dates, NULL)
- [X] T013 [P] [US1] Unit test for column reference parsing in /workspace/crates/core/tests/dsl_parser_tests.rs (table.column syntax)
- [X] T014 [P] [US1] Unit test for binary operator parsing in /workspace/crates/core/tests/dsl_parser_tests.rs (arithmetic, comparison, logical)
- [X] T015 [P] [US1] Unit test for unary operator parsing in /workspace/crates/core/tests/dsl_parser_tests.rs (NOT, negation)
- [X] T016 [P] [US1] Unit test for function call parsing in /workspace/crates/core/tests/dsl_parser_tests.rs (zero args, multiple args, nested)
- [X] T017 [P] [US1] Unit test for operator precedence in /workspace/crates/core/tests/dsl_parser_tests.rs (arithmetic < comparison < logical)
- [X] T018 [P] [US1] Unit test for parentheses grouping in /workspace/crates/core/tests/dsl_parser_tests.rs (nested grouping)
- [X] T019 [P] [US1] Unit test for parse error cases in /workspace/crates/core/tests/dsl_parser_tests.rs (unclosed string, unclosed paren, invalid tokens)
- [X] T020 [P] [US1] Unit test for position tracking in /workspace/crates/core/tests/dsl_parser_tests.rs (line/column accuracy)
- [X] T021 [P] [US1] Integration test for sample expressions in /workspace/crates/core/tests/dsl_integration_tests.rs (all sample expressions from spec)

### Implementation for User Story 1

- [X] T022 [US1] Complete pest grammar in /workspace/crates/core/src/dsl/grammar.pest (literals, operators, functions, precedence)
- [X] T023 [US1] Implement parser module /workspace/crates/core/src/dsl/parser.rs with parse_expression and parse_expression_with_span functions
- [X] T024 [US1] Implement PrattParser configuration in /workspace/crates/core/src/dsl/parser.rs for operator precedence
- [X] T025 [US1] Implement AST builder functions in /workspace/crates/core/src/dsl/parser.rs (convert pest Pairs to ExprAST)
- [X] T026 [US1] Implement error conversion from pest Error to ParseError in /workspace/crates/core/src/dsl/parser.rs
- [X] T027 [US1] Add Span struct and parse_expression_with_span implementation in /workspace/crates/core/src/dsl/parser.rs
- [X] T028 [US1] Export parser API from /workspace/crates/core/src/dsl/mod.rs

**Checkpoint**: At this point, User Story 1 should be fully functional - expressions parse into AST with clear errors

---

## Phase 4: User Story 2 - Validate AST (Priority: P2)

**Goal**: Validate AST with column resolution, type checking, selector interpolation, and aggregate context rules

**Independent Test**: Given an AST with column reference `transactions.amount`, validation succeeds if schema contains that column and fails with UnresolvedColumnRef otherwise. Type mismatches like `amount + "text"` produce TypeMismatch error. Aggregate functions outside aggregate context produce InvalidAggregateContext error.

### Tests for User Story 2 (MANDATORY - TDD Principle I) ⚠️

- [ ] T029 [P] [US2] Unit test for column resolution in /workspace/crates/core/tests/dsl_validation_tests.rs (valid columns pass, invalid fail)
- [ ] T030 [P] [US2] Unit test for type inference in /workspace/crates/core/tests/dsl_validation_tests.rs (literals, columns, operators, functions)
- [ ] T031 [P] [US2] Unit test for type checking in /workspace/crates/core/tests/dsl_validation_tests.rs (arithmetic requires Number, logical requires Boolean)
- [ ] T032 [P] [US2] Unit test for aggregate context validation in /workspace/crates/core/tests/dsl_validation_tests.rs (SUM/COUNT/AVG only when allow_aggregates=true)
- [ ] T033 [P] [US2] Unit test for selector interpolation in /workspace/crates/core/tests/dsl_validation_tests.rs (simple, nested, circular detection)
- [ ] T034 [P] [US2] Unit test for selector edge cases in /workspace/crates/core/tests/dsl_validation_tests.rs (unresolved selector, max depth)
- [ ] T035 [P] [US2] Integration test for end-to-end validation in /workspace/crates/core/tests/dsl_integration_tests.rs (parse + validate pipeline)

### Implementation for User Story 2

- [ ] T036 [P] [US2] Create TypedExprAST struct in /workspace/crates/core/src/dsl/types.rs with ast and return_type fields
- [ ] T037 [P] [US2] Implement validation module /workspace/crates/core/src/dsl/validation.rs with validate_expression function
- [ ] T038 [US2] Implement resolve_column function in /workspace/crates/core/src/dsl/validation.rs (lookup table.column in schema)
- [ ] T039 [US2] Implement infer_type function in /workspace/crates/core/src/dsl/validation.rs (bottom-up type inference)
- [ ] T040 [US2] Implement type checking rules in /workspace/crates/core/src/dsl/validation.rs (binary ops, function args)
- [ ] T041 [US2] Implement aggregate context validation in /workspace/crates/core/src/dsl/validation.rs (check allow_aggregates flag)
- [ ] T042 [P] [US2] Implement selector interpolation module /workspace/crates/core/src/dsl/interpolation.rs with interpolate_selectors function
- [ ] T043 [US2] Implement circular reference detection in /workspace/crates/core/src/dsl/interpolation.rs (expansion stack tracking)
- [ ] T044 [US2] Export validation API from /workspace/crates/core/src/dsl/mod.rs

**Checkpoint**: At this point, User Stories 1 AND 2 should both work - parse then validate expressions

---

## Phase 5: User Story 3 - Compile to Polars Expr (Priority: P3)

**Goal**: Compile validated expressions into Polars Expr and support full function mapping

**Independent Test**: Given a validated AST for `SUM(transactions.amount)`, compilation produces a Polars Expr `col("transactions.amount").sum()` that can be attached to a LazyFrame. All 50+ DSL functions map correctly to Polars equivalents.

### Tests for User Story 3 (MANDATORY - TDD Principle I) ⚠️

- [ ] T045 [P] [US3] Unit test for literal compilation in /workspace/crates/core/tests/dsl_compiler_tests.rs (numbers, strings, booleans, NULL)
- [ ] T046 [P] [US3] Unit test for column reference compilation in /workspace/crates/core/tests/dsl_compiler_tests.rs (col("table.column"))
- [ ] T047 [P] [US3] Unit test for arithmetic operator compilation in /workspace/crates/core/tests/dsl_compiler_tests.rs (+, -, *, /)
- [ ] T048 [P] [US3] Unit test for comparison operator compilation in /workspace/crates/core/tests/dsl_compiler_tests.rs (=, <>, <, <=, >, >=)
- [ ] T049 [P] [US3] Unit test for logical operator compilation in /workspace/crates/core/tests/dsl_compiler_tests.rs (AND, OR, NOT)
- [ ] T050 [P] [US3] Unit test for arithmetic functions in /workspace/crates/core/tests/dsl_compiler_tests.rs (ABS, ROUND, FLOOR, CEIL, MOD, MIN, MAX)
- [ ] T051 [P] [US3] Unit test for string functions in /workspace/crates/core/tests/dsl_compiler_tests.rs (CONCAT, UPPER, LOWER, TRIM, LEFT, RIGHT, LEN, CONTAINS, REPLACE)
- [ ] T052 [P] [US3] Unit test for conditional functions in /workspace/crates/core/tests/dsl_compiler_tests.rs (IF, ISNULL, COALESCE)
- [ ] T053 [P] [US3] Unit test for date functions in /workspace/crates/core/tests/dsl_compiler_tests.rs (DATE, TODAY, YEAR, MONTH, DAY, DATEDIFF, DATEADD)
- [ ] T054 [P] [US3] Unit test for aggregate functions in /workspace/crates/core/tests/dsl_compiler_tests.rs (SUM, COUNT, COUNT_ALL, AVG, MIN_AGG, MAX_AGG)
- [ ] T055 [P] [US3] Contract test for Polars compatibility in /workspace/crates/core/tests/dsl_compiler_tests.rs (attach compiled Expr to dummy LazyFrame)
- [ ] T056 [P] [US3] Integration test for end-to-end compilation in /workspace/crates/core/tests/dsl_integration_tests.rs (interpolate + parse + validate + compile)

### Implementation for User Story 3

- [ ] T057 [P] [US3] Create CompiledExpression struct in /workspace/crates/core/src/dsl/compiler.rs with source, expr, return_type fields
- [ ] T058 [P] [US3] Create CompilationError enum in /workspace/crates/core/src/dsl/error.rs (UnsupportedFunction, PolarsCompatibility)
- [ ] T059 [US3] Implement compiler module /workspace/crates/core/src/dsl/compiler.rs with compile_expression function
- [ ] T060 [US3] Implement literal compilation in /workspace/crates/core/src/dsl/compiler.rs (lit(value))
- [ ] T061 [US3] Implement column reference compilation in /workspace/crates/core/src/dsl/compiler.rs (col("table.column"))
- [ ] T062 [US3] Implement binary operator compilation in /workspace/crates/core/src/dsl/compiler.rs (add, sub, mul, div, eq, etc.)
- [ ] T063 [US3] Implement unary operator compilation in /workspace/crates/core/src/dsl/compiler.rs (not, negate)
- [ ] T064 [US3] Implement arithmetic function mappings in /workspace/crates/core/src/dsl/compiler.rs (ABS, ROUND, FLOOR, CEIL, MOD, MIN, MAX)
- [ ] T065 [US3] Implement string function mappings in /workspace/crates/core/src/dsl/compiler.rs (CONCAT, UPPER, LOWER, TRIM, LEFT, RIGHT, LEN, CONTAINS, REPLACE)
- [ ] T066 [US3] Implement conditional function mappings in /workspace/crates/core/src/dsl/compiler.rs (IF → when/then/otherwise, ISNULL, COALESCE)
- [ ] T067 [US3] Implement date function mappings in /workspace/crates/core/src/dsl/compiler.rs (DATE, TODAY, YEAR, MONTH, DAY, DATEDIFF, DATEADD)
- [ ] T068 [US3] Implement aggregate function mappings in /workspace/crates/core/src/dsl/compiler.rs (SUM, COUNT, AVG, MIN_AGG, MAX_AGG)
- [ ] T069 [US3] Implement compile_with_interpolation function in /workspace/crates/core/src/dsl/compiler.rs (full pipeline)
- [ ] T070 [US3] Export compiler API from /workspace/crates/core/src/dsl/mod.rs

**Checkpoint**: All user stories should now be independently functional - complete parse/validate/compile pipeline

---

## Phase 6: Polish & Cross-Cutting Concerns

**Purpose**: Improvements that affect multiple user stories

- [ ] T071 [P] Add quickstart examples to /workspace/specs/002-dsl-parser/quickstart.md validation section
- [ ] T072 [P] Add comprehensive documentation comments to /workspace/crates/core/src/dsl/mod.rs
- [ ] T073 [P] Add performance benchmarks in /workspace/crates/core/benches/dsl_benchmarks.rs (parse 1000 expressions)
- [ ] T074 Code cleanup and clippy fixes across /workspace/crates/core/src/dsl/
- [ ] T075 Run cargo test from /workspace/Cargo.toml to validate all integration points
- [ ] T076 Run cargo clippy --all-targets from /workspace/Cargo.toml to validate code quality
- [ ] T077 [P] Update /workspace/.github/agents/copilot-instructions.md with dsl module context
- [ ] T078 Validate all sample expressions from /workspace/specs/002-dsl-parser/spec.md compile successfully

---

## Dependencies & Execution Order

### Phase Dependencies

- **Setup (Phase 1)**: No dependencies - can start immediately
- **Foundational (Phase 2)**: Depends on Setup completion - BLOCKS all user stories
- **User Stories (Phase 3, 4, 5)**: All depend on Foundational phase completion
  - US1 (Parse) can start after Foundational (Phase 2)
  - US2 (Validate) depends on US1 (needs parser)
  - US3 (Compile) depends on US2 (needs validated AST)
- **Polish (Phase 6)**: Depends on all user stories being complete

### User Story Dependencies

- **User Story 1 (P1 - Parse)**: Can start after Foundational (Phase 2) - No dependencies on other stories
- **User Story 2 (P2 - Validate)**: Depends on US1 (requires parse_expression function) - Can share test infrastructure
- **User Story 3 (P3 - Compile)**: Depends on US2 (requires TypedExprAST) - Can reuse validation context

### Within Each User Story

- Tests MUST be written and FAIL before implementation
- Core types before implementation
- Helper functions before main API
- Integration tests after all unit tests pass
- Story complete before moving to next priority

### Parallel Opportunities

- All Setup tasks marked [P] can run in parallel (T002, T003, T006)
- All Foundational tasks marked [P] can run in parallel within Phase 2 (T008, T009, T011)
- All tests for a user story marked [P] can run in parallel (T012-T021 for US1)
- Different modules within a story marked [P] can run in parallel
- User stories have sequential dependencies (US1 → US2 → US3) but implementation within each can be parallel

---

## Parallel Example: User Story 1

```bash
# Launch all unit tests for User Story 1 together (MANDATORY per TDD principle):
Task T012: "Unit test for literal parsing in /workspace/crates/core/tests/dsl_parser_tests.rs"
Task T013: "Unit test for column reference parsing in /workspace/crates/core/tests/dsl_parser_tests.rs"
Task T014: "Unit test for binary operator parsing in /workspace/crates/core/tests/dsl_parser_tests.rs"
Task T015: "Unit test for unary operator parsing in /workspace/crates/core/tests/dsl_parser_tests.rs"
Task T016: "Unit test for function call parsing in /workspace/crates/core/tests/dsl_parser_tests.rs"
Task T017: "Unit test for operator precedence in /workspace/crates/core/tests/dsl_parser_tests.rs"
Task T018: "Unit test for parentheses grouping in /workspace/crates/core/tests/dsl_parser_tests.rs"
Task T019: "Unit test for parse error cases in /workspace/crates/core/tests/dsl_parser_tests.rs"
Task T020: "Unit test for position tracking in /workspace/crates/core/tests/dsl_parser_tests.rs"
Task T021: "Integration test for sample expressions in /workspace/crates/core/tests/dsl_integration_tests.rs"

# After tests are written and failing, implement parser components
```

---

## Parallel Example: User Story 2

```bash
# Launch all validation tests together:
Task T029: "Unit test for column resolution in /workspace/crates/core/tests/dsl_validation_tests.rs"
Task T030: "Unit test for type inference in /workspace/crates/core/tests/dsl_validation_tests.rs"
Task T031: "Unit test for type checking in /workspace/crates/core/tests/dsl_validation_tests.rs"
Task T032: "Unit test for aggregate context validation in /workspace/crates/core/tests/dsl_validation_tests.rs"
Task T033: "Unit test for selector interpolation in /workspace/crates/core/tests/dsl_validation_tests.rs"
Task T034: "Unit test for selector edge cases in /workspace/crates/core/tests/dsl_validation_tests.rs"
Task T035: "Integration test for end-to-end validation in /workspace/crates/core/tests/dsl_integration_tests.rs"

# Launch parallel implementation after tests fail:
Task T036: "Create TypedExprAST struct in /workspace/crates/core/src/dsl/types.rs"
Task T037: "Implement validation module in /workspace/crates/core/src/dsl/validation.rs"
Task T042: "Implement selector interpolation module in /workspace/crates/core/src/dsl/interpolation.rs"
```

---

## Parallel Example: User Story 3

```bash
# Launch all compiler tests together:
Task T045: "Unit test for literal compilation"
Task T046: "Unit test for column reference compilation"
Task T047: "Unit test for arithmetic operator compilation"
Task T048: "Unit test for comparison operator compilation"
Task T049: "Unit test for logical operator compilation"
Task T050: "Unit test for arithmetic functions"
Task T051: "Unit test for string functions"
Task T052: "Unit test for conditional functions"
Task T053: "Unit test for date functions"
Task T054: "Unit test for aggregate functions"
Task T055: "Contract test for Polars compatibility"
Task T056: "Integration test for end-to-end compilation"

# Launch parallel implementation after tests fail:
Task T057: "Create CompiledExpression struct in /workspace/crates/core/src/dsl/compiler.rs"
Task T058: "Create CompilationError enum in /workspace/crates/core/src/dsl/error.rs"
```

---

## Implementation Strategy

### MVP First (User Story 1 Only)

1. Complete Phase 1: Setup
2. Complete Phase 2: Foundational (CRITICAL - blocks all stories)
3. Complete Phase 3: User Story 1 (Parse expressions into AST)
4. **STOP and VALIDATE**: Test parsing independently with all sample expressions
5. Deploy/demo parser if ready (can parse and report errors clearly)

**MVP Checkpoint**: At this point, you have a working expression parser that can:
- Parse all DSL expressions into structured AST
- Report parse errors with line/column position
- Handle all operators, functions, and precedence rules
- Serve as foundation for validation and compilation

### Incremental Delivery

1. Complete Setup + Foundational → Foundation ready
2. Add User Story 1 (Parse) → Test independently → Functional parser with error reporting
3. Add User Story 2 (Validate) → Test independently → Parser + semantic validation
4. Add User Story 3 (Compile) → Test independently → Complete parse/validate/compile pipeline
5. Each story adds value without breaking previous stories

### Parallel Team Strategy

With multiple developers:

1. Team completes Setup + Foundational together
2. Once Foundational is done:
   - Developer A: User Story 1 (Parse) - Tests T012-T021, Implementation T022-T028
   - Developer B: Prepare User Story 2 tests (T029-T035) - blocked on US1 completion
   - Developer C: Research Polars API for User Story 3 mappings
3. Sequential integration: US1 → US2 → US3 (dependencies require this order)
4. Within each story: Tests in parallel, then implementation modules in parallel

---

## Notes

- [P] tasks = different files, no dependencies - can execute in parallel
- [US#] label maps task to specific user story for traceability
- Each user story should be independently completable and testable
- Verify tests fail before implementing (red-green-refactor TDD cycle)
- Commit after each task or logical group
- Stop at any checkpoint to validate story independently
- US2 and US3 have sequential dependencies on previous stories (cannot parallelize across stories)
- Within each story, leverage [P] markers for parallel execution
- Constitutional Principle I enforced: ALL implementation has tests written FIRST
