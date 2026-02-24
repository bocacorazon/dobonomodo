# Implementation Plan: Resolver Rule Evaluation Engine

**Branch**: `012-resolver-engine` | **Date**: 2026-02-22 | **Spec**: /workspace/specs/012-resolver-engine/spec.md
**Input**: Feature specification from `/specs/012-resolver-engine/spec.md`

**Note**: This template is filled in by the `/speckit.plan` command. See `.specify/templates/plan-template.md` for the execution workflow.

## Summary

Implement the Resolver rule evaluation engine that evaluates `when` conditions against resolution context, performs automatic period expansion using Calendar hierarchy, renders path/table/catalog templates, and returns a list of `ResolvedLocation`s. The engine will select the first matching rule from ordered resolver rules, expand requested periods to finer data levels using calendar hierarchies, render location templates with context tokens, and provide comprehensive diagnostics for troubleshooting.

## Technical Context

**Language/Version**: Rust 1.93.1 (edition 2021)  
**Primary Dependencies**: serde/serde_json (serialization), uuid (identifiers), chrono (date handling), polars (data processing context)  
**Storage**: PostgreSQL (entity metadata), object storage (trace files), file/database/catalog via DataSource adapters  
**Testing**: cargo test (unit tests in crates/core/tests/), integration tests (contract validation)  
**Target Platform**: Linux server (Kubernetes Jobs for engine-worker, API server deployment)  
**Project Type**: Cargo workspace monorepo (shared core library + multiple binaries)  
**Performance Goals**: <1 second for 95% of valid resolution requests, deterministic output ordering  
**Constraints**: Must integrate with existing Calendar hierarchy, support all three strategy types (Path/Table/Catalog), require no breaking changes to existing model structs  
**Scale/Scope**: Core library feature serving API server, engine-worker, and CLI; expected to resolve 100s-1000s of locations per pipeline run

## Constitution Check

*GATE: Must pass before Phase 0 research. Re-check after Phase 1 design.*

**Constitutional Principles to Verify**:

- [x] **Principle I (TDD)**: All implementation tasks paired with tests written FIRST - tests directory structure exists, contract tests pattern established, TDD workflow enforced. **Phase 1 validation**: Contract tests defined in `/contracts/resolver-engine-api.md` with 6 behavior contracts and concrete test cases.
- [x] **Principle II (Quality Gates)**: Build, lint, and test infrastructure configured - cargo test, workspace Cargo.toml with clippy/fmt configured. **Phase 1 validation**: Existing infrastructure sufficient; no new tooling required.
- [x] **Principle III (Completion Bias)**: Ambiguities resolved; no open decision blocks - existing models provide structure, calendar hierarchy exists, template rendering approach DECIDED (custom regex-based). **Phase 1 validation**: All technical decisions documented in `research.md` with rationale. Zero blocking unknowns.
- [x] **Principle IV (Comprehensive Testing)**: Test suite execution plan covers all test types - unit tests (rule matching, period expansion), integration tests (full resolution flow), contract tests (output validation). **Phase 1 validation**: Test structure defined: `resolver_us1_first_match.rs`, `resolver_us2_period_expansion.rs`, `resolver_us3_diagnostics.rs`, `contracts/resolver_engine_contract.rs`.

**Notes**: 
- **Pre-Phase 0**: Gate passed with one clarification (template rendering library choice).
- **Post-Phase 1**: All clarifications resolved autonomously. No principle violations. No complexity added. Data model extensions are backward-compatible. All functional requirements mapped to test contracts.
- **Gate Status**: ✅ PASS - Ready for Phase 2 (Task Generation) and implementation.

## Project Structure

### Documentation (this feature)

```text
specs/012-resolver-engine/
├── plan.md              # This file (/speckit.plan command output)
├── research.md          # Phase 0 output (/speckit.plan command)
├── data-model.md        # Phase 1 output (/speckit.plan command)
├── quickstart.md        # Phase 1 output (/speckit.plan command)
├── contracts/           # Phase 1 output (/speckit.plan command)
└── tasks.md             # Phase 2 output (/speckit.tasks command - NOT created by /speckit.plan)
```

### Source Code (repository root)

```text
crates/
├── core/
│   ├── src/
│   │   ├── lib.rs
│   │   ├── model/
│   │   │   ├── calendar.rs          # Calendar, LevelDef, DateRule (existing)
│   │   │   ├── period.rs             # Period (existing)
│   │   │   ├── resolver.rs           # Resolver, ResolutionRule, ResolvedLocation (existing)
│   │   │   └── expression.rs         # Expression (existing - may extend)
│   │   ├── resolver/
│   │   │   ├── mod.rs                # Module stub (existing)
│   │   │   ├── engine.rs             # NEW: Resolution engine entry point
│   │   │   ├── matcher.rs            # NEW: Rule condition evaluation
│   │   │   ├── expander.rs           # NEW: Period expansion logic
│   │   │   ├── renderer.rs           # NEW: Template rendering
│   │   │   ├── context.rs            # NEW: Resolution context types
│   │   │   └── diagnostics.rs        # NEW: Diagnostic generation
│   │   ├── engine/                   # Existing pipeline executor
│   │   ├── dsl/                      # Existing DSL parser
│   │   ├── trace/                    # Existing trace
│   │   └── validation/               # Existing validation
│   └── tests/
│       ├── resolver_us1_first_match.rs          # NEW: User Story 1 tests
│       ├── resolver_us2_period_expansion.rs     # NEW: User Story 2 tests
│       ├── resolver_us3_diagnostics.rs          # NEW: User Story 3 tests
│       ├── contracts/
│       │   └── resolver_engine_contract.rs      # NEW: Contract tests
│       └── fixtures/
│           └── resolvers/                       # NEW: Test resolver YAML files
│
├── api-server/                       # Existing - will call resolver engine
├── engine-worker/                    # Existing - will call resolver engine
├── cli/                              # Existing - will call resolver engine
└── test-resolver/                    # Existing test utilities
```

**Structure Decision**: Single Rust workspace project. This is a core library feature that adds a new `resolver` module implementation under `crates/core/src/resolver/`. The existing models (Resolver, ResolutionRule, Calendar, Period) are already defined in `crates/core/src/model/`. The implementation will add engine logic to evaluate rules, expand periods using calendar hierarchy, and render templates. Tests follow existing pattern: user story tests in `tests/resolver_us*.rs` and contract tests in `tests/contracts/`.

## Complexity Tracking

> **Fill ONLY if Constitution Check has violations that must be justified**

| Violation | Why Needed | Simpler Alternative Rejected Because |
|-----------|------------|-------------------------------------|
| N/A | No violations | No complexity added |

---

## Phase Completion Status

### ✅ Phase 0: Outline & Research (COMPLETE)

**Deliverable**: `research.md`

**Key Decisions**:
1. Template rendering: Custom regex-based substitution (zero deps, simple, fast)
2. Expression evaluation: Custom recursive descent parser (domain-specific, type-safe)
3. Period expansion: Graph traversal using Calendar hierarchy (correct semantics)
4. Resolver precedence: Three-level fallback (project → dataset → system default)
5. Diagnostics: Structured output with full evaluation trace
6. Integration: Pure function in core library, called by engine-worker

**Status**: All technical clarifications resolved. Zero blocking unknowns.

---

### ✅ Phase 1: Design & Contracts (COMPLETE)

**Deliverables**: `data-model.md`, `contracts/resolver-engine-api.md`, `quickstart.md`, agent context updated

**Artifacts Created**:
- **Data Model**: 13 entities documented (7 existing reference, 6 new internal types)
- **API Contract**: 6 behavior contracts with concrete test cases
- **Quickstart Guide**: 7-step tutorial with 3 common use cases
- **Agent Context**: Copilot instructions updated with Rust/serde/postgres context

**Key Design Choices**:
- Extended `ResolvedLocation` with traceability fields (backward-compatible)
- New internal types in `crates/core/src/resolver/` (context, diagnostics, engine)
- Test file structure: `resolver_us*.rs` for user stories, `contracts/` for API contracts

**Constitution Re-Check**: ✅ PASS (all 4 principles verified post-design)

---

### ⏸️ Phase 2: Task Breakdown (NEXT - NOT IN SCOPE)

**Note**: Per workflow specification, this command stops after Phase 1. Task generation is performed by the `/speckit.tasks` command.

**Next Steps for Implementation**:
1. Run `/speckit.tasks` to generate `tasks.md` with ordered implementation tasks
2. Follow TDD cycle for each task (write test → implement → refactor)
3. Run full test suite before each commit (`cargo test --all`)

---
