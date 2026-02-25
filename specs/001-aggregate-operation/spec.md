# Feature Specification: Aggregate Operation

**Feature Branch**: `001-aggregate-operation`  
**Created**: 2026-02-22  
**Status**: Draft  
**Input**: User description: "Implement the aggregate operation type: group rows by specified columns, compute aggregate expressions, and append summary rows to the working dataset."

## User Scenarios & Testing *(mandatory)*

### User Story 1 - Append grouped summary rows (Priority: P1)

As a project author, I want to define an aggregate operation that groups rows and computes summary values so I can produce rollup records without losing detail records.

**Why this priority**: This is the core business value of the feature and the main behavior users expect from aggregate operations.

**Independent Test**: Execute a pipeline containing one aggregate operation and verify summary rows are added while existing rows remain unchanged.

**Acceptance Scenarios**:

1. **Given** a working dataset with multiple rows across account types, **When** an aggregate operation groups by account type and computes totals, **Then** one summary row is appended per distinct account type.
2. **Given** a working dataset with existing detail rows, **When** an aggregate operation runs, **Then** all original rows remain present and unmodified.

---

### User Story 2 - Produce consistent summary row shape (Priority: P2)

As a project author, I want appended summary rows to follow the working dataset schema so downstream operations can use those rows safely.

**Why this priority**: Schema consistency is required for later operations and outputs to work reliably.

**Independent Test**: Run an aggregate operation and verify summary rows contain grouped and aggregated values, while all non-produced columns are present with null values.

**Acceptance Scenarios**:

1. **Given** a dataset with columns not included in group-by or aggregations, **When** summary rows are appended, **Then** those extra columns are present on summary rows with null values.
2. **Given** a successful aggregate operation, **When** the appended rows are inspected, **Then** each row has required system metadata fields and is marked as not deleted.

---

### User Story 3 - Handle invalid aggregate definitions early (Priority: P3)

As a project author, I want invalid aggregate configurations to fail before execution so I can correct definitions without producing partial results.

**Why this priority**: Early validation prevents silent data issues and reduces rework.

**Independent Test**: Submit invalid aggregate definitions (for example, unknown group-by columns or invalid aggregate expressions) and confirm execution is blocked with explicit validation feedback.

**Acceptance Scenarios**:

1. **Given** an aggregate operation referencing an unknown group-by column, **When** the operation is validated, **Then** validation fails with a clear column reference error.
2. **Given** an aggregate operation with no aggregation entries, **When** the operation is validated, **Then** validation fails and the pipeline does not run.

### Edge Cases

- Selector filters out all candidate rows: the operation completes successfully and appends zero summary rows.
- Input rows include null values in grouped or aggregated columns: grouping and aggregate results follow defined null-handling behavior without runtime failure.
- A group-by definition contains duplicate columns: validation rejects the definition to avoid ambiguous grouping intent.
- Aggregation output column name conflicts with a required system column: validation fails with a clear naming conflict error.

## Requirements *(mandatory)*

### Functional Requirements

- **FR-001**: The system MUST support an `aggregate` operation that accepts a non-empty list of grouping columns and a non-empty list of aggregate computations.
- **FR-002**: The system MUST apply the operation to all non-deleted working rows by default, and MUST honor an optional selector filter when provided.
- **FR-003**: The system MUST create exactly one appended summary row for each distinct group in the filtered input.
- **FR-004**: The system MUST evaluate aggregate computations using supported aggregate functions and write results into explicitly named output columns.
- **FR-005**: The system MUST append summary rows to the working dataset without removing or modifying pre-existing rows.
- **FR-006**: The system MUST populate non-grouped, non-aggregated business columns on summary rows with null values.
- **FR-007**: The system MUST assign each appended summary row a unique row identifier and required system metadata values.
- **FR-008**: The system MUST set the deleted marker on appended summary rows to false.
- **FR-009**: The system MUST reject aggregate definitions that reference unknown columns, contain duplicate group-by columns, or omit required aggregation definitions.
- **FR-010**: The system MUST return explicit validation errors before execution when an aggregate definition is invalid.

### Key Entities *(include if feature involves data)*

- **Aggregate Operation Definition**: Declares selector, group-by columns, and aggregation rules used to produce summary rows.
- **Aggregation Rule**: Defines a target output column and aggregate expression applied within each group.
- **Working Dataset Row**: Existing input row participating in grouping and aggregate calculations.
- **Summary Row**: Newly appended row containing grouped key values, computed aggregate values, system metadata, and nulls for non-produced business columns.

## Assumptions

- Aggregate expressions are already parsed and type-checked by the expression validation capability before runtime execution.
- Required system metadata columns are part of the working dataset contract.
- The feature does not introduce new aggregate function names beyond the current supported set.
- When no rows match the selector, the operation is considered successful with zero appended rows.

## Dependencies

- Expression parsing and validation capability is available to verify aggregate expressions before execution.
- Working dataset lifecycle rules for deleted-row filtering are available and consistently enforced.

## Success Criteria *(mandatory)*

### Measurable Outcomes

- **SC-001**: In acceptance tests, 100% of aggregate runs preserve all original rows while appending the expected number of summary rows based on distinct groups.
- **SC-002**: In acceptance tests, 100% of summary rows contain correct aggregate values for each configured computation.
- **SC-003**: In acceptance tests, 100% of appended summary rows contain a unique row identifier and are marked as not deleted.
- **SC-004**: In acceptance tests, 100% of non-grouped and non-aggregated business columns are null on appended summary rows.
- **SC-005**: In negative validation tests, 100% of invalid aggregate definitions are rejected before execution with explicit error messages.
