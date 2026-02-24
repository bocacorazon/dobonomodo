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

# Test harness - execute single scenario
dobo test <scenario.yaml>

# Test harness - execute test suite
dobo test --suite tests/scenarios

# Test harness - with output options
dobo test <scenario.yaml> --verbose
dobo test --suite tests/scenarios --output json
dobo test --suite tests/scenarios --output junit > results.xml
```

## Test Harness Usage

The test harness validates that the computation engine produces correct results by comparing actual output to expected output defined in YAML scenario files.

### Quick Start Example

Create a test scenario file `my-test.yaml`:

```yaml
name: "My First Test"
description: "Validates passthrough behavior"

periods:
  - identifier: "2026-01"
    level: "month"
    start_date: "2026-01-01"
    end_date: "2026-01-31"

input:
  dataset:
    id: "550e8400-e29b-41d4-a716-446655440000"
    name: "test_data"
    description: "Test dataset"
    owner: "test"
    version: 1
    status: active
    main_table:
      name: "orders"
      temporal_mode: period
      columns:
        - name: "order_id"
          type: integer
          nullable: false
        - name: "amount"
          type: decimal
          nullable: false
  
  data:
    orders:
      rows:
        - order_id: 1
          amount: 100.0
          _period: "2026-01"
        - order_id: 2
          amount: 200.0
          _period: "2026-01"

project:
  id: "660e8400-e29b-41d4-a716-446655440001"
  name: "passthrough"
  owner: "test"
  version: 1
  status: active
  visibility: private
  input_dataset_id: "550e8400-e29b-41d4-a716-446655440000"
  input_dataset_version: 1
  materialization: eager
  operations:
    - order: 1
      type: output
      parameters: {}

expected_output:
  data:
    rows:
      - order_id: 1
        amount: 100.0
      - order_id: 2
        amount: 200.0

config:
  match_mode: exact
  validate_metadata: false
  validate_traceability: false
  snapshot_on_failure: true
```

Run the test:

```bash
cargo run --bin cli -- test my-test.yaml
```

### Test Configuration Options

- `match_mode`: `exact` (all rows must match) or `subset` (expected rows must exist, extras tolerated)
- `validate_metadata`: Include system columns (`_row_id`, `_created_at`, etc.) in comparison
- `validate_traceability`: Validate trace events match expected assertions
- `snapshot_on_failure`: Save actual output to `.snapshots/` directory on failure
- `order_sensitive`: Require row order to match (default: false)

### Test Suite Organization

Organize test scenarios in directories:

```
tests/scenarios/
├── basic/
│   ├── passthrough-test.yaml
│   └── simple-update-test.yaml
├── complex/
│   ├── multi-operation-test.yaml
│   └── trace-validation-test.yaml
└── data/
    ├── orders.csv
    └── products.parquet
```

Run all tests:

```bash
cargo run --bin cli -- test --suite tests/scenarios
```

For more details, see:
- Full quickstart: `/workspace/specs/003-test-harness/quickstart.md`
- Test scenario specification: `/workspace/specs/003-test-harness/spec.md`
- Data model: `/workspace/specs/003-test-harness/data-model.md`

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
