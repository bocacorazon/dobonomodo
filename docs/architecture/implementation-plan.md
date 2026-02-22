# Implementation Plan — DobONoMoDo

**Created**: 2026-02-22  
**Status**: Draft  
**Workflow**: Each spec follows the spec-kit pipeline: `specify → plan → tasks → implement`

---

## Strategy

The implementation is decomposed into **fine-grained specs**, each targeting a single capability, operation type, or infrastructure concern. Specs are ordered by dependency — earlier specs produce the traits, types, and test infrastructure that later specs build on. Where no dependency exists, specs can be developed **in parallel by separate agents**.

The test harness and sample datasets are developed first (Spec 02), making every subsequent spec testable from day one — aligning with the constitution's TDD mandate.

---

## Sample Data Domain: Financial / Accounting

All test scenarios use a consistent financial domain with the following tables:

- **transactions** — GL journal entries (main table, `temporal_mode: period`)
- **accounts** — chart of accounts (lookup, `temporal_mode: period`)
- **cost_centers** — organisational units (lookup, `temporal_mode: period`)
- **exchange_rates** — currency conversion rates (lookup, `temporal_mode: bitemporal`)
- **budgets** — budget line items (separate Dataset for append/join scenarios, `temporal_mode: period`)

See `docs/architecture/sample-datasets.md` for full schema, sample rows, and test scenario catalogue.

---

## Spec Inventory

### Phase 0 — Foundation

| Spec | Name | Description | Depends On | Parallel Group |
|---|---|---|---|---|
| **S00** | Workspace Scaffold | Cargo workspace, crate structure, entity model structs, IO trait definitions, empty modules — must compile | — | — |
| **S01** | DSL Parser & Expression Compiler | Parse expression strings → AST → Polars `Expr`; type-checking; column resolution; `{{SELECTOR}}` interpolation | S00 | A |
| **S02** | Test Harness | YAML scenario loader, metadata injection, built-in test Resolver, output comparison engine, diff reporting | S00 | A |

> **S01 and S02 can run in parallel** — they share S00 types but don't depend on each other.

### Phase 1 — Core Operations (each spec = one operation type)

| Spec | Name | Description | Depends On | Parallel Group |
|---|---|---|---|---|
| **S03** | Period Filter | Load data into `LazyFrame`; apply `temporal_mode`-based period filtering (`_period` exact match / bitemporal asOf) | S01, S02 | — |
| **S04** | Update Operation | Selector filtering, expression-based assignments, system column updates (`_updated_at`) | S03 | B |
| **S05** | RuntimeJoin | Resolve join Dataset via Resolver, load, period-filter, attach to working frame under alias, column references | S03 | B |
| **S06** | Delete Operation | Selector filtering, soft delete (`_deleted = true`), automatic exclusion from downstream operations | S03 | B |
| **S07** | Aggregate Operation | Group-by + aggregate expressions, append summary rows (not replace), `_row_id` generation for new rows | S03 | B |
| **S08** | Append Operation | Load source Dataset, optional selector, optional aggregation, column alignment, `_row_id` generation | S03 | B |
| **S09** | Output Operation | Write working dataset to destination, selector, column projection, `include_deleted`, `register_as_dataset` | S03 | B |

> **S04 through S09 can all run in parallel** — each is an independent operation type. They share S03 (period-filtered `LazyFrame`) but don't depend on each other.

### Phase 2 — Orchestration

| Spec | Name | Description | Depends On | Parallel Group |
|---|---|---|---|---|
| **S10** | Pipeline Executor | Sequential operation execution engine: iterate `ProjectSnapshot.operations`, dispatch to operation implementations, manage working `LazyFrame` state | S04–S09 (all operations) | — |
| **S11** | Resolver Engine | Rule evaluation, `when` condition matching, period expansion via Calendar hierarchy, template rendering, strategy dispatch → `Vec<ResolvedLocation>` | S01 | C |
| **S12** | Trace Engine | Before/after diff generation per operation, `TraceEvent` production, change type detection (`created`/`updated`/`deleted`) | S10 | — |

> **S11 can start as soon as S01 is done** — it uses Expression evaluation but not the pipeline. It can run in parallel with Phase 1 operations.

### Phase 3 — Lifecycle & Validation

| Spec | Name | Description | Depends On | Parallel Group |
|---|---|---|---|---|
| **S13** | Activation Validation | VAL-001–009: expression syntax, type-checking, column resolution, selector refs, Resolver availability; `ActivationError` reporting | S01, S11 | D |
| **S14** | Project Lifecycle | Status transitions (`draft`→`active`→`inactive`→`conflict`), structural edit detection, Dataset conflict model, `ConflictReport` | S13 | — |
| **S15** | Run Lifecycle | Run creation, `ProjectSnapshot` capture (including `resolver_snapshots`), status transitions, error recording, one-at-a-time guard per Project+Period | S10, S12 | D |

> **S13 and S15 can run in parallel** once their respective dependencies are met.

### Phase 4 — IO & Infrastructure

| Spec | Name | Description | Depends On | Parallel Group |
|---|---|---|---|---|
| **S16** | DataSource Adapters | `DataLoader` implementations: S3/Parquet adapter, filesystem adapter, database adapter (sqlx); `OutputWriter` implementations | S00 (traits) | E |
| **S17** | Metadata Store | PostgreSQL `MetadataStore` implementation: entity CRUD, migrations, version auto-increment | S00 (traits) | E |
| **S18** | Trace Writer | `TraceWriter` implementation: write `TraceEvent`s as Parquet to object storage, partitioned by Run | S12 | E |

> **S16, S17, S18 can all run in parallel** — they implement independent IO traits. S16 and S17 can start as early as Phase 0 is done.

### Phase 5 — Binaries & Deployment

| Spec | Name | Description | Depends On | Parallel Group |
|---|---|---|---|---|
| **S19** | Engine Worker Binary | `main.rs`: receive `RunSpec`, load metadata, resolve data, execute pipeline, write output + trace, report status | S10, S11, S12, S16, S17, S18 | — |
| **S20** | CLI Binary | `dobo test`, `dobo run`, `dobo validate`, `dobo parse` commands; local execution mode | S02, S10, S13 | F |
| **S21** | API Server | REST endpoints (entity CRUD, Run dispatch), K8s Job creation, sandbox redirect for draft mode | S17, S19 | — |

> **S20 can start as soon as S10 + S13 are done**, parallel with S19 infrastructure work.

---

## Parallelism Map

```
Phase 0:  S00
           │
     ┌─────┼─────┐
     ▼     ▼     ▼
    S01   S02   S16,S17 ◄── can start immediately after S00
     │     │
     ▼     ▼
    S03   (test harness ready)
     │
     ├──────────────────────────┐
     ▼     ▼     ▼     ▼     ▼ ▼
    S04   S05   S06   S07   S08 S09   S11 ◄── all parallel (Phase 1 + Resolver)
     │     │     │     │     │   │     │
     └─────┴─────┴─────┴─────┴───┘     │
                  │                     │
                  ▼                     ▼
                 S10                   S13 ◄── parallel
                  │                     │
              ┌───┤                     ▼
              ▼   ▼                    S14
             S12  S15
              │    
              ▼    
             S18   
              │    
     ┌────────┤
     ▼        ▼
    S19      S20 ◄── parallel
     │
     ▼
    S21
```

**Maximum parallelism**: up to **8 specs simultaneously** during Phase 1 (S04–S09 + S11 + S16/S17).

---

## Spec Document Conventions

Each spec is a self-contained prompt for `speckit.specify`. It lives at:

```
docs/specs/S##-spec-name/prompt.md
```

After running `speckit.specify`, the spec-kit pipeline produces:
```
docs/specs/S##-spec-name/
├── prompt.md      # input to speckit.specify (what we produce now)
├── spec.md        # output of speckit.specify
├── plan.md        # output of speckit.plan
└── tasks.md       # output of speckit.tasks
```

Each `prompt.md` includes:
1. **Feature name** and one-line summary
2. **Context references** — which entity/capability docs to read
3. **Scope** — exactly what this spec covers and what it does NOT
4. **Sample test scenarios** — concrete YAML examples using the financial domain data
5. **Dependencies** — which other specs must be complete before this one starts
6. **Parallel opportunities** — which specs can run alongside this one
7. **Key design decisions** — already-made decisions that the spec should honour (not re-debate)
