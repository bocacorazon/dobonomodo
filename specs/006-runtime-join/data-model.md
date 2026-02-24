# Data Model: Runtime Join Resolution

**Feature**: 006-runtime-join  
**Created**: 2026-02-22  
**Purpose**: Define data structures and relationships for RuntimeJoin feature

---

## Entities

### RuntimeJoin

**Description**: Configuration for a single runtime join within an update operation. Specifies which external dataset to join, how to join it, and what alias to use for referencing joined columns.

**Attributes**:

| Field | Type | Required | Validation | Description |
|-------|------|----------|------------|-------------|
| `alias` | String | Yes | Non-empty, unique within operation, alphanumeric+underscore | Logical name for referencing joined columns in expressions (e.g., "fx", "customers") |
| `dataset_id` | UUID | Yes | Must reference an existing, active Dataset | ID of the Dataset to join |
| `dataset_version` | Integer | No | Positive integer; must exist if specified | Pins to specific Dataset version. Omit for latest active version at Run time |
| `on` | Expression | Yes | Boolean expression; must reference valid columns | Join condition (e.g., "transactions.currency = fx.from_currency AND fx.to_currency = 'USD'") |

**Lifecycle**: Created and validated at Project activation. Immutable once snapshotted into a Run.

**Relationships**:
- Embedded in `UpdateOperation.arguments.joins` (0 to many per operation)
- References `Dataset` by `dataset_id`
- Resolved to physical data via `Resolver` and `DataLoader`
- Filtered by `Period` according to join table's `temporal_mode`

**State Transitions**: None (immutable structure)

**Validation Rules**:
- `alias` must not conflict with working dataset table names or other join aliases in the same operation
- `dataset_id` must exist in MetadataStore and have status `active`
- `dataset_version`, if provided, must exist for the given `dataset_id`
- `on` expression must compile successfully and reference only valid columns from working dataset and join dataset

---

### ResolverSnapshot (extension)

**Description**: Existing entity extended to capture resolved join dataset versions for reproducibility.

**New Field**:

| Field | Type | Description |
|-------|------|-------------|
| `join_datasets` | List<JoinDatasetSnapshot> | One entry per RuntimeJoin resolution with alias, dataset, version, and resolver source |

**Purpose**: When a Run executes, each RuntimeJoin's dataset version is resolved (pinned or latest). This list records each resolved join (including alias), ensuring re-execution of the Run uses the same data even if newer Dataset versions are published.

**Example**:
```yaml
resolver_snapshot:
  input_dataset_id: "ds-gl"
  input_dataset_version: 3
  join_datasets:
    - alias: "fx"
      dataset_id: "ds-exchange-rates"
      dataset_version: 2
      resolver_source: "project_override"
    - alias: "customers"
      dataset_id: "ds-customers"
      dataset_version: 5
      resolver_source: "dataset_resolver"
```

---

### UpdateOperation (extension)

**Description**: Existing entity extended to include `joins` argument.

**Modified Argument Schema**:

```yaml
arguments:
  joins:  # NEW FIELD (optional list)
    - alias: string
      dataset_id: uuid
      dataset_version: integer  # optional
      on: expression
  assignments:  # EXISTING FIELD
    - column: string
      expression: expression
```

**Behavior**: 
- `joins` is optional; defaults to empty list
- Joins are resolved and applied in order before evaluating assignments
- Join aliases are scoped to this operation only; not visible to other operations
- Assignment expressions may reference `alias.column_name` for any alias in `joins`

---

## Relationships

```
Project
  `-- resolver_overrides: Map<UUID, String>  # dataset_id -> resolver_name

UpdateOperation
  |-- joins: List<RuntimeJoin>
  |     |-- dataset_id --references--> Dataset
  |     |-- dataset_version --optional--> Dataset.version
  |     `-- on: Expression
  |
  `-- assignments: List<Assignment>
        `-- expression --may reference--> RuntimeJoin.alias

Dataset
  |-- id: UUID
  |-- version: Integer
  |-- resolver_id: String (optional)
  `-- main_table: TableRef
        `-- temporal_mode: TemporalMode

Run
  |-- period: Period
  `-- resolver_snapshot: ResolverSnapshot
        `-- join_datasets: List<JoinDatasetSnapshot>  # NEW

Resolver (interface)
  `-- resolve(dataset, period) -> Location

DataLoader (interface)
  `-- load(location, period) -> LazyFrame

Period
  |-- identifier: String
  |-- start_date: Date
  `-- end_date: Date
```

---

## Data Flow

1. **Resolution Phase** (at Run initialization):
   - For each RuntimeJoin in the UpdateOperation:
     - Resolve dataset version (pinned or latest active)
     - Apply resolver precedence: Project override -> Dataset resolver_id -> system default
     - Query Resolver for physical location of join Dataset's tables
     - Record resolved version in `Run.resolver_snapshot.join_datasets`

2. **Load Phase** (at operation execution):
   - For each RuntimeJoin:
     - Load join Dataset via DataLoader -> LazyFrame
     - Apply period filter based on join table's `temporal_mode`:
       - `period`: filter `_period == run.period.identifier`
       - `bitemporal`: filter `_period_from <= run.period.start_date AND (_period_to IS NULL OR _period_to > run.period.start_date)`
     - Suffix join columns with `_<alias>`

3. **Join Phase**:
   - For each RuntimeJoin:
     - Compile `on` expression to Polars boolean expression
     - Perform left join: `working_lf.join(join_lf, on, JoinType::Left)`
     - Joined columns available as `<column>_<alias>` in Polars

4. **Assignment Phase**:
   - Compile assignment expressions with symbol table containing:
     - Working dataset columns
     - Join aliases mapped to suffixed column names
   - Evaluate assignments on joined LazyFrame

---

## Schema Evolution

### V1 (this feature)
- `RuntimeJoin` structure with 4 fields
- `ResolverSnapshot.join_datasets` list
- `UpdateOperation.arguments.joins` list

### Future Considerations
- **Join types**: Add `join_type` field (left/inner/right/full) - currently hardcoded to left
- **Join optimization hints**: Add `expected_cardinality` for query planning
- **Cross-dataset joins**: Join two external datasets to each other, not just to working dataset
- **Self-join**: Join working dataset to itself with different aliases (deferred per OQ-002)

---

## Validation Rules (compile-time)

| Rule | Check | Error Message |
|------|-------|---------------|
| VR-001 | RuntimeJoin.dataset_id exists | "Dataset {dataset_id} not found in MetadataStore" |
| VR-002 | RuntimeJoin.dataset_id status is active | "Dataset {dataset_id} is disabled and cannot be used in joins" |
| VR-003 | RuntimeJoin.dataset_version (if provided) exists | "Dataset {dataset_id} version {version} not found" |
| VR-004 | RuntimeJoin.alias is unique within operation | "Join alias '{alias}' is already used in this operation" |
| VR-005 | RuntimeJoin.alias does not conflict with working dataset table | "Join alias '{alias}' conflicts with working dataset table name" |
| VR-006 | RuntimeJoin.on expression compiles | "Join condition failed to compile: {error}" |
| VR-007 | RuntimeJoin.on references valid columns | "Unknown column '{column}' in join condition" |
| VR-008 | Assignment expression using join alias references valid column | "Unknown column '{alias}.{column}' in assignment expression" |

---

## Invariants

- A Run's `resolver_snapshot.join_datasets` list contains an entry for every RuntimeJoin in every UpdateOperation in the Project
- Resolved dataset versions are immutable for a given Run (re-execution uses the same versions)
- Join aliases are scoped to their operation - the same alias can be reused in different operations with different semantics
- Period filtering always uses the Run's current Period; there is no per-join Period override
- Join order in the `joins` list is preserved during execution (joins are applied sequentially to the working LazyFrame)
