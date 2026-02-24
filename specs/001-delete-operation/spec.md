# Feature Specification: Delete Operation

**Feature Branch**: `001-delete-operation`  
**Created**: 2026-02-22  
**Status**: Draft  
**Input**: User description: "Implement the delete operation type: apply selector-based row filtering and set _deleted = true on matching rows. Verify automatic exclusion of deleted rows from all subsequent operations."

## User Scenarios & Testing *(mandatory)*

### User Story 1 - Soft-delete matching rows (Priority: P1)

As a pipeline author, I want to mark rows that match a business rule as deleted so they no longer affect downstream calculations.

**Why this priority**: This is the core feature value and enables controlled data cleanup without destructive data loss.

**Independent Test**: Execute a pipeline with a delete step using a selector and verify matching rows are marked deleted while non-matching rows remain unchanged.

**Acceptance Scenarios**:

1. **Given** a working dataset with rows that both match and do not match a selector, **When** the delete step runs, **Then** only matching rows are marked as deleted.
2. **Given** rows were marked as deleted by a delete step, **When** the next pipeline step runs, **Then** those deleted rows are not included in that step's input.

---

### User Story 2 - Delete all active rows when no selector is provided (Priority: P2)

As a pipeline author, I want the delete step to support a "delete all active rows" behavior when no selector is provided.

**Why this priority**: It supports reset and purge workflows and removes the need for artificial always-true selector expressions.

**Independent Test**: Execute a pipeline with a delete step that has no selector and verify all currently active rows are marked deleted.

**Acceptance Scenarios**:

1. **Given** a working dataset containing active rows, **When** the delete step runs without a selector, **Then** all active rows are marked as deleted.
2. **Given** some rows are already deleted before execution, **When** the delete step runs without a selector, **Then** those rows remain deleted and are not modified again.

---

### User Story 3 - Respect deleted-row visibility rules in outputs (Priority: P3)

As a pipeline consumer, I want default outputs to exclude deleted rows so published results contain only active records unless explicitly requested otherwise.

**Why this priority**: It prevents accidental exposure of logically deleted data and keeps downstream consumers aligned with business expectations.

**Independent Test**: Execute a pipeline with delete followed by output and verify deleted rows are excluded by default and can be included only when explicitly requested by output settings.

**Acceptance Scenarios**:

1. **Given** a pipeline where rows were marked deleted earlier, **When** output runs with default settings, **Then** deleted rows are not written.
2. **Given** a pipeline where rows were marked deleted earlier, **When** output runs with explicit include-deleted behavior enabled, **Then** deleted rows are included in the written result.

### Edge Cases

- Selector matches zero rows: no rows are marked deleted, and downstream behavior remains unchanged.
- Selector matches every active row: all active rows are marked deleted, and downstream operations receive zero active rows.
- Dataset already contains deleted rows before delete execution: delete processing does not reactivate or duplicate deletion state.
- Selector references unknown fields or is invalid: pipeline validation fails with a clear error and no partial deletion occurs.

## Requirements *(mandatory)*

### Functional Requirements

- **FR-001**: The system MUST support a delete operation that marks matching rows as deleted using the row metadata flag.
- **FR-002**: The delete operation MUST determine matching rows using the operation selector when provided.
- **FR-003**: If a delete operation is executed without a selector, the system MUST apply the delete operation to all currently active rows.
- **FR-004**: The system MUST update row modification metadata for each row newly marked as deleted.
- **FR-005**: Rows marked deleted by a delete operation MUST be automatically excluded from all subsequent non-output operations in the same pipeline execution.
- **FR-006**: Outputs MUST exclude deleted rows by default.
- **FR-007**: The system MUST allow outputs to include deleted rows only when output settings explicitly request inclusion.
- **FR-008**: Non-matching rows MUST remain unchanged after delete execution.
- **FR-009**: Invalid delete selectors MUST be rejected during validation before execution begins.

### Key Entities *(include if feature involves data)*

- **Working Row**: A row in the pipeline working dataset containing business columns and system metadata such as deletion status and modification timestamp.
- **Delete Operation**: A pipeline step that applies row-level logical deletion rules without physically removing data.
- **Selector**: A boolean condition that defines which active rows are targeted by a delete operation.
- **Output View**: The final row set presented to consumers, which excludes deleted rows by default and may include them only by explicit request.

## Dependencies

- Existing selector evaluation behavior used by pipeline operations.
- Existing pipeline sequencing rules where each operation consumes the prior operation output.
- Existing row metadata lifecycle rules for deletion flag and modification timestamp handling.

## Assumptions

- Soft deletion is the only supported deletion mode for this feature; physical row removal is out of scope.
- Deleted rows stay deleted for the duration of a run unless a future feature explicitly defines undeletion.
- Existing output behavior already supports an explicit include-deleted option.

## Success Criteria *(mandatory)*

### Measurable Outcomes

- **SC-001**: In acceptance testing, 100% of rows matching a delete selector are marked deleted, and 0% of non-matching rows are marked deleted.
- **SC-002**: In acceptance testing, 100% of rows marked deleted are excluded from every subsequent non-output operation in the same run.
- **SC-003**: In acceptance testing, default outputs contain 0 rows marked deleted, and explicit include-deleted output behavior returns all requested deleted rows.
- **SC-004**: In repeated scenario execution (minimum 20 runs), delete-related outcomes remain consistent with no run-to-run variance in row inclusion results.
