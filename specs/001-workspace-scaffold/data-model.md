# Data Model: Workspace Scaffold Baseline

## Overview

This phase defines compile-time domain contracts and serialization surfaces only. No runtime execution behavior is implemented in S00.

## Entity Catalog

### Dataset
- **Fields**: `id`, `name`, `description`, `owner`, `version`, `status`, `resolver_id`, `main_table`, `lookups`, `natural_key_columns`, `created_at`, `updated_at`
- **Embedded**: `TableRef`, `ColumnDef`, `LookupDef`, `JoinCondition`
- **Validation rules**:
  - Exactly one `main_table`
  - At least one `columns` entry per table
  - User column names must not use `_` prefix
  - `temporal_mode` in `{period,bitemporal}`

### Project
- **Fields**: `id`, `name`, `description`, `owner`, `version`, `status`, `visibility`, `input_dataset_id`, `input_dataset_version`, `materialization`, `operations`, `selectors`, `resolver_overrides`, `conflict_report`, `created_at`, `updated_at`
- **Embedded**: `OperationInstance`, `ConflictReport`, `BreakingChange`
- **Validation rules**:
  - `operations` non-empty and ordered
  - `status` in `{draft,active,inactive,conflict}`
  - `materialization` in `{eager,runtime}`

### OperationInstance
- **Fields**: `order`, `type`, `alias`, `parameters`
- **Validation rules**:
  - `order` unique per project
  - `type` maps to `OperationKind`

### Run
- **Fields**: `id`, `project_id`, `project_version`, `project_snapshot`, `period_ids`, `status`, `trigger_type`, `triggered_by`, `last_completed_operation`, `output_dataset_id`, `parent_run_id`, `error`, `started_at`, `completed_at`, `created_at`
- **Embedded**: `ProjectSnapshot`, `ResolverSnapshot`, `ErrorDetail`
- **Validation rules**:
  - `period_ids` non-empty
  - `status` in `{queued,running,completed,failed,cancelled}`

### Resolver
- **Fields**: `id`, `name`, `description`, `version`, `status`, `is_default`, `rules`, `created_at`, `updated_at`
- **Embedded**: `ResolutionRule`, `ResolutionStrategy`
- **Validation rules**:
  - `rules` non-empty, first-match ordering semantics
  - `status` in `{active,disabled}`

### Calendar
- **Fields**: `id`, `name`, `description`, `status`, `is_default`, `levels`, `created_at`, `updated_at`
- **Embedded**: `LevelDef`, `DateRule`

### Period
- **Fields**: `id`, `identifier`, `name`, `description`, `calendar_id`, `year`, `sequence`, `start_date`, `end_date`, `status`, `parent_id`, `created_at`, `updated_at`
- **Validation rules**:
  - `start_date <= end_date`
  - `status` in `{open,closed,locked}`

### DataSource
- **Fields**: `id`, `name`, `description`, `owner`, `status`, `type`, `options`, `credential_ref`, `created_at`, `updated_at`
- **Validation rules**:
  - `status` in `{active,disabled}`
  - credentials must not be embedded in `options`

### Expression
- **Shape**: string newtype/value object (`source`) carried in operation parameters

## Enumerations

- `TemporalMode`: `period`, `bitemporal`
- `ColumnType`: `string`, `integer`, `decimal`, `boolean`, `date`, `timestamp`
- `RunStatus`: `queued`, `running`, `completed`, `failed`, `cancelled`
- `ProjectStatus`: `draft`, `active`, `inactive`, `conflict`
- `OperationKind`: `update`, `aggregate`, `append`, `output`, `delete`
- `StrategyType`: `path`, `table`, `catalog`
- `TriggerType`: `manual`, `scheduled`

## Core Relationships

- Project **consumes** one Dataset and defines many OperationInstances.
- Run **instantiates** one Project snapshot and references one or more Periods.
- Dataset and Project may both reference Resolver (project-level override precedence).
- Resolver strategies may reference DataSource identifiers.
- Calendar contains many Periods in a hierarchy.

## State Transitions (for enum-bearing entities)

- Project: `draft -> active -> inactive`; conflict path `active -> conflict -> draft/active` depending resolution.
- Run: `queued -> running -> completed|failed|cancelled` with retry semantics handled later.
- Resolver: `active <-> disabled`.
- Calendar: `draft -> active -> deprecated`.
- Period: `open -> closed -> locked`.

## IO Contracts (model-level interfaces)

- `DataLoader`: load resolved location into frame.
- `OutputWriter`: write output frame to destination.
- `MetadataStore`: retrieve/update metadata entities and run state.
- `TraceWriter`: persist run trace events.
