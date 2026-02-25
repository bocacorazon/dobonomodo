# Tasks: Resolver Rule Evaluation Engine

**Input**: Design documents from `/workspace/specs/012-resolver-engine/`
**Prerequisites**: plan.md (required), spec.md (required for user stories), research.md, data-model.md, contracts/

**Tests**: Per constitutional principle I (TDD), all tasks MUST include tests written BEFORE implementation. Tests are MANDATORY and NON-NEGOTIABLE.

**Organization**: Tasks are grouped by user story to enable independent implementation and testing of each story.

## Format: `[ID] [P?] [Story] Description`

- **[P]**: Can run in parallel (different files, no dependencies)
- **[Story]**: Which user story this task belongs to (e.g., US1, US2, US3)
- Include exact file paths in descriptions

## Path Conventions

Single Rust workspace project:
- Core library: `crates/core/src/`
- Tests: `crates/core/tests/`
- Project root: `/workspace/`

---

## Phase 1: Setup (Shared Infrastructure)

**Purpose**: Project initialization and basic structure

- [X] T001 Create resolver module structure at crates/core/src/resolver/ with mod.rs, engine.rs, matcher.rs, expander.rs, renderer.rs, context.rs, diagnostics.rs
- [X] T002 Define ResolutionRequest and ResolutionContext types in crates/core/src/resolver/context.rs
- [X] T003 Define ResolutionResult, ResolutionDiagnostic, RuleDiagnostic, ResolverSource, and DiagnosticOutcome in crates/core/src/resolver/diagnostics.rs
- [X] T004 Extend ResolvedLocation struct with resolver_id and rule_name fields in crates/core/src/model/resolver.rs
- [X] T005 Create test fixtures directory structure at crates/core/tests/fixtures/resolvers/ for test YAML files
- [X] T006 [P] Create test fixture: sample calendar with year→quarter→month hierarchy in crates/core/tests/fixtures/calendars/fiscal_calendar.yaml
- [X] T007 [P] Create test fixture: sample periods for 2023-2024 in crates/core/tests/fixtures/periods/test_periods.yaml
- [X] T008 [P] Create test fixture: multi-rule resolver in crates/core/tests/fixtures/resolvers/sales_resolver.yaml

---

## Phase 2: Foundational (Blocking Prerequisites)

**Purpose**: Core infrastructure that MUST be complete before ANY user story can be implemented

**⚠️ CRITICAL**: No user story work can begin until this phase is complete

- [X] T009 Implement template token parser with regex-based validation in crates/core/src/resolver/renderer.rs (parse_template function)
- [X] T010 Implement template renderer with context substitution in crates/core/src/resolver/renderer.rs (render_template function)
- [X] T011 [P] Implement expression lexer for when conditions in crates/core/src/resolver/matcher.rs (tokenize function)
- [X] T012 [P] Implement expression parser (recursive descent) in crates/core/src/resolver/matcher.rs (parse_expression function)
- [X] T013 Implement expression evaluator for boolean conditions in crates/core/src/resolver/matcher.rs (evaluate_expression function)
- [X] T014 Create helper function to build ResolutionContext from ResolutionRequest + Period in crates/core/src/resolver/context.rs
- [X] T015 Create diagnostic builder utilities for rule evaluation tracing in crates/core/src/resolver/diagnostics.rs

**Checkpoint**: Foundation ready - user story implementation can now begin in parallel

---

## Phase 3: User Story 1 - Resolve by First Matching Rule (Priority: P1) 🎯 MVP

**Goal**: Evaluate resolver rules in order and select the first matching rule for each resolution request

**Independent Test**: Submit resolution requests with contexts that match different rules and verify that only the first matching rule is selected for each request

### Tests for User Story 1 (MANDATORY - TDD Principle I) ⚠️

> **NOTE: Write these tests FIRST, ensure they FAIL before implementation**

- [X] T016 [P] [US1] Contract test for first-match semantics in crates/core/tests/contracts/resolver_engine_contract.rs (test_first_match_semantics function)
- [X] T017 [P] [US1] Contract test for unconditional catch-all rule in crates/core/tests/contracts/resolver_engine_contract.rs (test_unconditional_rule_match function)
- [X] T018 [P] [US1] Integration test for ordered rule evaluation in crates/core/tests/resolver_us1_first_match.rs (test_ordered_rule_evaluation function)
- [X] T019 [P] [US1] Integration test for when condition evaluation with period comparison in crates/core/tests/resolver_us1_first_match.rs (test_period_condition_match function)
- [X] T020 [P] [US1] Integration test for when condition evaluation with table name match in crates/core/tests/resolver_us1_first_match.rs (test_table_condition_match function)
- [X] T021 [P] [US1] Integration test for multiple matching rules returning only first in crates/core/tests/resolver_us1_first_match.rs (test_first_match_wins function)
- [X] T022 [P] [US1] Integration test for catch-all rule when earlier rules don't match in crates/core/tests/resolver_us1_first_match.rs (test_catch_all_fallback function)

### Implementation for User Story 1

- [X] T023 [P] [US1] Implement rule condition matcher for single rule in crates/core/src/resolver/matcher.rs (evaluate_rule function)
- [X] T024 [US1] Implement first-match rule selection logic in crates/core/src/resolver/engine.rs (select_matching_rule function)
- [X] T025 [US1] Implement main resolve function (single period, no expansion) in crates/core/src/resolver/engine.rs (resolve function - minimal version)
- [X] T026 [US1] Add diagnostics generation for matched rule in crates/core/src/resolver/diagnostics.rs (record_rule_match function)
- [X] T027 [US1] Add diagnostics generation for non-matched rules in crates/core/src/resolver/diagnostics.rs (record_rule_no_match function)
- [X] T028 [US1] Implement error handling for NoMatchingRule case in crates/core/src/resolver/engine.rs
- [X] T029 [US1] Add validation for when_expression syntax in crates/core/src/resolver/matcher.rs (validate_expression function)

**Checkpoint**: At this point, User Story 1 should be fully functional and testable independently (rule matching works, no period expansion yet)

---

## Phase 4: User Story 2 - Expand Periods to Data Granularity (Priority: P2)

**Goal**: Expand requested periods to finer data levels using calendar hierarchy so one request resolves all required child periods

**Independent Test**: Resolve requests where requested period is coarser than configured data level and verify child periods and returned locations match hierarchy definition

### Tests for User Story 2 (MANDATORY - TDD Principle I) ⚠️

- [X] T030 [P] [US2] Contract test for quarter-to-month expansion in crates/core/tests/contracts/resolver_engine_contract.rs (test_period_expansion function)
- [X] T031 [P] [US2] Contract test for "any" data level (no expansion) in crates/core/tests/contracts/resolver_engine_contract.rs (test_no_expansion_for_any_level function)
- [X] T032 [P] [US2] Contract test for deterministic ordering in crates/core/tests/contracts/resolver_engine_contract.rs (test_deterministic_ordering function)
- [X] T033 [P] [US2] Integration test for year-to-month expansion (12 locations) in crates/core/tests/resolver_us2_period_expansion.rs (test_year_to_month_expansion function)
- [X] T034 [P] [US2] Integration test for quarter-to-month expansion (3 locations) in crates/core/tests/resolver_us2_period_expansion.rs (test_quarter_to_month_expansion function)
- [X] T035 [P] [US2] Integration test for same-level period (no expansion needed) in crates/core/tests/resolver_us2_period_expansion.rs (test_same_level_no_expansion function)
- [X] T036 [P] [US2] Integration test for data_level="any" returns single location in crates/core/tests/resolver_us2_period_expansion.rs (test_any_level_single_location function)
- [X] T037 [P] [US2] Integration test for invalid hierarchy path failure in crates/core/tests/resolver_us2_period_expansion.rs (test_invalid_hierarchy_expansion_fails function)
- [X] T038 [P] [US2] Integration test for expanded periods in diagnostic output in crates/core/tests/resolver_us2_period_expansion.rs (test_diagnostic_expanded_periods function)

### Implementation for User Story 2

- [X] T039 [P] [US2] Implement calendar level hierarchy traversal in crates/core/src/resolver/expander.rs (find_level_path function)
- [X] T040 [P] [US2] Implement period tree traversal to find children at target level in crates/core/src/resolver/expander.rs (find_child_periods function)
- [X] T041 [US2] Implement period expansion logic with hierarchy validation in crates/core/src/resolver/expander.rs (expand_period function)
- [X] T042 [US2] Add deterministic ordering by sequence field in crates/core/src/resolver/expander.rs (sort_periods_by_sequence function)
- [X] T043 [US2] Integrate period expansion into main resolve function in crates/core/src/resolver/engine.rs (update resolve function)
- [X] T044 [US2] Implement data_level="any" special case (skip expansion) in crates/core/src/resolver/expander.rs
- [X] T045 [US2] Implement same-level detection (requested level == data_level) in crates/core/src/resolver/expander.rs
- [X] T046 [US2] Add error handling for PeriodExpansionFailed in crates/core/src/resolver/engine.rs
- [X] T047 [US2] Update diagnostics to include expanded_periods list in crates/core/src/resolver/diagnostics.rs

**Checkpoint**: At this point, User Stories 1 AND 2 should both work independently (period expansion functional)

---

## Phase 5: User Story 3 - Explain No-Match Outcomes and Precedence (Priority: P3)

**Goal**: Provide clear diagnostics when no rules match and document resolver precedence for troubleshooting

**Independent Test**: Trigger no-match requests and precedence conflicts, then verify diagnostics and selected resolver source are explicit and correct

### Tests for User Story 3 (MANDATORY - TDD Principle I) ⚠️

- [X] T048 [P] [US3] Contract test for traceability metadata in crates/core/tests/contracts/resolver_engine_contract.rs (test_traceability function)
- [X] T049 [P] [US3] Integration test for no-match diagnostic with all rules listed in crates/core/tests/resolver_us3_diagnostics.rs (test_no_match_diagnostic_completeness function)
- [X] T050 [P] [US3] Integration test for no-match error includes evaluation reasons in crates/core/tests/resolver_us3_diagnostics.rs (test_no_match_reasons function)
- [X] T051 [P] [US3] Integration test for template render error diagnostic in crates/core/tests/resolver_us3_diagnostics.rs (test_template_error_diagnostic function)
- [X] T052 [P] [US3] Integration test for expression syntax error diagnostic in crates/core/tests/resolver_us3_diagnostics.rs (test_expression_error_diagnostic function)
- [X] T053 [P] [US3] Integration test for successful resolution includes all rules in diagnostic in crates/core/tests/resolver_us3_diagnostics.rs (test_success_diagnostic_all_rules function)
- [X] T054 [P] [US3] Integration test for resolver source tracking (DatasetReference) in crates/core/tests/resolver_us3_diagnostics.rs (test_resolver_source_dataset function)
- [X] T055 [P] [US3] Integration test for resolver_id and rule_name in all locations in crates/core/tests/resolver_us3_diagnostics.rs (test_location_traceability function)

### Implementation for User Story 3

- [X] T056 [P] [US3] Implement comprehensive no-match diagnostic generation in crates/core/src/resolver/diagnostics.rs (build_no_match_diagnostic function)
- [X] T057 [P] [US3] Implement detailed reason strings for each rule evaluation outcome in crates/core/src/resolver/diagnostics.rs (format_rule_reason function)
- [X] T058 [US3] Update rule matcher to capture expression evaluation details in crates/core/src/resolver/matcher.rs
- [X] T059 [US3] Add resolver source tracking to resolution context in crates/core/src/resolver/context.rs (add resolver_source field)
- [X] T060 [US3] Update resolve function to populate resolver_id and rule_name in ResolvedLocation in crates/core/src/resolver/engine.rs
- [X] T061 [US3] Implement template render error with token identification in crates/core/src/resolver/renderer.rs
- [X] T062 [US3] Add diagnostics for invalid expression syntax errors in crates/core/src/resolver/matcher.rs
- [X] T063 [US3] Update NoMatchingRule error to include full diagnostic in crates/core/src/resolver/engine.rs
- [X] T064 [US3] Add diagnostic outcome classification logic in crates/core/src/resolver/diagnostics.rs (determine_outcome function)

**Checkpoint**: All user stories should now be independently functional with comprehensive diagnostics

---

## Phase 6: Polish & Cross-Cutting Concerns

**Purpose**: Improvements that affect multiple user stories

- [X] T065 [P] Add unit tests for template renderer edge cases in crates/core/tests/unit/test_renderer.rs (URL encoding, empty tokens, special characters)
- [X] T066 [P] Add unit tests for expression parser edge cases in crates/core/tests/unit/test_matcher.rs (nested expressions, operator precedence, invalid syntax)
- [X] T067 [P] Add unit tests for period expansion edge cases in crates/core/tests/unit/test_expander.rs (empty children, cycle detection, missing parent)
- [X] T068 Add comprehensive documentation comments (rustdoc) for all public API functions in crates/core/src/resolver/
- [X] T069 Add module-level documentation with usage examples in crates/core/src/resolver/mod.rs
- [X] T070 [P] Verify all contract tests pass in crates/core/tests/contracts/resolver_engine_contract.rs
- [X] T071 [P] Verify quickstart examples work end-to-end using fixtures from crates/core/tests/fixtures/
- [X] T072 Run cargo clippy and fix all warnings in crates/core/src/resolver/
- [X] T073 Run cargo fmt to ensure consistent code style across all resolver module files in crates/core/src/resolver/
- [X] T074 Add performance benchmarks for 100-period expansion in crates/core/benches/resolver_bench.rs
- [X] T075 Validate all success criteria from spec.md against test suite results in specs/012-resolver-engine/spec.md

---

## Dependencies & Execution Order

### Phase Dependencies

- **Setup (Phase 1)**: No dependencies - can start immediately
- **Foundational (Phase 2)**: Depends on Setup completion - BLOCKS all user stories
- **User Stories (Phase 3-5)**: All depend on Foundational phase completion
  - User stories can then proceed in parallel (if staffed)
  - Or sequentially in priority order (P1 → P2 → P3)
- **Polish (Phase 6)**: Depends on all user stories being complete

### User Story Dependencies

- **User Story 1 (P1)**: Can start after Foundational (Phase 2) - No dependencies on other stories
  - Implements: Rule matching, condition evaluation, first-match selection, basic diagnostics
  - Outputs: Single ResolvedLocation for requested period (no expansion)
  
- **User Story 2 (P2)**: Can start after Foundational (Phase 2) - Extends US1 with period expansion
  - **Minimal dependency on US1**: Reuses rule matching logic from US1
  - Can be developed in parallel if interface is clear
  - Implements: Calendar hierarchy traversal, period tree navigation, expansion logic
  - Outputs: Multiple ResolvedLocations (one per expanded period)
  
- **User Story 3 (P3)**: Can start after Foundational (Phase 2) - Enhances US1 and US2 diagnostics
  - **Minimal dependency on US1 and US2**: Adds diagnostic richness to existing flows
  - Can be developed in parallel if diagnostic interfaces are defined
  - Implements: Comprehensive error messages, resolver source tracking, traceability metadata
  - Outputs: Enhanced ResolutionDiagnostic with full evaluation trace

### Within Each User Story

1. **Tests FIRST** (TDD Principle I): All tests for a story MUST be written and FAIL before implementation
2. **Foundational logic**: Core algorithms (matcher, expander, renderer)
3. **Integration**: Wire logic into main resolve() function
4. **Error handling**: Add specific error cases and diagnostics
5. **Story validation**: Run all tests for the story, verify independent functionality

### Parallel Opportunities

- **Setup (Phase 1)**:
  - T005-T008 (test fixtures) can all run in parallel
  
- **Foundational (Phase 2)**:
  - T009-T010 (template renderer) can run in parallel with T011-T013 (expression evaluator)
  
- **User Story 1 Tests**:
  - T016-T022 (all US1 tests) can be written in parallel once test framework is ready
  
- **User Story 1 Implementation**:
  - T023 (matcher) and T029 (validation) can run in parallel
  - T026 and T027 (diagnostics) can run in parallel
  
- **User Story 2 Tests**:
  - T030-T038 (all US2 tests) can be written in parallel
  
- **User Story 2 Implementation**:
  - T039-T040 (hierarchy and tree traversal) can run in parallel
  
- **User Story 3 Tests**:
  - T048-T055 (all US3 tests) can be written in parallel
  
- **User Story 3 Implementation**:
  - T056-T057 (diagnostic generation) can run in parallel with T061-T062 (error handling)
  
- **Polish (Phase 6)**:
  - T065-T067 (unit tests) can all run in parallel
  - T068-T069 (documentation) can run in parallel
  - T070-T071 (validation) can run in parallel
  - T072-T073 (linting/formatting) can run in parallel

---

## Parallel Example: User Story 1

```bash
# Step 1: Launch all tests for User Story 1 together (MANDATORY per TDD principle):
# These MUST be written first and MUST fail before implementation begins
Task T016: "Contract test for first-match semantics in crates/core/tests/contracts/resolver_engine_contract.rs"
Task T017: "Contract test for unconditional catch-all rule in crates/core/tests/contracts/resolver_engine_contract.rs"
Task T018: "Integration test for ordered rule evaluation in crates/core/tests/resolver_us1_first_match.rs"
Task T019: "Integration test for when condition evaluation with period comparison in crates/core/tests/resolver_us1_first_match.rs"
Task T020: "Integration test for when condition evaluation with table name match in crates/core/tests/resolver_us1_first_match.rs"
Task T021: "Integration test for multiple matching rules returning only first in crates/core/tests/resolver_us1_first_match.rs"
Task T022: "Integration test for catch-all rule when earlier rules don't match in crates/core/tests/resolver_us1_first_match.rs"

# Step 2: Verify all tests FAIL (red phase)
cargo test --package dobo-core --test resolver_us1_first_match
cargo test --package dobo-core --test contracts/resolver_engine_contract

# Step 3: Launch parallelizable implementation tasks together:
Task T023: "Implement rule condition matcher in crates/core/src/resolver/matcher.rs"
Task T029: "Add validation for when_expression syntax in crates/core/src/resolver/matcher.rs"
# And in parallel:
Task T026: "Add diagnostics generation for matched rule in crates/core/src/resolver/diagnostics.rs"
Task T027: "Add diagnostics generation for non-matched rules in crates/core/src/resolver/diagnostics.rs"

# Step 4: Complete sequential implementation tasks:
Task T024: "Implement first-match rule selection logic in crates/core/src/resolver/engine.rs"
Task T025: "Implement main resolve function in crates/core/src/resolver/engine.rs"
Task T028: "Implement error handling for NoMatchingRule case in crates/core/src/resolver/engine.rs"

# Step 5: Verify all tests PASS (green phase)
cargo test --package dobo-core --test resolver_us1_first_match
cargo test --package dobo-core --test contracts/resolver_engine_contract

# Step 6: Refactor if needed, run tests again
```

---

## Parallel Example: User Story 2

```bash
# Step 1: Launch all tests for User Story 2 together (MANDATORY per TDD principle):
Task T030: "Contract test for quarter-to-month expansion in crates/core/tests/contracts/resolver_engine_contract.rs"
Task T031: "Contract test for 'any' data level in crates/core/tests/contracts/resolver_engine_contract.rs"
Task T032: "Contract test for deterministic ordering in crates/core/tests/contracts/resolver_engine_contract.rs"
Task T033: "Integration test for year-to-month expansion in crates/core/tests/resolver_us2_period_expansion.rs"
Task T034: "Integration test for quarter-to-month expansion in crates/core/tests/resolver_us2_period_expansion.rs"
Task T035: "Integration test for same-level period in crates/core/tests/resolver_us2_period_expansion.rs"
Task T036: "Integration test for data_level='any' in crates/core/tests/resolver_us2_period_expansion.rs"
Task T037: "Integration test for invalid hierarchy path in crates/core/tests/resolver_us2_period_expansion.rs"
Task T038: "Integration test for expanded periods diagnostic in crates/core/tests/resolver_us2_period_expansion.rs"

# Step 2: Verify all tests FAIL (red phase)
cargo test --package dobo-core --test resolver_us2_period_expansion

# Step 3: Launch parallelizable implementation tasks:
Task T039: "Implement calendar level hierarchy traversal in crates/core/src/resolver/expander.rs"
Task T040: "Implement period tree traversal in crates/core/src/resolver/expander.rs"
# Continue with sequential tasks T041-T047...

# Step 4: Verify all tests PASS (green phase)
cargo test --package dobo-core --test resolver_us2_period_expansion
```

---

## Implementation Strategy

### MVP First (User Story 1 Only)

1. **Complete Phase 1: Setup** (T001-T008)
   - Create module structure
   - Define all types and enums
   - Create test fixtures
   - Extend existing models
   
2. **Complete Phase 2: Foundational** (T009-T015) - CRITICAL BLOCKER
   - Template rendering (simple token substitution)
   - Expression parsing and evaluation
   - Context building utilities
   - Diagnostic foundations
   
3. **Complete Phase 3: User Story 1** (T016-T029)
   - Write all US1 tests FIRST (T016-T022) - verify they FAIL
   - Implement rule matching logic (T023-T029)
   - Run tests - verify they PASS
   
4. **STOP and VALIDATE**: 
   - Test User Story 1 independently using test fixtures
   - Verify first-match semantics work correctly
   - Verify diagnostics show rule evaluation trace
   - No period expansion yet (returns single location)
   
5. **Demo capability**: Show rule-based resolution with condition evaluation

### Incremental Delivery

1. **Setup + Foundational** → Foundation ready
   - Module structure exists
   - Core utilities (parser, renderer) functional
   - Ready for user story implementation
   
2. **Add User Story 1** → Test independently → MVP!
   - Rule matching works
   - First-match selection correct
   - Basic diagnostics available
   - **Value**: Can resolve single-period locations based on conditions
   
3. **Add User Story 2** → Test independently → Enhanced MVP
   - Period expansion via hierarchy
   - Multiple locations per request
   - Deterministic ordering
   - **Value**: Can resolve multi-period datasets (quarters → months)
   
4. **Add User Story 3** → Test independently → Production-ready
   - Comprehensive diagnostics
   - Clear error messages
   - Full traceability
   - **Value**: Operators can troubleshoot resolution failures

### Parallel Team Strategy

With multiple developers:

1. **Team completes Setup + Foundational together** (Week 1)
   - Critical: Everyone understands core types and utilities
   - T001-T015 must be complete before splitting
   
2. **Once Foundational is done, split work by story**:
   - **Developer A**: User Story 1 (T016-T029)
     - Focus: Rule matching, condition evaluation
     - Timeline: 2-3 days
   - **Developer B**: User Story 2 (T030-T047)
     - Focus: Period expansion, hierarchy traversal
     - Timeline: 3-4 days (more complex)
   - **Developer C**: User Story 3 (T048-T064)
     - Focus: Diagnostics, error handling
     - Timeline: 2-3 days
     
3. **Stories complete and integrate independently**
   - Each story has its own test file
   - Integration point: main resolve() function in engine.rs
   - Minimal merge conflicts (different files)
   
4. **Team completes Polish together** (Phase 6)
   - Run full test suite
   - Address any integration issues
   - Performance validation

---

## Notes

- **[P] tasks** = different files, no dependencies, safe to parallelize
- **[Story] label** maps task to specific user story for traceability
- **Each user story should be independently completable and testable**
- **TDD workflow**: Write test → verify FAIL → implement → verify PASS → refactor
- **Commit strategy**: Commit after each task or logical group (e.g., all tests for a story)
- **Stop at any checkpoint** to validate story independently before proceeding
- **Avoid**: 
  - Vague tasks without file paths
  - Editing same file in parallel (causes conflicts)
  - Cross-story dependencies that break independence (US2 and US3 extend US1 but don't block each other)
  - Implementing before writing tests (violates TDD principle)

---

## Task Summary

**Total Tasks**: 75

**By Phase**:
- Phase 1 (Setup): 8 tasks
- Phase 2 (Foundational): 7 tasks (CRITICAL BLOCKER)
- Phase 3 (User Story 1): 14 tasks (7 tests + 7 implementation)
- Phase 4 (User Story 2): 18 tasks (9 tests + 9 implementation)
- Phase 5 (User Story 3): 17 tasks (8 tests + 9 implementation)
- Phase 6 (Polish): 11 tasks

**By User Story**:
- User Story 1 (First Match): 14 tasks (50% tests, 50% implementation)
- User Story 2 (Period Expansion): 18 tasks (50% tests, 50% implementation)
- User Story 3 (Diagnostics): 17 tasks (47% tests, 53% implementation)

**Parallel Opportunities Identified**:
- Setup: 4 tasks can run in parallel (fixtures)
- Foundational: 2 groups can run in parallel (renderer + evaluator)
- US1: 10 parallelizable tasks (7 tests + 3 implementation)
- US2: 11 parallelizable tasks (9 tests + 2 implementation)
- US3: 10 parallelizable tasks (8 tests + 2 implementation)
- Polish: 8 tasks can run in parallel

**MVP Scope** (User Story 1 only):
- 15 tasks (Setup + Foundational + US1)
- Estimated effort: 3-4 days for single developer
- Delivers: Rule-based resolution with condition evaluation (no period expansion)

**Full Feature Scope** (All 3 user stories):
- 64 tasks (Setup + Foundational + US1 + US2 + US3)
- Estimated effort: 7-10 days for single developer, 4-5 days with 3 developers
- Delivers: Complete resolver engine with period expansion and diagnostics

---

## Validation Checklist

✅ **Format compliance**: All tasks follow `- [ ] T### [P?] [US?] Description with file path` format
✅ **User story organization**: Tasks grouped by story (US1, US2, US3)
✅ **TDD enforcement**: Tests written BEFORE implementation for each story
✅ **Independent testability**: Each story has acceptance criteria and can be validated standalone
✅ **File path specificity**: Every task includes exact file path
✅ **Parallel markers**: [P] tags identify parallelizable tasks
✅ **Dependencies documented**: Phase and story dependencies clearly listed
✅ **MVP identified**: User Story 1 marked as MVP scope
✅ **Contract mapping**: All contracts from contracts/resolver-engine-api.md covered by tests
✅ **Data model coverage**: All new entities from data-model.md implemented
✅ **Functional requirements**: All FR-001 through FR-012 mapped to tasks
