# System Architecture

**Status**: Draft  
**Created**: 2026-02-22  
**Stack**: Rust + Polars + PostgreSQL + Kubernetes

---

## Overview

DobONoMoDo is a configuration-driven computation engine that executes ordered operation pipelines over Datasets. The system is composed of three deployable artefacts — an **API Server**, **Engine Workers**, and a **CLI** — built from a shared core library in a Cargo workspace monorepo. Runs are dispatched as Kubernetes Jobs, giving elastic horizontal scaling at the Run level. Each Run is an isolated Polars process that reads input data via a Resolver, executes the operation pipeline, and writes output to the configured destination.

---

## Architecture Decisions

| Decision | Choice | Rationale |
|---|---|---|
| Computation engine | Polars (Rust-native) | Lazy DataFrame API maps directly to the sequential operation pipeline; expression compilation target; zero-overhead; single binary |
| Language | Rust | Compiled single binary; Polars is native Rust; Arrow ecosystem; memory safety |
| Metadata store | PostgreSQL | Entities (Datasets, Projects, Resolvers, Runs) stored relationally; mature, reliable |
| Run dispatch | Kubernetes Jobs | One Job per Run; elastic scaling; isolation; no custom queue infrastructure |
| Run output storage | Configurable per `output` operation | Destinations defined in the operation DSL; supports S3, filesystem, database via DataSource |
| Trace storage | Object storage | Trace events written as files alongside Run output; queryable by Run ID |
| Code organisation | Cargo workspace monorepo | Shared `core` crate; separate binaries for API server, engine worker, CLI |
| API interface | REST (API server) + CLI | CLI for dev/testing; API server for production orchestration |

---

## System Components

```
┌─────────────────────────────────────────────────────────────────┐
│                        Kubernetes Cluster                       │
│                                                                 │
│  ┌──────────────┐     K8s Job      ┌──────────────────────┐    │
│  │  API Server   │───creates───────▶│  Engine Worker (Run)  │    │
│  │  (Deployment) │                  │  (Job per Run)        │    │
│  │              │◄──status─────────│                      │    │
│  └──────┬───────┘                  └──────────┬───────────┘    │
│         │                                     │                 │
│         │                                     │ Polars pipeline │
│         │                                     │                 │
│  ┌──────▼───────┐              ┌──────────────▼──────────────┐ │
│  │  PostgreSQL   │              │  Data Layer                 │ │
│  │  (Metadata)   │              │  ┌────────┐ ┌────────────┐ │ │
│  │  - Datasets   │              │  │ S3/GCS │ │ Databases  │ │ │
│  │  - Projects   │              │  │ (files)│ │ (tables)   │ │ │
│  │  - Resolvers  │              │  └────────┘ └────────────┘ │ │
│  │  - Runs       │              │  ┌────────────────────────┐ │ │
│  │  - Calendars  │              │  │ Trace Events (files)   │ │ │
│  └──────────────┘              └─────────────────────────────┘ │
│                                                                 │
│  ┌──────────────┐                                               │
│  │  CLI          │  (dev/test — runs outside or inside cluster) │
│  │  (local bin)  │                                               │
│  └──────────────┘                                               │
└─────────────────────────────────────────────────────────────────┘
```

---

## Cargo Workspace Structure

```
dobonomodo/
├── Cargo.toml                    # workspace root
├── crates/
│   ├── core/                     # shared domain logic — no IO, no framework deps
│   │   ├── Cargo.toml
│   │   └── src/
│   │       ├── lib.rs
│   │       ├── model/            # entity structs: Dataset, Project, Run, Resolver, etc.
│   │       ├── dsl/              # DSL parser + expression compiler → Polars Expr
│   │       ├── engine/           # pipeline executor: loads data, runs operations, writes output
│   │       ├── resolver/         # Resolver rule evaluator + period expansion + strategy dispatch
│   │       ├── trace/            # trace event generation (diffs)
│   │       └── validation/       # activation validation rules (VAL-001..009)
│   │
│   ├── api-server/               # REST API — thin wrapper over core
│   │   ├── Cargo.toml            # deps: axum, sqlx, k8s-client
│   │   └── src/
│   │       ├── main.rs
│   │       ├── routes/           # CRUD for entities + Run dispatch
│   │       ├── k8s/              # K8s Job creation + status polling
│   │       └── db/               # PostgreSQL repositories (sqlx)
│   │
│   ├── engine-worker/            # the binary that K8s Jobs run
│   │   ├── Cargo.toml            # deps: core, polars, object-store, sqlx
│   │   └── src/
│   │       ├── main.rs           # receives RunSpec, executes pipeline, writes output + trace
│   │       └── io/               # DataSource adapters: S3, filesystem, database connectors
│   │
│   ├── cli/                      # developer CLI — test harness + local execution
│   │   ├── Cargo.toml            # deps: core, clap
│   │   └── src/
│   │       ├── main.rs
│   │       ├── commands/         # run, test, validate, activate
│   │       └── test_harness/     # loads YAML scenarios, runs engine, compares output
│   │
│   └── test-resolver/            # built-in test Resolver for harness
│       ├── Cargo.toml
│       └── src/
│           └── lib.rs            # serves inline/file-referenced test data
│
├── docs/                         # entity + capability + architecture docs
├── tests/
│   └── scenarios/                # convention: tests/scenarios/**/*.yaml
└── migrations/                   # PostgreSQL schema migrations (sqlx-migrate)
```

---

## Component Responsibilities

### `core` (library crate — the heart)

The core crate contains all domain logic with **no IO dependencies**. It is the single source of truth for:

| Module | Responsibility |
|---|---|
| `model/` | Rust structs for all entities (Dataset, Project, Run, Resolver, Operation, Expression, etc.). Serde-derived for YAML/JSON serialization |
| `dsl/` | DSL parser (pest or lalrpop) that compiles Expression strings into `polars::lazy::dsl::Expr`. Handles `{{SELECTOR}}` interpolation, column resolution, type-checking |
| `engine/` | Pipeline executor: takes a `ProjectSnapshot` + resolved `LazyFrame`s → executes operations in sequence → produces output `DataFrame`. Pure transformation — IO is injected via traits |
| `resolver/` | Evaluates Resolver rules: `when` condition matching, period expansion via Calendar hierarchy, template rendering. Returns `Vec<ResolvedLocation>` — does NOT load data (IO is the caller's job) |
| `trace/` | Generates trace events by diffing `DataFrame` state before/after each operation. Produces `TraceEvent` structs |
| `validation/` | Activation validation: expression syntax, type-checking, column resolution, selector refs, Resolver availability. Returns `Vec<ValidationFailure>` |

**Key design principle**: `core` defines IO traits (e.g., `DataLoader`, `OutputWriter`, `MetadataStore`) but never implements them. This makes it testable without any infrastructure.

### `api-server` (binary crate)

| Responsibility | Detail |
|---|---|
| Entity CRUD | REST endpoints for Datasets, Projects, Resolvers, Calendars, Periods |
| Run dispatch | Creates a K8s Job with the `RunSpec` (ProjectSnapshot + Period IDs + Resolver snapshots) serialized as a config |
| Run monitoring | Polls Job status; updates Run entity in PostgreSQL |
| Activation | Invokes `core::validation` and transitions Project status |
| Auth / multi-tenancy | Out of scope for initial version; placeholder middleware |

### `engine-worker` (binary crate)

| Responsibility | Detail |
|---|---|
| Receives `RunSpec` | From K8s Job environment (config map or argument) |
| Resolves data | Uses `core::resolver` to get locations, then loads via IO adapters (S3, filesystem, database) into Polars `LazyFrame`s |
| Executes pipeline | Calls `core::engine` with the loaded frames |
| Writes output | Serializes result `DataFrame` to configured destination via IO adapters |
| Writes trace | Serializes `TraceEvent`s to object storage as Parquet files alongside output |
| Reports status | Updates Run status in PostgreSQL (running → completed/failed) |

### `cli` (binary crate)

| Command | Description |
|---|---|
| `dobo test <scenario.yaml>` | Runs test harness: loads scenario, injects metadata, executes via `core::engine`, compares output |
| `dobo test --suite <dir>` | Discovers `**/*.yaml` in directory, runs all, reports aggregate results |
| `dobo run <project> --period <id>` | Local execution — same as engine-worker but in-process, no K8s |
| `dobo validate <project>` | Runs activation validation without changing status |
| `dobo parse <expression>` | Parses and type-checks an expression string — dev utility |

### `test-resolver` (library crate)

A Resolver implementation that serves data from in-memory `DataBlock`s (inline rows from YAML or loaded from referenced files). Automatically injected by the test harness. Implements the same `DataLoader` trait as production IO adapters.

---

## Data Flow: Run Execution

```
User/Scheduler
     │
     ▼
API Server ──────► PostgreSQL: Create Run (status: queued)
     │
     ▼
K8s API: Create Job (RunSpec in ConfigMap)
     │
     ▼
Engine Worker starts
     │
     ├─► PostgreSQL: Update Run (status: running)
     │
     ├─► core::resolver — evaluate rules, expand periods → Vec<ResolvedLocation>
     │
     ├─► IO adapters — load data from locations → LazyFrame per table
     │         │
     │         ├── S3 adapter (Parquet/CSV)
     │         ├── Filesystem adapter
     │         └── Database adapter (sqlx)
     │
     ├─► core::engine — execute operations sequentially
     │         │
     │         ├── Period filter (by temporal_mode)
     │         ├── For each Operation:
     │         │     ├── Resolve selector (interpolate {{NAME}})
     │         │     ├── Compile expressions → Polars Expr
     │         │     ├── Load RuntimeJoin Datasets (via Resolver + IO)
     │         │     ├── Apply operation (update/aggregate/append/delete/output)
     │         │     └── Generate trace events (diff before/after)
     │         │
     │         └── Return output DataFrame + trace events
     │
     ├─► IO adapters — write output to configured destination
     │
     ├─► Object storage — write trace events as Parquet
     │
     └─► PostgreSQL: Update Run (status: completed, output_dataset_id)
```

---

## IO Trait Boundary

The clean separation between `core` (pure logic) and IO is achieved through traits:

```rust
/// Loads data from a resolved location into a Polars LazyFrame.
trait DataLoader {
    fn load(&self, location: &ResolvedLocation, schema: &TableSchema) -> Result<LazyFrame>;
}

/// Writes a DataFrame to a destination.
trait OutputWriter {
    fn write(&self, frame: &DataFrame, destination: &OutputDestination) -> Result<()>;
}

/// Reads/writes entity metadata (Datasets, Projects, Runs, etc.).
trait MetadataStore {
    fn get_dataset(&self, id: &Uuid, version: Option<i32>) -> Result<Dataset>;
    fn get_project(&self, id: &Uuid) -> Result<Project>;
    fn get_resolver(&self, id: &str) -> Result<Resolver>;
    fn update_run_status(&self, id: &Uuid, status: RunStatus) -> Result<()>;
    // ...
}

/// Writes trace events.
trait TraceWriter {
    fn write_events(&self, run_id: &Uuid, events: &[TraceEvent]) -> Result<()>;
}
```

Production implementations: `S3DataLoader`, `FsDataLoader`, `DbDataLoader`, `PostgresMetadataStore`, `S3TraceWriter`.

Test implementation: `InMemoryDataLoader` (from test-resolver), `InMemoryMetadataStore`, `InMemoryTraceWriter`.

---

## DSL Compilation Pipeline

```
Expression string (YAML)
     │
     ▼
Parser (pest/lalrpop) → AST
     │
     ▼
Selector interpolation: {{NAME}} → inline expression
     │
     ▼
Column resolution: validate against Dataset schema + join aliases
     │
     ▼
Type checking: ensure type compatibility
     │
     ▼
Polars Expr generation: AST → polars::lazy::dsl::Expr
     │
     ▼
Attached to LazyFrame operation (.filter / .with_column / .group_by)
```

---

## Sandbox Behaviour (Draft Mode)

When the API server dispatches a Run for a `draft` Project, it modifies the `RunSpec` before creating the K8s Job:

- All `output` operation destinations are replaced with the deployment-level **sandbox DataSource** configuration.
- The engine worker is unaware of the substitution — it writes to whatever destination is in the `RunSpec`.
- The sandbox DataSource is configured as an environment variable or config file on the API server.

---

## Key Technology Choices

| Concern | Library / Tool |
|---|---|
| HTTP framework | `axum` |
| PostgreSQL client | `sqlx` (compile-time checked queries) |
| K8s client | `kube-rs` |
| CLI framework | `clap` |
| DSL parser | `pest` or `lalrpop` (evaluate during implementation) |
| Object storage | `object_store` crate (unified S3/GCS/Azure/local filesystem) |
| Serialization | `serde` + `serde_yaml` + `serde_json` |
| Schema migrations | `sqlx-migrate` |
| DataFrame engine | `polars` (lazy API) |
| Testing | `cargo test` + test harness (YAML scenarios) |

---

## Open Questions

| # | Question | Status |
|---|---|---|
| OQ-001 | How does the engine worker receive the `RunSpec` — K8s ConfigMap, command-line argument, or fetched from PostgreSQL by Run ID? | Open |
| OQ-002 | Should the API server poll K8s Job status or use a K8s watch/informer for real-time updates? | Open |
| OQ-003 | How is the sandbox DataSource configured — environment variable, config file, or a named DataSource in PostgreSQL? | Open |
| OQ-004 | Should trace Parquet files be partitioned by operation or written as a single file per Run? | Open |
| OQ-005 | DSL parser choice: `pest` (PEG, simpler) vs `lalrpop` (LALR, more powerful) — evaluate during Expression implementation | Open |
