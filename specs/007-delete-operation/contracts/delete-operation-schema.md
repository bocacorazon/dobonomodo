# Contract: Delete Operation YAML Schema

**Feature**: Delete Operation  
**Version**: 1.0  
**Date**: 2026-02-22

## Overview

This contract defines the YAML schema for delete operations in DobONoMoDo project definitions. The delete operation marks matching rows as logically deleted using the `_deleted` metadata flag.

## YAML Schema

### Delete Operation Structure

```yaml
# Minimal delete operation (delete all active rows)
- seq: <integer>
  type: delete

# Delete with direct selector expression
- seq: <integer>
  type: delete
  selector: <boolean_expression>

# Delete with named selector reference
- seq: <integer>
  type: delete
  selector: "{{<selector_name>}}"

# Delete with optional alias for tracing
- seq: <integer>
  type: delete
  alias: <string>
  selector: <boolean_expression>
```

### Field Specifications

| Field | Type | Required | Default | Constraints | Description |
|-------|------|----------|---------|-------------|-------------|
| `seq` | `integer` | **Yes** | N/A | `seq > 0`, unique within project | Operation sequence number for execution order |
| `type` | `string` | **Yes** | N/A | Must be `"delete"` | Operation type identifier |
| `alias` | `string` | No | `null` | Non-empty if provided | Human-readable operation identifier for tracing |
| `selector` | `string` | No | `null` | Valid boolean expression or `{{name}}` reference | Row filter expression; `null` = delete all active rows |

### Selector Expression Rules

**Direct Boolean Expression**:
```yaml
selector: "orders.status = 'cancelled'"
selector: "orders.amount = 0"
selector: "orders.created_at < '2024-01-01'"
selector: "(orders.status = 'pending') AND (orders.age_days > 90)"
```

**Named Selector Reference**:
```yaml
# In project-level selectors
selectors:
  invalid_orders: "orders.amount = 0"
  old_pending: "(orders.status = 'pending') AND (orders.age_days > 90)"

# In operation
operations:
  - seq: 5
    type: delete
    selector: "{{invalid_orders}}"
```

**No Selector (Delete All)**:
```yaml
- seq: 10
  type: delete
  # No selector field = delete all active rows
```

### Validation Rules

**Schema Validation** (parse-time):
1. `type` field MUST equal `"delete"`
2. `seq` MUST be positive integer
3. `selector` if provided MUST be non-empty string
4. No additional fields allowed beyond `seq`, `type`, `alias`, `selector`

**Semantic Validation** (pre-execution):
1. If `selector` contains `{{NAME}}`, NAME MUST exist in `project.selectors`
2. Selector expression (direct or interpolated) MUST parse as valid DSL expression
3. Selector type MUST be boolean (not arithmetic, aggregate, or string)
4. Referenced columns in selector MUST exist in dataset schema

**Execution Preconditions**:
1. Working DataFrame MUST contain `_deleted` and `_modified_at` metadata columns
2. Pipeline MUST be in valid execution state (not failed/cancelled)

## Examples

### Example 1: Delete with Direct Selector

**YAML**:
```yaml
selectors: {}

operations:
  - seq: 1
    type: update
    selector: "true"
    arguments:
      assignments:
        - column: "processed"
          expression: "true"

  - seq: 2
    type: delete
    selector: "orders.amount = 0"

  - seq: 3
    type: output
    arguments:
      destination:
        datasource_id: "ds-warehouse"
        table: "valid_orders"
```

**Behavior**:
- Operation 1: Mark all rows as processed
- Operation 2: Mark rows with `amount = 0` as deleted (`_deleted = true`)
- Operation 3: Write only non-deleted rows to output (deleted rows excluded)

**Expected Outcome**:
- Rows with `amount = 0`: `_deleted = true`, excluded from output
- Rows with `amount > 0`: `_deleted = false`, included in output

---

### Example 2: Delete with Named Selector

**YAML**:
```yaml
selectors:
  cancelled_orders: "orders.status = 'cancelled'"
  refunded_orders: "orders.refund_amount > 0"

operations:
  - seq: 1
    type: delete
    alias: "remove_cancelled"
    selector: "{{cancelled_orders}}"

  - seq: 2
    type: delete
    alias: "remove_refunded"
    selector: "{{refunded_orders}}"
```

**Behavior**:
- Operation 1: Delete rows matching `orders.status = 'cancelled'`
- Operation 2: Delete rows matching `orders.refund_amount > 0` (from remaining active rows)

**Expected Outcome**:
- All cancelled orders: `_deleted = true`
- All refunded orders: `_deleted = true`
- Other orders: `_deleted = false`

---

### Example 3: Delete All Active Rows

**YAML**:
```yaml
operations:
  - seq: 1
    type: update
    selector: "orders.region = 'EMEA'"
    arguments:
      assignments:
        - column: "region_tag"
          expression: "'EMEA_PROCESSED'"

  - seq: 2
    type: delete
    # No selector = delete all active rows

  - seq: 3
    type: output
    arguments:
      destination:
        datasource_id: "ds-archive"
        table: "archived_orders"
      include_deleted: true
```

**Behavior**:
- Operation 1: Tag EMEA orders
- Operation 2: Mark ALL active rows as deleted
- Operation 3: Write all rows (including deleted) to archive

**Expected Outcome**:
- All rows: `_deleted = true`
- Archive output: Contains all rows (because `include_deleted: true`)

---

### Example 4: Output with Delete Visibility Control

**YAML**:
```yaml
operations:
  - seq: 1
    type: delete
    selector: "orders.test_flag = true"

  # Output 1: Production export (exclude deleted)
  - seq: 2
    type: output
    alias: "production_export"
    arguments:
      destination:
        datasource_id: "ds-warehouse"
        table: "orders_prod"
      # include_deleted defaults to false

  # Output 2: Audit export (include deleted)
  - seq: 3
    type: output
    alias: "audit_export"
    arguments:
      destination:
        datasource_id: "ds-audit"
        table: "orders_audit"
      include_deleted: true
```

**Behavior**:
- Operation 1: Delete test orders
- Operation 2: Write only non-test orders to production
- Operation 3: Write all orders (including test) to audit

**Expected Outcome**:
- Production table: Contains only `_deleted = false` rows
- Audit table: Contains all rows (both `_deleted = true` and `_deleted = false`)

---

## Error Cases

### Invalid Selector Reference

**YAML**:
```yaml
selectors:
  valid_selector: "orders.status = 'active'"

operations:
  - seq: 1
    type: delete
    selector: "{{nonexistent_selector}}"
```

**Error**:
```
Validation Error: Operation 1 (delete)
  Invalid selector reference: {{nonexistent_selector}}
  Available selectors: [valid_selector]
```

---

### Non-Boolean Selector Type

**YAML**:
```yaml
operations:
  - seq: 1
    type: delete
    selector: "orders.amount"  # Returns number, not boolean
```

**Error**:
```
Validation Error: Operation 1 (delete)
  Selector must evaluate to boolean type
  Found: Float64
  Selector: "orders.amount"
```

---

### Unknown Column in Selector

**YAML**:
```yaml
operations:
  - seq: 1
    type: delete
    selector: "orders.nonexistent_column = 5"
```

**Error**:
```
Validation Error: Operation 1 (delete)
  Column not found: nonexistent_column
  Available columns: [id, status, amount, created_at]
  Selector: "orders.nonexistent_column = 5"
```

---

## JSON Schema Definition

```json
{
  "$schema": "http://json-schema.org/draft-07/schema#",
  "title": "DeleteOperation",
  "description": "Delete operation for marking rows as logically deleted",
  "type": "object",
  "required": ["seq", "type"],
  "properties": {
    "seq": {
      "type": "integer",
      "minimum": 1,
      "description": "Operation sequence number for execution order"
    },
    "type": {
      "type": "string",
      "const": "delete",
      "description": "Operation type identifier"
    },
    "alias": {
      "type": "string",
      "minLength": 1,
      "description": "Optional human-readable operation identifier"
    },
    "selector": {
      "type": "string",
      "minLength": 1,
      "description": "Row filter expression or {{name}} reference; null = delete all"
    }
  },
  "additionalProperties": false
}
```

## OpenAPI Specification (REST API)

If project CRUD operations expose delete operation configuration via REST API:

```yaml
components:
  schemas:
    DeleteOperation:
      type: object
      required:
        - seq
        - type
      properties:
        seq:
          type: integer
          minimum: 1
          example: 5
        type:
          type: string
          enum: [delete]
        alias:
          type: string
          minLength: 1
          example: "remove_invalid_records"
        selector:
          type: string
          minLength: 1
          nullable: true
          example: "orders.amount = 0"
          description: "Boolean expression or {{name}} reference; null deletes all active rows"
```

## Compatibility Notes

**Backward Compatibility**:
- Adding `selector` field to existing delete operations is non-breaking (defaults to `null` = delete all)
- Removing `selector` from operation is non-breaking (delete all behavior)

**Forward Compatibility**:
- Future versions may add optional fields (e.g., `delete_reason`, `cascade_rules`)
- Parsers SHOULD ignore unknown fields (use `additionalProperties: false` cautiously)

**Breaking Changes** (prohibited without major version bump):
- Changing `selector` semantics (e.g., inverting boolean logic)
- Removing support for `{{name}}` interpolation
- Making `selector` required (would break existing delete-all operations)

## Version History

| Version | Date | Changes |
|---------|------|---------|
| 1.0 | 2026-02-22 | Initial contract definition |

## References

- Feature Specification: `../spec.md`
- Data Model: `../data-model.md`
- Selector Syntax: (reference to DSL documentation)
- Operation Sequencing: (reference to pipeline execution documentation)
