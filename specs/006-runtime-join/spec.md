# Feature Specification: Runtime Join Resolution

**Feature Branch**: `006-runtime-join`  
**Created**: 2026-02-22  
**Status**: Draft  
**Input**: Implement RuntimeJoin resolution for the `update` operation: resolve a join Dataset via the Resolver, load it through DataLoader, apply period filtering based on the join table's temporal_mode, and attach it to the working LazyFrame under an alias so assignment expressions can reference alias.column_name.

## User Scenarios & Testing *(mandatory)*

### User Story 1 - Execute FX Conversion Join (Priority: P1)

As a financial analyst running a reporting pipeline, I want the update operation to automatically join exchange rate data and compute reporting currency amounts so multi-currency transactions can be normalized without manual data preparation.

**Why this priority**: This is the primary use case driving RuntimeJoin - enabling dynamic enrichment from external datasets during computation without requiring pre-joined input data.

**Independent Test**: Provide a GL transactions dataset with local currency amounts and a bitemporal exchange_rates dataset. Execute an update operation with a RuntimeJoin that references the exchange_rates dataset, joins on currency match, and computes amount_reporting. Verify the output contains correctly converted amounts using the asOf exchange rate for the run period.

**Acceptance Scenarios**:

1. **Given** a working dataset with transactions in USD, EUR, GBP, JPY and a join to a bitemporal exchange_rates dataset, **When** the update operation executes with join on currency and assignment expression `amount_local * fx.rate`, **Then** the output contains amount_reporting computed using the correct asOf rate for the run period (EUR 1.0920, GBP 1.2710, JPY 0.00672, USD 1.0000).
2. **Given** a RuntimeJoin with dataset_version pinned to a specific version, **When** the join resolves, **Then** the exact pinned version is loaded regardless of newer versions being available, and the resolved version is recorded in the resolver snapshot.
3. **Given** a RuntimeJoin with dataset_version omitted, **When** the join resolves, **Then** the latest active version is loaded, and the resolved version is recorded in the resolver snapshot.

---

### User Story 2 - Support Multiple Independent Joins (Priority: P2)

As a data engineer building complex transformations, I want a single update operation to support multiple RuntimeJoins so I can enrich working data from multiple external sources in one step.

**Why this priority**: Multi-source enrichment is a common pattern in data pipelines. Supporting multiple joins in a single operation simplifies pipeline design and improves performance by batching enrichment logic.

**Independent Test**: Define an update operation with two RuntimeJoins: one to customers (to add customer tier) and one to products (to add product category). Execute the operation and verify both join aliases are available in assignment expressions and the correct enriched values appear in the output.

**Acceptance Scenarios**:

1. **Given** an update operation with joins to both customers and products datasets, **When** the operation executes, **Then** assignment expressions can reference both `customers.tier` and `products.category`, and the output contains enriched values from both sources.
2. **Given** two RuntimeJoins using different temporal_mode settings (one period, one bitemporal), **When** the operation executes, **Then** each join dataset is period-filtered according to its own temporal_mode independently.
3. **Given** multiple RuntimeJoins in the same operation, **When** one join fails to resolve, **Then** the entire operation fails with a clear error indicating which dataset could not be resolved.

---

### User Story 3 - Apply Correct Period Filtering per Temporal Mode (Priority: P1)

As a system operator running period-based computations, I want join datasets to be filtered using their own temporal_mode configuration so bitemporal joins return asOf snapshots and period joins return exact matches for the run period.

**Why this priority**: Correct temporal filtering is critical for data accuracy. Bitemporal datasets (like exchange rates) require asOf logic, while period datasets require exact match. Incorrect filtering produces wrong results.

**Independent Test**: Execute a join against a bitemporal exchange_rates table for run period 2026-01 (starting 2026-01-01). Verify the asOf query selects rates where _period_from <= 2026-01-01 AND (_period_to IS NULL OR _period_to > 2026-01-01), returning the 2026-01-01 rate (1.0920 for EUR) not the 2025-01-01 rate (1.0850).

**Acceptance Scenarios**:

1. **Given** a RuntimeJoin to a bitemporal table with multiple rate versions, **When** the join executes for period 2026-01, **Then** the asOf filter selects rows where _period_from <= 2026-01-01 AND (_period_to IS NULL OR _period_to > 2026-01-01).
2. **Given** a RuntimeJoin to a period table, **When** the join executes for period 2026-01, **Then** the exact match filter selects rows where _period = "2026-01".
3. **Given** a RuntimeJoin where the join table has no matching period data, **When** the operation executes, **Then** the join produces zero matches and assignment expressions using join columns result in NULL values for unmatched rows.

---

### User Story 4 - Resolve Dataset via Resolver with Correct Precedence (Priority: P2)

As a project owner configuring resolver overrides, I want RuntimeJoin datasets to use the same resolver precedence as the input dataset (Project resolver_overrides -> Dataset resolver_id -> system default) so join data sources can be controlled consistently across the entire computation.

**Why this priority**: Resolver configuration determines data location. Consistent precedence ensures that environment-specific overrides (e.g., test vs production) apply uniformly to both input and join datasets.

**Independent Test**: Configure a Project with resolver_overrides that points to a test resolver. Execute an update with a RuntimeJoin to a dataset that specifies a different resolver_id. Verify the Project's resolver_override takes precedence and the join data is loaded from the test resolver.

**Acceptance Scenarios**:

1. **Given** a Project with resolver_overrides and a RuntimeJoin to a dataset with its own resolver_id, **When** the join resolves, **Then** the Project resolver_override takes precedence.
2. **Given** a RuntimeJoin to a dataset with resolver_id set and no Project resolver_override, **When** the join resolves, **Then** the dataset's resolver_id is used.
3. **Given** a RuntimeJoin to a dataset with no resolver_id and no Project resolver_override, **When** the join resolves, **Then** the system default resolver is used.

### Edge Cases

- A RuntimeJoin references a dataset_id that does not exist, and the operation fails with a clear "Dataset not found" error before attempting to load data.
- A RuntimeJoin references a dataset_id that exists but is disabled, and the operation fails with a clear "Dataset is disabled" error.
- A RuntimeJoin has an `on` expression that references an unknown column, and the operation fails at compile time with a clear "Unknown column reference" error.
- A RuntimeJoin to a bitemporal table with overlapping period ranges for the same logical entity produces undefined results (documented behavior, not enforced by engine).
- An update operation has zero RuntimeJoins, and assignment expressions reference only working dataset columns - this is valid and executes normally.
- An assignment expression references a join alias that does not exist in the joins list, and the operation fails at compile time with a clear error.

## Requirements *(mandatory)*

### Functional Requirements

- **FR-001**: The update operation MUST support a `joins` argument containing a list of zero or more RuntimeJoin definitions.
- **FR-002**: Each RuntimeJoin MUST specify an `alias` (String), `dataset_id` (UUID), `on` (Boolean Expression), and optionally `dataset_version` (Integer).
- **FR-003**: The engine MUST resolve the join dataset using the Resolver with precedence: Project `resolver_overrides` -> Dataset `resolver_id` -> system default resolver.
- **FR-004**: When `dataset_version` is omitted, the engine MUST resolve to the latest active version of the dataset at Run time.
- **FR-005**: When `dataset_version` is provided, the engine MUST resolve to the exact pinned version regardless of newer versions.
- **FR-006**: The engine MUST capture the resolved `dataset_id + dataset_version` in the Run's `ResolverSnapshot` for reproducibility.
- **FR-007**: The engine MUST load the join dataset via the DataLoader using the resolved location from the Resolver.
- **FR-008**: The engine MUST period-filter the join data using the join table's own `temporal_mode` and the Run's current Period.
- **FR-009**: For `temporal_mode: period`, the engine MUST filter join rows where `_period = run_period.identifier` (exact match).
- **FR-010**: For `temporal_mode: bitemporal`, the engine MUST filter join rows where `_period_from <= run_period.start_date AND (_period_to IS NULL OR _period_to > run_period.start_date)` (asOf query).
- **FR-011**: The engine MUST perform a left join between the working LazyFrame and the join LazyFrame using the compiled `on` expression. The currently supported `on` subset is AND-connected equality predicates between working and join references, plus join-only filter predicates.
- **FR-012**: Joined columns MUST be available in assignment expressions using the syntax `alias.column_name`.
- **FR-013**: Multiple RuntimeJoins in the same operation MUST execute independently, and each alias MUST be unique within the operation.
- **FR-014**: Join aliases MUST be operation-scoped - they are not visible to other operations in the pipeline.
- **FR-015**: If a RuntimeJoin references a dataset_id that does not exist or is disabled, the operation MUST fail with a clear error before attempting data load.
- **FR-016**: If a RuntimeJoin `on` expression or assignment expression references an unknown column, the operation MUST fail at compile time with a clear error.
- **FR-017**: The engine MUST test RuntimeJoin with the InMemoryDataLoader and the TS-03 scenario (FX conversion via bitemporal join).

### Key Entities *(include if feature involves data)*

- **RuntimeJoin**: Embedded structure in update operation arguments defining a join to an external dataset. Contains `alias` (String), `dataset_id` (UUID), `dataset_version` (optional Integer), `on` (Expression).
- **ResolverSnapshot**: Run metadata capturing the resolved dataset version for each join, ensuring reproducibility.
- **DataLoader**: Component responsible for loading dataset rows from resolved physical locations.
- **LazyFrame**: Polars lazy execution structure representing the working dataset and joined datasets.
- **Period**: Time partition defining the current run period, used to filter both working dataset and join datasets according to their temporal_mode.

## Assumptions

- The DSL Parser (S01) is implemented and can compile `on` expressions to executable Polars expressions.
- The Period Filter (S03) is implemented and provides the filtering logic for both period and bitemporal temporal_mode.
- The Update Operation (S04) is implemented and provides the integration point for RuntimeJoin.
- The MetadataStore provides a lookup method to retrieve Dataset definitions by id and optional version.
- The Resolver is implemented and returns physical locations for TableRefs given a Period.
- The DataLoader is implemented and can load data from resolved locations into Polars LazyFrames.
- The InMemoryDataLoader is available for testing and can be seeded with sample data matching the TS-03 scenario.
- Self-join support (joining the working dataset to itself) is deferred per operation.md OQ-002.
- Join types other than left join are out of scope.
- Overlapping period ranges in bitemporal datasets are a data contract violation handled by the data producer, not enforced by the engine.

## Success Criteria *(mandatory)*

### Measurable Outcomes

- **SC-001**: 100% of TS-03 FX conversion test scenarios pass: EUR, GBP, JPY, and USD amounts are correctly converted using asOf exchange rates for run period 2026-01.
- **SC-002**: 100% of RuntimeJoin operations with pinned dataset_version load the exact specified version and record it in the ResolverSnapshot.
- **SC-003**: 100% of RuntimeJoin operations with omitted dataset_version load the latest active version and record it in the ResolverSnapshot.
- **SC-004**: 100% of multi-join update operations execute successfully, with each join alias independently accessible in assignment expressions.
- **SC-005**: 100% of invalid RuntimeJoin references (nonexistent dataset, disabled dataset, unknown columns) fail at compile or resolve time with clear error messages, not at execution time.
- **SC-006**: Bitemporal join datasets are filtered using asOf logic, and period join datasets are filtered using exact match logic, with 100% test coverage for both temporal_mode values.
