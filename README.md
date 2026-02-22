# DobONoMoDo

DobONoMoDo is a configuration-driven computation engine for running ordered data transformation pipelines over versioned Datasets using a domain-specific language (DSL).

## Current project status

This repository is currently **specification-first**: architecture, domain model, and implementation plans are defined in `docs/`, while the Rust workspace scaffold is planned but not yet generated in the root.

## Target architecture (planned)

- **Language/runtime**: Rust
- **Computation engine**: Polars (lazy API)
- **Metadata store**: PostgreSQL
- **Run orchestration**: Kubernetes Jobs (one Job per Run)
- **Interfaces**: REST API server + `dobo` CLI
- **Workspace layout**: Cargo monorepo with `core`, `api-server`, `engine-worker`, `cli`, `test-resolver`

See: `docs/architecture/system-architecture.md`.

## Repository structure

- `docs/entities/` — domain entity definitions (Dataset, Project, Operation, Run, Resolver, DataSource, etc.)
- `docs/capabilities/` — behavior and execution capabilities
- `docs/architecture/` — system architecture and implementation decomposition
- `docs/specs/S##-*/prompt.md` — spec prompts to drive implementation phases
- `.specify/` — Spec-Kit templates, memory, and workflow scripts
- `.github/agents/` and `.github/prompts/` — custom agent and prompt definitions

## Development workflow

The project uses Spec-Kit as the source-of-truth workflow:

`/speckit.constitution -> /speckit.specify -> /speckit.plan -> /speckit.tasks -> /speckit.implement`

Supporting context:
- `.specify/memory/project-context.md`
- `.specify/memory/constitution.md`

## Commands

### Available now (Spec-Kit workflow scripts)

```bash
.specify/scripts/bash/check-prerequisites.sh --json
.specify/scripts/bash/check-prerequisites.sh --json --require-tasks --include-tasks
.specify/scripts/bash/check-prerequisites.sh --json --paths-only
```

### Planned after workspace scaffold (S00+)

```bash
cargo build
cargo test
cargo test <test_name>
cargo clippy

dobo test <scenario.yaml>
dobo test --suite tests/scenarios
```

## Core execution model

1. A Project defines an ordered list of operations over an input Dataset.
2. A Run snapshots the Project and executes operations sequentially.
3. A Resolver maps logical tables + period context to physical data locations.
4. Only `output` operations perform IO; other operations transform in-memory working data.
5. Trace events capture before/after changes across operation execution.

## Additional docs

- High-level statement: `DobONoMoDo.md`
- Implementation plan inventory: `docs/architecture/implementation-plan.md`
- Sample domain data/test shape: `docs/architecture/sample-datasets.md`
