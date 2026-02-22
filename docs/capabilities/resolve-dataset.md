# Capability: Resolve Dataset

**Status**: Draft  
**Created**: 2026-02-22  
**Domain**: Computation / Data Access

## Definition

Resolve Dataset is the capability by which the system, given a Dataset and a Period, invokes the appropriate Resolver to locate and load the physical data for that Dataset's tables and produce a **ResolutionResult** — an implementation-specific DataHandle (e.g., a DataFrame or DuckDB view) together with a status and diagnostics. The Resolver validates the loaded data against the Dataset's declared schema before returning. The caller decides whether an empty or failed resolution is acceptable based on context.

## Purpose & Role

This capability is the bridge between the Dataset's pure logical contract (schema, structure) and the actual physical data needed for computation. Without it, the execution engine has no way to access data — the Dataset intentionally holds no location information. By making resolution pluggable, the system transparently supports heterogeneous environments: legacy parquet stores with custom naming schemes, databases, cloud storage, and future storage types, all behind a uniform interface.

## Inputs

| Input | Type | Required | Constraints | Description |
|---|---|---|---|---|
| `dataset` | `Dataset` | Yes | Must be in `active` status; must have at least one ColumnDef on each TableRef | The Dataset whose schema defines what data is expected |
| `period` | `Period` | Yes | Must exist in the system | The Period that scopes which physical data to load |

## Outputs

| Output | Type | Description |
|---|---|---|
| `result` | `ResolutionResult` | Envelope containing the DataHandle, status, and diagnostics |

### ResolutionResult (structure)

| Field | Type | Description |
|---|---|---|
| `handle` | `DataHandle` | Implementation-specific data accessor (e.g., Pandas DataFrame, DuckDB view). Present when `status` is `resolved`; may be an empty handle when `status` is `empty` |
| `status` | `Enum` | `resolved` — data found and schema-valid; `empty` — no data found for the Period; `error` — resolution failed (schema mismatch, connection failure, etc.) |
| `diagnostics` | `List<String>` | Human-readable messages describing what happened. Populated on `empty` and `error`; may contain warnings on `resolved` |

### DataHandle

A DataHandle is an opaque, implementation-specific reference to the loaded data. Its concrete type depends on the execution runtime:

| Runtime | DataHandle type |
|---|---|
| Pandas | `DataFrame` |
| DuckDB | Relation / View |
| (future) | Implementation-defined |

The engine uses the DataHandle to execute Operations against the resolved data. The DataHandle is never persisted — it is in-memory for the duration of the Run.

## Trigger

Invoked by the **Execute Project Calculation** capability at the start of a Run (and at the point of any `append` or RuntimeJoin operation that references another Dataset or table). Resolution is always initiated by the engine — never directly by a user.

## Preconditions

- **PRE-001**: The Dataset MUST be in `active` status.
- **PRE-002**: The Period MUST exist in the system.
- **PRE-003**: A Resolver MUST be available — either the Dataset's configured `resolver_id` or the system default. If neither is available, the capability cannot execute.

## Postconditions

- **POST-001**: A ResolutionResult is always returned — the capability never raises an unhandled exception. All failures are captured in `status` and `diagnostics`.
- **POST-002**: When `status` is `resolved`, the DataHandle conforms to the Dataset's declared schema (column names, types, nullability). The Resolver is responsible for this guarantee.
- **POST-003**: When `status` is `empty`, the DataHandle contains zero rows but is still schema-valid (correct columns and types).
- **POST-004**: When `status` is `error`, the DataHandle is absent (`null`). The caller must not attempt to use it.

## Error Cases

| Error | Trigger Condition | Handling |
|---|---|---|
| `ResolverNotFound` | No Resolver is registered for the Dataset's `resolver_id`, and no default Resolver is configured | `status: error`; diagnostics include the missing resolver identifier |
| `DataNotFound` | The Resolver finds no physical data matching the Dataset + Period combination | `status: empty`; diagnostics describe the attempted lookup |
| `SchemaMismatch` | The physical data's columns or types do not match the Dataset's declared schema | `status: error`; diagnostics list each mismatched column |
| `ConnectionFailure` | The Resolver cannot connect to the underlying data store | `status: error`; diagnostics include the connection error detail |
| `PartialResolution` | Some tables in the Dataset resolve successfully but others fail | `status: error`; diagnostics identify which tables failed; no partial DataHandle is returned |

## Boundaries

- This capability does **NOT** execute Operations — it only loads data for use by the engine.
- This capability does **NOT** decide whether an `empty` or `error` result is fatal — that decision belongs to the caller (Execute Project Calculation).
- This capability does **NOT** cache or persist DataHandles — data is loaded fresh on each resolution.
- This capability does **NOT** define or configure Resolvers — Resolver registration is a separate concern.
- This capability does **NOT** apply Period filters to rows — the Resolver is responsible for returning only rows relevant to the given Period.

## Dependencies

| Dependency | Type | Description |
|---|---|---|
| Dataset | Entity | Provides the expected schema (columns, types, nullability) and the `resolver_id` override if set |
| Period | Entity | Passed to the Resolver to scope which physical data to load |
| Resolver | Entity / Plugin | The pluggable implementation that performs the actual data location and loading |
| Execute Project Calculation | Capability | The primary caller; invokes this capability at Run start and during RuntimeJoin/append operations |

## Open Questions

- [ ] Should the Resolver receive the full Dataset entity (schema + all metadata) or just the TableRef being resolved? Some resolvers may need only the table name and period; others may need the full schema for validation.
- [ ] How is the system-default Resolver configured — as a deployment-level setting, or as a named entry in a Resolver registry?
- [ ] Should resolution of lookup tables be lazy (resolved only when the Operation that needs them executes) or eager (all tables resolved at Run start)?
- [ ] For Datasets that reference nested Datasets as lookups, does each nested Dataset use its own `resolver_id`, or does the parent Dataset's resolver handle all tables?
