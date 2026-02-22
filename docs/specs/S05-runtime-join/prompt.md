# S05: RuntimeJoin

## Feature
Implement RuntimeJoin resolution for the `update` operation: resolve a join Dataset via the Resolver, load it through `DataLoader`, apply period filtering based on the join table's `temporal_mode`, and attach it to the working `LazyFrame` under an alias so assignment expressions can reference `alias.column_name`.

## Context
- Read: `docs/entities/operation.md` (RuntimeJoin structure, BR-008a/008b/008c, dataset_id/dataset_version fields)
- Read: `docs/entities/dataset.md` (schema, `temporal_mode` per TableRef)
- Read: `docs/architecture/sample-datasets.md` (TS-03 FX conversion — joins `exchange_rates` bitemporal table)

## Scope

### In Scope
- `core::engine::join` module
- Resolve join Dataset: look up by `dataset_id` + optional `dataset_version` via `MetadataStore`
- Resolver selection: Project `resolver_overrides` → Dataset `resolver_id` → system default (same precedence as input Dataset)
- Load join data via `DataLoader` with resolved locations
- Period filter the join data using the join table's own `temporal_mode` and the Run's current Period
- Polars join: left join working `LazyFrame` with join `LazyFrame` using compiled `on` expression
- Column aliasing: joined columns available as `alias.column_name` in downstream expressions
- Multiple joins per operation (list of RuntimeJoin, each independent)
- Capture resolved `dataset_id + dataset_version` for `ResolverSnapshot`
- Test with `InMemoryDataLoader` and TS-03 scenario (FX conversion through bitemporal join)

### Out of Scope
- Self-join support (deferred per operation.md OQ-002)
- Join types other than left join

## Dependencies
- **S01** (DSL Parser): compile `on` expression
- **S03** (Period Filter): filter join data by `temporal_mode`
- **S04** (Update Operation): integrates with update to provide join columns to assignments

## Parallel Opportunities
Can run in parallel with **S06, S07, S08, S09, S11** (but depends on S04 for integration).

## Key Design Decisions
- Join source is a Dataset ID, not a raw TableRef — resolved via Resolver
- Period used is always the Run's current Period (no per-join override)
- Version can be pinned or left floating (latest at Run time)
- Resolver precedence is identical to input Dataset
- Join aliases are operation-scoped only

## Success Criteria
- Bitemporal join table (`exchange_rates`) returns correct asOf rates for the Run's period
- Join columns are accessible as `alias.column_name` in expressions
- Multiple joins in one operation work independently
- Missing join Dataset produces clear error
- Floating version resolves to latest; pinned version uses exact
