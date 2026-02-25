# Copilot Instructions for `dobonomodo`

## Build, test, and lint commands

This repository is currently spec/docs-first; there is no root `Cargo.toml` (workspace scaffold is defined in `docs/specs/S00-workspace-scaffold/prompt.md`).

- Planned workspace build (after S00 scaffold is generated): `cargo build`
- Planned full test run: `cargo test`
- Planned single test run (Rust): `cargo test <test_name>`
- Planned scenario test run (CLI): `dobo test <scenario.yaml>`
- Planned scenario suite run: `dobo test --suite tests/scenarios`
- Planned lint command (from agent-context generator logic): `cargo clippy`

Spec-Kit workflow checks that are available now:
- Validate feature prerequisites: `.specify/scripts/bash/check-prerequisites.sh --json`
- Validate implementation prerequisites (requires tasks): `.specify/scripts/bash/check-prerequisites.sh --json --require-tasks --include-tasks`
- Resolve current feature paths only: `.specify/scripts/bash/check-prerequisites.sh --json --paths-only`

## High-level architecture

DobONoMoDo is designed as a Rust Cargo workspace monorepo with one shared core library and multiple binaries (`docs/architecture/system-architecture.md`):

- `core`: domain model + DSL parser/compiler + pipeline executor + resolver engine + trace + validation (no IO implementations)
- `api-server`: REST CRUD + run dispatch + run status tracking
- `engine-worker`: one run per process/job; resolves inputs, executes pipeline, writes outputs + traces
- `cli`: local/dev entry points (`dobo test/run/validate/parse`)
- `test-resolver`: in-memory/file-backed resolver used by the test harness

Execution model: API/CLI triggers a Run, Resolver resolves physical locations from logical Dataset definitions, worker executes ordered operations over Polars frames, output operations write results, and trace events are persisted.

## Key repository conventions

- Source-of-truth process is Spec-Kit: `/speckit.constitution -> /speckit.specify -> /speckit.plan -> /speckit.tasks -> /speckit.implement` (`.specify/memory/project-context.md`).
- Architecture and implementation decomposition live in `docs/architecture/`; feature work is spec-driven under `docs/specs/S##-<name>/`.
- Operations are strictly sequential; only `output` operations are allowed to perform IO (`docs/entities/project.md`, `docs/entities/operation.md`).
- Deleted rows are soft-deleted via `_deleted = true` and excluded by default from downstream operations and outputs.
- Resolver precedence is fixed: Project override -> Dataset resolver -> system default (`docs/entities/resolver.md`).
- Test scenarios are YAML-driven, conventionally discovered at `tests/scenarios/**/*.yaml`, and executed via the built-in test resolver (`docs/capabilities/execute-test-scenario.md`).
- Constitution rules are strict and should be treated as mandatory constraints in implementation sessions (`.specify/memory/constitution.md`), especially TDD + full quality gate adherence.

## Headless agent mode

When running inside a devcontainer as a headless agent (indicated by `AGENT_MODE=headless` env var):

- Complete all tasks without asking for user input or confirmation.
- If you encounter ambiguity, make the best decision consistent with the spec and constitution, then document the decision in the commit message.
- If you encounter a CRITICAL blocker that cannot be resolved (e.g., conflicting spec requirements, missing dependencies that don't exist), write a description to `.agent-escalation` and exit.
- Always run quality gates (build, test, clippy, fmt) before considering work complete.
- Follow TDD strictly: write failing tests first, then implement, then refactor.
- Commit frequently with descriptive messages including the spec ID.
