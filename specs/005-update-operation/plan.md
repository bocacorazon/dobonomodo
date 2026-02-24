# Implementation Plan: Update Operation

**Branch**: `005-update-operation` | **Date**: 2025-02-22 | **Spec**: [docs/specs/S04-update-operation/prompt.md](../../docs/specs/S04-update-operation/prompt.md)
**Input**: Feature specification from `docs/specs/S04-update-operation/prompt.md`

**Note**: This template is filled in by the `/speckit.plan` command. See `.specify/templates/plan-template.md` for the execution workflow.

## Summary

Implement the `update` operation type for the DobONoMoDo computation engine. The update operation applies selector-based row filtering, compiles assignment expressions, executes them against a working Polars LazyFrame, and updates system columns (`_updated_at`). This enables data transformation within the computation pipeline by modifying column values on matching rows based on expressions.

## Technical Context

**Language/Version**: Rust 1.75+ (Rust 2021 edition)  
**Primary Dependencies**: Polars 0.46 (lazy API), serde, serde_json, serde_yaml, anyhow, thiserror  
**Storage**: N/A (operates on in-memory LazyFrame)  
**Testing**: cargo test (unit tests with inline DataFrames + test harness scenarios)  
**Target Platform**: Linux server (Kubernetes Jobs for production, local for development)
**Project Type**: Single Cargo workspace (monorepo with multiple crates)  
**Performance Goals**: Sub-millisecond expression compilation (<1ms for <10 assignments); execution latency dominated by dataset size and expression complexity  
**Constraints**: No hardcoded memory limits in operation code; rely on Polars lazy/streaming execution and deployment-level resource limits  
**Scale/Scope**: Module within `dobo-core` crate; implements one of five operation types (update, aggregate, append, delete, output)

## Constitution Check

*GATE: Must pass before Phase 0 research. Re-check after Phase 1 design.*

**Constitutional Principles to Verify**:

- [x] **Principle I (TDD)**: All implementation tasks paired with tests written FIRST
  - ✓ Test harness (S02) already exists for scenario execution
  - ✓ Unit tests will be written before implementation code
  - ✓ Red-Green-Refactor cycle enforced
  
- [x] **Principle II (Quality Gates)**: Build, lint, and test infrastructure configured
  - ✓ Cargo workspace already configured with linting (clippy implied)
  - ✓ `cargo test` infrastructure in place
  - ✓ Tests in `/crates/core/tests/` directory structure established
  
- [x] **Principle III (Completion Bias)**: Ambiguities resolved; no open decision blocks
  - ✓ Performance goals resolved: sub-millisecond expression compilation target
  - ✓ Memory constraints resolved: rely on Polars streaming + external limits
  - ✓ Error handling strategy resolved: Result<T, anyhow::Error> pattern
  - ✓ All technical unknowns documented in research.md
  
- [x] **Principle IV (Comprehensive Testing)**: Test suite execution plan covers all test types
  - ✓ Unit tests planned (inline DataFrame scenarios)
  - ✓ Integration tests via test harness scenarios (TS-03, TS-08)
  - ✓ Contract tests for Operation deserialization already exist

**Notes**: 
- Performance and memory decisions are documented in `research.md` and adopted in this plan.
- Error handling strategy is standardized as `Result<T, anyhow::Error>` with contextual propagation.
- No constitutional violations; all Technical Context clarifications are resolved in `research.md`.

---

### Post-Design Re-Evaluation (After Phase 1)

**Date**: 2025-02-22  
**Status**: ✅ All principles satisfied

- [x] **Principle I (TDD)**: Quickstart guide demonstrates TDD workflow with Red-Green-Refactor cycle
  - Test scaffolding defined in quickstart.md
  - Unit tests specified before implementation
  - Integration tests via test harness scenarios (TS-03, TS-08)
  
- [x] **Principle II (Quality Gates)**: Data model and contracts specify validation requirements
  - Compile-time validation rules defined (VR-001 to VR-005)
  - Runtime validation via Polars error propagation (VR-006 to VR-008)
  - Quality gate checklist in quickstart: cargo test, clippy, fmt, build
  
- [x] **Principle III (Completion Bias)**: All design decisions made autonomously
  - Expression compilation strategy: Polars Expr API (researched)
  - Selector interpolation: compile-time string substitution (decided)
  - System column update: always set `_updated_at` to run timestamp (decided)
  - Error handling: Result<T, anyhow::Error> pattern (decided)
  - No blocking questions remain
  
- [x] **Principle IV (Comprehensive Testing)**: Test coverage specified in contracts
  - Unit tests: selector resolution, assignment execution, system columns, row filtering, error cases
  - Integration tests: TS-03 (FX conversion), TS-08 (named selector)
  - Contract tests: deserialization (already exists)

**Conclusion**: Design artifacts complete. Ready for Phase 2 (task generation via `/speckit.tasks`).

## Project Structure

### Documentation (this feature)

```text
specs/005-update-operation/
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
│   │   ├── engine/
│   │   │   ├── mod.rs
│   │   │   ├── types.rs
│   │   │   ├── io_traits.rs
│   │   │   └── ops/           # NEW: operation implementations
│   │   │       ├── mod.rs     # NEW: exports all operation modules
│   │   │       └── update.rs  # NEW: update operation implementation
│   │   ├── model/
│   │   │   ├── operation.rs   # EXISTING: Operation types and structures
│   │   │   └── expression.rs  # EXISTING: Expression definition
│   │   └── dsl/               # EXISTING: DSL parser (S01 dependency)
│   └── tests/
│       ├── unit/              # NEW: unit tests with inline DataFrames
│       │   └── update_operation_test.rs
│       └── integration/       # EXISTING: test harness scenarios (S02 dependency)
│           └── scenarios/
│               ├── ts03_fx_conversion.yaml      # Update without joins
│               └── ts08_named_selector.yaml     # Named selector interpolation
├── api-server/
├── engine-worker/
├── cli/
└── test-resolver/
```

**Structure Decision**: 
This is a Cargo workspace monorepo. The update operation implementation goes in `crates/core/src/engine/ops/update.rs` as a new module. The `engine/ops/` directory will be created to house operation implementations, following the principle of organizing by feature (operation type). Unit tests go in `crates/core/tests/unit/`, and integration tests leverage the existing test harness in S02 with scenario YAML files.

## Complexity Tracking

> **Fill ONLY if Constitution Check has violations that must be justified**

No constitutional violations. This section intentionally left empty.

---

## Planning Phase Summary

**Branch**: `005-update-operation`  
**Status**: Phase 0 & Phase 1 Complete (Ready for Phase 2)  
**Date Completed**: 2025-02-22

### Artifacts Generated

1. ✅ **plan.md** (this file) - Implementation plan with technical context
2. ✅ **research.md** - Resolved all NEEDS CLARIFICATION items with decisions and rationale
3. ✅ **data-model.md** - Entity definitions, validation rules, state transitions
4. ✅ **contracts/rust-api.md** - Public API contract with function signatures and behavior
5. ✅ **quickstart.md** - TDD workflow and implementation guide

### Key Design Decisions

| Decision | Choice | Rationale |
|----------|--------|-----------|
| Expression Compilation | Polars Expr API | Battle-tested, optimized, lazy evaluation |
| Selector Interpolation | Compile-time string substitution | Simple, early error detection |
| Error Handling | Result<T, anyhow::Error> | Aligns with existing codebase |
| System Column Update | Always set `_updated_at` to run timestamp | Consistency and auditability |
| Performance Target | Sub-millisecond compilation for <10 assignments | Polars optimizes execution |
| Memory Constraints | No explicit limits (rely on Polars streaming) | Deployment-specific configuration |

### Dependencies Confirmed

- **S01** (DSL Parser): Expression compilation from DSL strings to Polars Expr
- **S02** (Test Harness): Integration test execution via YAML scenarios
- **S03** (Period Filter): Not a blocker; update operates on any LazyFrame

### Next Steps

1. Run `/speckit.tasks` to generate ordered task list (tasks.md)
2. Execute tasks via `/speckit.implement` following TDD workflow
3. Ensure all tests pass before moving to S05 (Runtime Join)

### Constitutional Compliance

✅ All four constitutional principles verified (pre-design and post-design)  
✅ No violations or exceptions required  
✅ Ready for implementation
