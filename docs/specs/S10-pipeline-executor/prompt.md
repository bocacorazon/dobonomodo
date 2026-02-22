# S10: Pipeline Executor

## Feature
Implement the sequential pipeline executor that takes a `ProjectSnapshot` and a set of period-filtered `LazyFrame`s, dispatches each operation in `seq` order to the correct operation implementation, manages the working `LazyFrame` state, and produces the final output.

## Context
- Read: `docs/entities/operation.md` (operation types, seq ordering, pipeline semantics)
- Read: `docs/entities/project.md` (OperationInstance, selectors map)
- Read: `docs/capabilities/execute-project-calculation.md` (execution flow, error handling, partial output)
- Read: `docs/architecture/system-architecture.md` (data flow section)

## Scope

### In Scope
- `core::engine::pipeline` module
- Accept `ProjectSnapshot` + loaded input `LazyFrame` + `Period` + IO trait implementations
- Iterate operations in `seq` order
- Dispatch each operation to its implementation (update, aggregate, append, delete, output)
- Pass working `LazyFrame` between operations
- Selector interpolation (expand `{{NAME}}` from `project.selectors` before compiling)
- Track `last_completed_operation` for resumability
- On failure: capture `ErrorDetail` (operation_order, message, detail) and return partial state
- Integration test: multi-operation pipeline using sample data (e.g., TS-03 + TS-05 combined)

### Out of Scope
- Run lifecycle management (S15)
- Trace event generation during execution (S12 — added as a hook after this spec)
- Resolver invocation (handled before pipeline starts)

## Dependencies
- **S04–S09** (all operation implementations)

## Parallel Opportunities
None — this is the integration point. However, **S11** (Resolver) can complete independently.

## Success Criteria
- Multi-operation pipeline executes in correct order
- Working `LazyFrame` state flows between operations
- Failure at operation N preserves partial output from operations 1..N-1
- `last_completed_operation` is correctly tracked
- All 5 operation types dispatch correctly
