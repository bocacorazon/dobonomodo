# Feature Specification: Append Operation

**Feature Branch**: `009-append-operation`  
**Created**: 2026-02-22  
**Status**: Draft  
**Input**: User description: "Implement the `append` operation type: load rows from a source Dataset, optionally filter with `source_selector`, optionally aggregate before appending, align columns with the working dataset, and append the rows."

## User Scenarios & Testing *(mandatory)*

### User Story 1 - Basic Budget vs Actual Comparison (Priority: P1)

As a financial analyst, I need to append budget rows into the working dataset alongside actual transactions so that I can perform budget vs actual comparisons in a single analysis.

**Why this priority**: This is the core value proposition of the append operation - combining data from different datasets for side-by-side analysis. Without this, users cannot perform comparative analysis across datasets.

**Independent Test**: Can be fully tested by loading a transactions dataset, appending a budget dataset with matching columns, and verifying both transaction and budget rows exist in the output. Delivers the fundamental capability to merge datasets.

**Acceptance Scenarios**:

1. **Given** a working dataset with 10 transaction rows and a budget dataset with 4 budget rows, **When** an append operation references the budget dataset as source, **Then** the output contains 14 rows total (10 transactions + 4 budgets)
2. **Given** budget rows with columns that are a subset of the working dataset columns, **When** the append executes, **Then** budget rows are successfully appended with missing columns set to NULL
3. **Given** a working dataset with columns [journal_id, account_code, amount, description] and budget rows with columns [budget_id, account_code, amount], **When** append executes, **Then** budget rows have journal_id and description set to NULL

---

### User Story 2 - Filtered Source Data Append (Priority: P2)

As a financial analyst, I need to append only specific budget rows (e.g., only "original" budget type) from the budget dataset so that I can control which source data gets included in my analysis.

**Why this priority**: Filtering source data before appending is essential for selective data integration. This prevents polluting the working dataset with irrelevant rows and enables focused analysis.

**Independent Test**: Can be tested independently by appending a budget dataset with a source_selector that filters to specific budget types, and verifying only matching budget rows are appended.

**Acceptance Scenarios**:

1. **Given** a budget dataset with 12 rows containing budget_type values ["original", "revised", "forecast"], **When** append operation uses source_selector "budget_type = 'original'", **Then** only the 4 "original" budget rows are appended
2. **Given** a source_selector expression "amount > 10000", **When** append executes, **Then** only budget rows with amount greater than 10000 are appended to the working dataset
3. **Given** a source dataset with 100 rows and a highly selective source_selector matching only 5 rows, **When** append executes, **Then** exactly 5 rows are appended and 95 rows are filtered out

---

### User Story 3 - Aggregated Data Append (Priority: P3)

As a financial analyst, I need to append pre-aggregated summary rows from a source dataset (e.g., monthly totals by account) so that I can include summary statistics alongside detailed transaction data.

**Why this priority**: Aggregation before append enables hierarchical reporting and summary comparisons. This allows users to combine detail-level and summary-level data in one dataset for multi-level analysis.

**Independent Test**: Can be tested independently by appending a source dataset with aggregation configured (group_by + aggregations), and verifying the appended rows contain aggregated values rather than raw source rows.

**Acceptance Scenarios**:

1. **Given** a budget dataset with 12 rows, **When** append operation includes aggregation with group_by: [account_code] and aggregations: [SUM(amount) as total_budget], **Then** appended rows contain one row per unique account_code with aggregated totals
2. **Given** source data with 100 rows across 5 accounts, **When** aggregation groups by account_code and computes SUM(amount) and COUNT(*), **Then** exactly 5 summary rows are appended with correct sum and count values
3. **Given** a source_selector filtering to 50 of 100 rows and aggregation grouping by cost_center, **When** append executes, **Then** only the filtered 50 rows are aggregated before appending (not all 100)

---

### User Story 4 - Period-Filtered Source Data (Priority: P2)

As a financial analyst, I need source dataset rows to be automatically filtered by the run period according to the source dataset's temporal_mode so that appended data is temporally consistent with the working dataset.

**Why this priority**: Temporal consistency is critical for accurate period-based reporting. Without proper period filtering, appended data could include rows from wrong time periods, corrupting analysis results.

**Independent Test**: Can be tested independently by creating a run for period "2026-01", appending a source dataset with temporal_mode: period containing rows for multiple periods, and verifying only "2026-01" rows are appended.

**Acceptance Scenarios**:

1. **Given** a run for period "2026-01" and a source dataset with temporal_mode: period containing rows with _period values ["2025-12", "2026-01", "2026-02"], **When** append executes, **Then** only rows where _period = "2026-01" are appended
2. **Given** a source dataset with temporal_mode: bitemporal and an asOf date of 2026-01-01, **When** append executes, **Then** only rows valid as of 2026-01-01 are appended
3. **Given** a source dataset with temporal_mode: snapshot, **When** append executes, **Then** all rows are appended without period filtering

---

### Edge Cases

- What happens when source dataset columns are NOT a subset of working dataset columns (extra columns in source)?
  - **Expected**: Operation fails with a validation error identifying the extra columns that don't exist in the working dataset
  
- What happens when source dataset has zero rows matching the source_selector?
  - **Expected**: Append operation succeeds with zero rows appended; working dataset remains unchanged
  
- What happens when source dataset reference (dataset_id) points to a non-existent dataset?
  - **Expected**: Operation fails during planning/compilation with a "dataset not found" error
  
- What happens when aggregation is specified but group_by references columns that don't exist in the source dataset?
  - **Expected**: Operation fails during validation with "column not found in source dataset" error
  
- What happens when appending rows that would create duplicate natural_key values?
  - **Expected**: System generates unique _row_id values for all appended rows; natural_key uniqueness is not enforced across source datasets
  
- What happens when the source dataset has the same dataset_id as the working dataset (self-append)?
  - **Expected**: Out of scope per requirements - behavior undefined or produces validation error

- What happens when source_selector and aggregation are both specified?
  - **Expected**: source_selector filters rows first, then aggregation is applied only to the filtered rows

## Requirements *(mandatory)*

### Functional Requirements

- **FR-001**: System MUST load source dataset rows via MetadataStore and DataLoader using the same Resolver precedence as RuntimeJoin (Project resolver_overrides → Dataset resolver_id → system default)
- **FR-002**: System MUST apply period filtering to source data according to the source dataset's temporal_mode (period: filter by _period = run_period.identifier; bitemporal: use asOf query; snapshot: no filtering)
- **FR-003**: System MUST support optional source_selector to filter source dataset rows before appending
- **FR-004**: System MUST evaluate source_selector expression against source dataset columns (not working dataset columns)
- **FR-005**: System MUST support optional aggregation of source rows before appending via AppendAggregation structure (group_by + aggregations)
- **FR-006**: System MUST apply source_selector filter BEFORE aggregation when both are specified
- **FR-007**: System MUST validate that all columns in appended rows (either raw source columns or aggregation output columns) exist in the working dataset schema
- **FR-008**: System MUST align appended rows with working dataset schema by setting columns present in working dataset but absent from appended rows to NULL
- **FR-009**: System MUST fail with a validation error if source rows contain columns that don't exist in the working dataset
- **FR-010**: System MUST generate unique _row_id values for all appended rows
- **FR-011**: System MUST set system columns on appended rows (_row_id, _source_dataset, _operation_seq, _deleted = false)
- **FR-012**: System MUST respect dataset version pinning when specified in source DatasetRef (use pinned version if dataset_version is set, otherwise use latest active version)
- **FR-013**: Aggregation expressions MUST use aggregate functions (SUM, COUNT, AVG, MIN_AGG, MAX_AGG) and reference source dataset columns
- **FR-014**: Aggregation group_by columns MUST reference columns from the source dataset
- **FR-015**: System MUST produce an error if source dataset cannot be resolved or loaded

### Key Entities *(include if feature involves data)*

- **DatasetRef**: References a source dataset to append from, with optional version pinning (dataset_id, optional dataset_version)
- **AppendAggregation**: Configuration for aggregating source rows before appending, containing group_by columns and a list of Aggregation expressions
- **Aggregation**: Single aggregate computation with an output column name and an aggregate expression
- **Source Selector**: Boolean expression evaluated against source dataset columns to filter which rows are appended
- **System Columns**: Metadata columns set on appended rows (_row_id, _source_dataset, _operation_seq, _deleted)

## Success Criteria *(mandatory)*

### Measurable Outcomes

- **SC-001**: Source dataset rows are successfully appended to the working dataset, increasing the total row count by the number of source rows appended
- **SC-002**: When source rows have columns missing from the working dataset schema, column alignment fills those columns with NULL values in 100% of appended rows
- **SC-003**: When source rows contain extra columns not in the working dataset, the operation fails with a validation error 100% of the time before execution
- **SC-004**: When source_selector is specified, only rows matching the filter expression are appended (filtered row count matches expected count based on expression)
- **SC-005**: When aggregation is specified, appended rows contain aggregated values grouped by group_by columns, with row count equal to the number of unique group combinations
- **SC-006**: All appended rows have system columns (_row_id, _source_dataset, _operation_seq, _deleted) correctly populated
- **SC-007**: For test scenario TS-06 (append budgets to transactions), the output contains exactly 14 rows (10 transactions + 4 budgets) with all columns properly aligned
- **SC-008**: Period filtering correctly restricts appended rows to the run period according to the source dataset's temporal_mode (100% accuracy in temporal filtering)
- **SC-009**: When source_selector filters to zero rows, the operation completes successfully with zero rows appended and no errors
- **SC-010**: Resolver precedence for loading source datasets matches RuntimeJoin behavior (Project overrides → Dataset resolver → system default) in 100% of cases
