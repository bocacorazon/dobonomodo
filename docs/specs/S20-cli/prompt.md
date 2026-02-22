# S20: CLI Binary

## Feature
Build the `dobo` CLI with commands: `test` (run test scenarios), `run` (local execution), `validate` (activation validation), and `parse` (expression parsing utility).

## Context
- Read: `docs/architecture/system-architecture.md` (CLI responsibilities, command table)
- Read: `docs/capabilities/execute-test-scenario.md` (test harness flow)
- Read: `docs/capabilities/activate-project.md` (validation rules)

## Scope

### In Scope
- `cli/src/main.rs` with `clap` command definitions
- `dobo test <scenario.yaml>` — load scenario, run harness (S02), print `TestResult`
- `dobo test --suite <dir>` — discover `**/*.yaml`, run all, aggregate pass/fail counts
- `dobo run <project-file> --period <id>` — local execution using inline Project YAML + `InMemoryDataLoader` or filesystem
- `dobo validate <project-file>` — run activation validation (S13), print `ActivationError` or "Valid"
- `dobo parse <expression>` — parse expression, print AST or error (dev utility)
- Coloured terminal output for pass/fail/error
- Exit codes: 0 = all pass, 1 = failures, 2 = error

### Out of Scope
- API server interaction (no `dobo deploy`, `dobo activate` — those go through the API)
- Scheduler integration

## Dependencies
- **S02** (Test Harness), **S10** (Pipeline Executor), **S13** (Activation Validation)

## Parallel Opportunities
Can run in parallel with **S19** (Engine Worker).

## Success Criteria
- `dobo test` runs a scenario and prints pass/fail with diff report
- `dobo test --suite` discovers and runs all scenarios in a directory
- `dobo validate` reports all validation failures
- `dobo parse` parses and prints AST for valid expressions
- Exit codes are correct
