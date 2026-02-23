# Implementation Plan: DSL Parser & Expression Compiler

**Branch**: `002-dsl-parser` | **Date**: 2026-02-22 | **Spec**: /workspace/docs/specs/S01-dsl-parser/prompt.md
**Input**: Feature specification from `/workspace/docs/specs/S01-dsl-parser/prompt.md`

**Note**: The setup script pointed to a missing spec.md at /workspace/specs/002-dsl-parser/spec.md. Instead, I detected the valid feature specification at /workspace/docs/specs/S01-dsl-parser/prompt.md and am proceeding with that as the authoritative source. This decision is documented here for transparency.

## Summary

Parse expression strings from the YAML DSL into an AST, perform type-checking and column resolution, interpolate `{{SELECTOR}}` references, and compile the AST into Polars `Expr` objects that can be attached to `LazyFrame` operations. This enables the DobONoMoDo computation engine to execute user-defined transformations using a domain-specific language with Excel-style function syntax.

## Technical Context

**Language/Version**: Rust 2021 edition (workspace baseline: 0.1.0)
**Primary Dependencies**: 
- Polars 0.46 (lazy API for Expr compilation target)
- Parser generator: pest (chosen in Phase 0 research; see research.md)
- serde/serde_json/serde_yaml (already in workspace for entity deserialization)
- thiserror (error type definition)
- chrono (date/time handling for TODAY() and date functions)

**Storage**: N/A (this is a pure compilation/parsing module)
**Testing**: cargo test (unit tests for parser, type checker, compiler; integration tests for end-to-end expression compilation)
**Target Platform**: Linux server (part of DobONoMoDo computation engine)
**Project Type**: Single Cargo workspace (monorepo structure under crates/)
**Performance Goals**: 
- Parse + compile 1000 expressions in <100ms (expression compilation is a pre-execution step, not runtime-critical)
- Zero-copy parsing where possible
**Constraints**: 
- Must integrate with existing dobo-core crate structure
- Must produce Polars `polars::lazy::dsl::Expr` objects compatible with Polars 0.46 lazy API
- Must not execute expressions (execution deferred to S03+)
- Expression compilation is synchronous and deterministic
**Scale/Scope**: 
- Support ~50 DSL functions across 5 categories
- Handle expressions up to ~1000 characters
- Typical Projects contain 10-100 expressions across all operations

## Constitution Check

*GATE: Must pass before Phase 0 research. Re-check after Phase 1 design.*

**Constitutional Principles to Verify**:

- [x] **Principle I (TDD)**: All implementation tasks paired with tests written FIRST ✅
  - Parser tests: invalid syntax, edge cases, position tracking (defined in parser-api.md contract)
  - Type checker tests: type mismatches, aggregate context violations (defined in validation-api.md contract)
  - Compiler tests: each DSL function → Polars Expr mapping (defined in compiler-api.md contract)
  - Integration tests: end-to-end expression compilation with sample expressions from spec
  - **Post-Phase 1**: All test requirements documented in API contracts with specific test cases
  
- [x] **Principle II (Quality Gates)**: Build, lint, and test infrastructure configured ✅
  - Cargo workspace already configured (S00 baseline)
  - cargo build, cargo test, cargo clippy all functional
  - Pre-commit hooks can be added if not already present
  - **Post-Phase 1**: No new infrastructure required; uses existing workspace setup
  
- [x] **Principle III (Completion Bias)**: Ambiguities resolved; no open decision blocks ✅
  - Parser generator decision resolved: **pest selected** (see research.md)
  - All design decisions are clear: Excel-style syntax, Polars compilation target, error types
  - AST structure defined in data-model.md
  - API contracts specify all public interfaces
  - **Post-Phase 1**: All technical decisions made; zero open questions remaining
  
- [x] **Principle IV (Comprehensive Testing)**: Test suite execution plan covers all test types ✅
  - Unit tests: parser grammar rules, AST construction, type inference, Polars mapping (per contracts)
  - Integration tests: sample expressions from spec, error cases, selector interpolation (per contracts)
  - Contract tests: ensure generated Polars Expr objects are valid (can attach to dummy LazyFrame)
  - **Post-Phase 1**: Comprehensive test plans documented in each API contract

**Notes** (Post-Phase 1 Re-evaluation):
- ✅ All principles verified successfully
- ✅ Parser generator decision resolved (pest chosen, rationale in research.md)
- ✅ Data model complete with all entities, state transitions, and validation rules
- ✅ API contracts define all public interfaces with complete test requirements
- ✅ Agent context updated (copilot-instructions.md)
- ✅ No constitutional violations or exceptions required
- **GATE PASSED**: Ready to proceed to Phase 2 (task generation via /speckit.tasks)

## Project Structure

### Documentation (this feature)

```text
specs/[###-feature]/
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
│   │   ├── dsl/                    # NEW: DSL parser & compiler module
│   │   │   ├── mod.rs              # Module exports
│   │   │   ├── parser.rs           # Expression string → AST
│   │   │   ├── ast.rs              # AST node definitions
│   │   │   ├── resolver.rs         # Column resolution & validation
│   │   │   ├── type_checker.rs     # Type inference & validation
│   │   │   ├── compiler.rs         # AST → Polars Expr
│   │   │   ├── interpolation.rs    # {{SELECTOR}} expansion
│   │   │   ├── error.rs            # Error types
│   │   │   └── grammar.pest or grammar.lalrpop  # Parser grammar (chosen in Phase 0)
│   │   ├── model/
│   │   │   ├── expression.rs       # EXISTING: Expression newtype
│   │   │   ├── dataset.rs          # EXISTING: DatasetSchema, ColumnDef, ColumnType
│   │   │   ├── operation.rs        # EXISTING: OperationInstance (for aggregate context)
│   │   │   └── project.rs          # EXISTING: Project (for selectors map)
│   │   └── lib.rs
│   └── tests/
│       ├── dsl_parser_tests.rs     # NEW: Parser unit tests
│       ├── dsl_compiler_tests.rs   # NEW: Compiler unit tests
│       └── dsl_integration_tests.rs # NEW: End-to-end tests with sample expressions
├── api-server/                     # No changes
├── engine-worker/                  # No changes
├── cli/                            # No changes
└── test-resolver/                  # No changes
```

**Structure Decision**: This feature adds a new `dsl/` submodule within `crates/core/src/` to house the parser, type checker, and Polars compiler. The existing `model/` submodule already contains the domain entities referenced by the parser (Expression, Dataset, Operation, Project). Tests are added under `crates/core/tests/` following the existing test structure from S00.

## Complexity Tracking

> **Fill ONLY if Constitution Check has violations that must be justified**

No violations detected. All constitutional principles verified successfully.

---

## Phase Completion Summary

### Phase 0: Research (COMPLETE) ✅

**Artifacts Generated**:
- `/workspace/specs/002-dsl-parser/research.md` - Comprehensive research on parser generator choice (pest vs lalrpop), best practices, Polars integration patterns, and selector interpolation strategy

**Key Decisions**:
- **Parser Generator**: pest (PEG-based) selected over lalrpop
  - Rationale: Simpler for expression grammars, PrattParser for precedence, existing project guidance
- **Selector Interpolation**: String substitution + re-parse approach
- **Error Handling**: thiserror for custom error types with position tracking
- **Type Checking**: Bottom-up inference with explicit validation rules

**Outcome**: All technical unknowns resolved; zero open questions remaining.

---

### Phase 1: Design & Contracts (COMPLETE) ✅

**Artifacts Generated**:
1. `/workspace/specs/002-dsl-parser/data-model.md` - Complete data model with:
   - Core entities: ExprAST, ExprType, ParseError, ValidationError, CompilationContext, CompiledExpression
   - Entity relationships and state transitions
   - Validation rules (BR-001 through BR-004)
   - Domain invariants

2. `/workspace/specs/002-dsl-parser/contracts/parser-api.md` - Parser API contract:
   - Public API: `parse_expression`, `parse_expression_with_span`
   - Grammar definition (pest PEG)
   - Precedence levels via PrattParser
   - Comprehensive test requirements

3. `/workspace/specs/002-dsl-parser/contracts/compiler-api.md` - Compiler API contract:
   - Public API: `compile_expression`, `compile_with_interpolation`
   - Complete DSL → Polars function mapping table (50+ functions)
   - Operator mappings
   - Comprehensive test requirements

4. `/workspace/specs/002-dsl-parser/contracts/validation-api.md` - Validation API contract:
   - Public API: `validate_expression`, `resolve_column`, `infer_type`, `interpolate_selectors`
   - Validation rules with implementation pseudocode
   - Type inference rules
   - Comprehensive test requirements

5. `/workspace/specs/002-dsl-parser/quickstart.md` - Developer quickstart guide:
   - Installation and setup
   - Basic usage examples (parse, validate, compile)
   - Expression syntax reference
   - Error handling patterns
   - Common patterns and troubleshooting

**Agent Context Updates**:
- ✅ Updated `/workspace/.github/agents/copilot-instructions.md` with Rust 2021 edition and project type information

**Outcome**: Complete design specification ready for implementation task generation.

---

### Phase 2: Task Generation (PENDING) ⏸️

**Next Command**: `/speckit.tasks` to generate dependency-ordered implementation tasks

**Expected Output**: `/workspace/specs/002-dsl-parser/tasks.md` with:
- Ordered tasks following TDD workflow (tests first, then implementation)
- Task dependencies ensuring proper build sequence
- Clear acceptance criteria for each task
- Coverage of all API contracts and data model entities

---

## Constitution Check Re-evaluation (Post-Phase 1)

All constitutional principles verified ✅:
- **Principle I (TDD)**: Test requirements documented in all API contracts
- **Principle II (Quality Gates)**: Build/lint/test infrastructure ready (existing workspace)
- **Principle III (Completion Bias)**: All decisions made; zero ambiguities
- **Principle IV (Comprehensive Testing)**: Complete test plans in contracts

**GATE PASSED**: Ready for task generation and implementation.

---

## References

- Feature Spec: `/workspace/docs/specs/S01-dsl-parser/prompt.md`
- Constitution: `/workspace/.specify/memory/constitution.md`
- Entity Documentation: `/workspace/docs/entities/expression.md`
- Workspace Baseline: S00 (001-workspace-scaffold)
