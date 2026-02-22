# Feature Specification: Workspace Scaffold Baseline

**Feature Branch**: `001-workspace-scaffold`  
**Created**: 2026-02-22  
**Status**: Draft  
**Input**: User description: "using docs/specs/S00-workspace-scaffold"

## Clarifications

### Session 2026-02-22

- Q: What is the required scaffold verification gate? → A: Both `cargo build` and `cargo test` must pass.

## User Scenarios & Testing *(mandatory)*

### User Story 1 - Initialize Project Skeleton (Priority: P1)

As a platform developer, I need a complete repository skeleton with all required top-level modules so the team can start implementing features without blocking on project setup.

**Why this priority**: This is the foundation for all later features. Without it, no downstream work can proceed consistently.

**Independent Test**: Can be fully tested by creating a fresh checkout, validating that all required modules are present, and confirming both `cargo build` and `cargo test` complete successfully.

**Acceptance Scenarios**:

1. **Given** a fresh repository checkout, **When** the developer inspects the project structure, **Then** all required top-level modules and shared library areas are present in the expected layout.
2. **Given** the baseline scaffold is present, **When** the developer runs `cargo build` and `cargo test`, **Then** both commands complete successfully with zero errors.

---

### User Story 2 - Use Shared Domain Contracts (Priority: P2)

As an implementation developer, I need shared domain entities and lifecycle enums defined centrally so all components use consistent data contracts.

**Why this priority**: Shared contracts prevent schema drift and reduce integration failures between modules.

**Independent Test**: Can be tested by referencing each shared entity and enum from at least one module boundary and validating they load from structured configuration samples.

**Acceptance Scenarios**:

1. **Given** the shared core library, **When** a developer references domain entities and lifecycle enums from another module, **Then** all required types are available and consistently named.
2. **Given** a valid structured configuration sample for each entity, **When** it is loaded, **Then** all required entity fields and enum values are accepted.

---

### User Story 3 - Integrate IO Through Contracts (Priority: P3)

As a platform architect, I need IO capabilities represented as interface contracts so execution logic can remain independent from concrete storage and transport choices.

**Why this priority**: Contract-first IO boundaries keep core execution logic portable and make later adapters replaceable.

**Independent Test**: Can be tested by confirming each required IO contract exists with callable operation signatures and that no concrete external integration behavior is required in this feature.

**Acceptance Scenarios**:

1. **Given** the core execution domain, **When** a developer inspects available IO contracts, **Then** contracts exist for loading inputs, writing outputs, persisting metadata, and recording traces.
2. **Given** this baseline feature scope, **When** developers run project verification, **Then** the presence of IO contracts is sufficient without requiring concrete IO adapter behavior.

### Edge Cases

- If a required module folder or entry file is missing, baseline verification must fail with a clear error before any downstream feature work starts.
- If a shared entity definition omits required fields or lifecycle values, loading that entity from structured configuration must fail deterministically.
- If an IO contract is declared inconsistently with the defined boundary, module integration must fail during baseline verification.
- If no project tests exist yet, the standard test command must still complete successfully with a passing outcome.

## Requirements *(mandatory)*

### Functional Requirements

- **FR-001**: The system MUST provide a workspace root that includes all required component modules: shared core domain, API service, worker process, command-line entry point, and test resolver.
- **FR-002**: The shared core module MUST include dedicated areas for domain model, DSL handling, pipeline execution, resolver behavior, trace handling, and validation.
- **FR-003**: The system MUST define the complete baseline set of shared domain entities for datasets, table references, columns, lookups, projects, operations, runs, snapshots, resolvers, strategies, expressions, calendars, periods, and data sources.
- **FR-004**: The system MUST define lifecycle and behavior enums covering temporal mode, column type, run status, project status, operation kind, strategy type, and trigger type.
- **FR-005**: The system MUST expose shared domain entities and enums from the core module so other modules can consume them through a stable contract.
- **FR-006**: The system MUST define IO interface contracts for data loading, output writing, metadata persistence, and trace persistence.
- **FR-007**: The system MUST include executable entry points for each runtime-facing module so both `cargo build` and `cargo test` can evaluate all modules together.
- **FR-008**: The system MUST support loading all shared domain entities from YAML-formatted configuration inputs.
- **FR-009**: The system MUST support loading all shared domain entities from JSON-formatted configuration inputs.
- **FR-010**: The baseline feature MUST require no concrete business execution behavior beyond compile-safe placeholders for unimplemented logic.

### Key Entities *(include if feature involves data)*

- **Dataset & Related Definitions**: Logical data shape definitions, including table references, columns, lookup metadata, temporal behavior, and source linkage.
- **Project & OperationInstance**: Project-level configuration and ordered operation declarations that describe how a run should be executed.
- **Run**: A single execution instance with lifecycle status and trigger context.
- **ProjectSnapshot & ResolverSnapshot**: Captured state used to ensure repeatable execution and traceability.
- **Resolver, ResolutionRule, ResolutionStrategy**: Logical-to-physical resolution contract and precedence behavior.
- **Expression**: Reusable expression container representing operation logic input.
- **Calendar & Period**: Time modeling entities supporting period-aware processing.
- **DataSource**: External input/output source definition used by resolution and execution boundaries.
- **IO Contracts**: Interface entities for loading input data, writing output data, storing metadata, and storing trace events.

## Assumptions

- This feature is the first implementation baseline and has no prerequisite feature dependencies.
- Placeholder behavior is acceptable where execution logic is intentionally deferred to later features.
- Validation of entity loading from YAML/JSON is performed using representative valid samples for each required entity type.
- Additional runtime integrations (database, web server framework, orchestration, CLI behavior) are explicitly deferred to subsequent features.

## Success Criteria *(mandatory)*

### Measurable Outcomes

- **SC-001**: 100% of required top-level modules and shared core sub-areas are present and discoverable in a fresh checkout.
- **SC-002**: `cargo build` completes with zero errors on first run in a clean environment.
- **SC-003**: `cargo test` completes successfully on first run in the baseline scaffold state.
- **SC-004**: 100% of required shared entities and enums are loadable from valid YAML samples.
- **SC-005**: 100% of required shared entities and enums are loadable from valid JSON samples.
- **SC-006**: All four required IO contracts are available for consumption by other modules without requiring concrete adapter behavior.
