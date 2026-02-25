# Append Operation JSON Schema

This schema defines the structure for the `append` operation type in JSON format.

```json
{
  "$schema": "http://json-schema.org/draft-07/schema#",
  "title": "AppendOperation",
  "description": "Operation that loads rows from a source dataset and appends them to the working dataset",
  "type": "object",
  "required": ["type", "parameters"],
  "properties": {
    "type": {
      "type": "string",
      "enum": ["append"],
      "description": "Operation type identifier"
    },
    "order": {
      "type": "integer",
      "minimum": 0,
      "description": "Execution order of this operation in the pipeline"
    },
    "alias": {
      "type": "string",
      "description": "Optional alias for referencing this operation"
    },
    "parameters": {
      "type": "object",
      "required": ["source"],
      "properties": {
        "source": {
          "$ref": "#/definitions/DatasetRef",
          "description": "Reference to the source dataset to append from"
        },
        "source_selector": {
          "type": "string",
          "description": "Optional filter expression for source rows (e.g., 'budget_type = \"original\"')",
          "examples": [
            "budget_type = 'original'",
            "amount > 10000",
            "status = 'active' AND fiscal_year = 2026"
          ]
        },
        "aggregation": {
          "$ref": "#/definitions/AppendAggregation",
          "description": "Optional aggregation to apply before appending"
        }
      },
      "additionalProperties": false
    }
  },
  "definitions": {
    "DatasetRef": {
      "type": "object",
      "required": ["dataset_id"],
      "properties": {
        "dataset_id": {
          "type": "string",
          "format": "uuid",
          "description": "UUID of the source dataset",
          "examples": ["550e8400-e29b-41d4-a716-446655440000"]
        },
        "dataset_version": {
          "type": "integer",
          "minimum": 1,
          "description": "Optional version pinning (use specific version instead of latest)",
          "examples": [1, 2, 3]
        }
      },
      "additionalProperties": false
    },
    "AppendAggregation": {
      "type": "object",
      "required": ["group_by", "aggregations"],
      "properties": {
        "group_by": {
          "type": "array",
          "items": {
            "type": "string"
          },
          "minItems": 1,
          "description": "Columns to group by (must exist in source dataset)",
          "examples": [
            ["account_code"],
            ["account_code", "cost_center_code"]
          ]
        },
        "aggregations": {
          "type": "array",
          "items": {
            "$ref": "#/definitions/Aggregation"
          },
          "minItems": 1,
          "description": "List of aggregate computations to perform"
        }
      },
      "additionalProperties": false
    },
    "Aggregation": {
      "type": "object",
      "required": ["column", "expression"],
      "properties": {
        "column": {
          "type": "string",
          "description": "Output column name for the aggregated value",
          "examples": ["total_budget", "avg_amount", "line_count"]
        },
        "expression": {
          "type": "string",
          "pattern": "^(SUM|COUNT|AVG|MIN_AGG|MAX_AGG)\\(.+\\)$",
          "description": "Aggregate function expression (e.g., 'SUM(amount)')",
          "examples": [
            "SUM(amount)",
            "COUNT(budget_id)",
            "AVG(amount_local)",
            "MIN_AGG(date_posted)",
            "MAX_AGG(amount)"
          ]
        }
      },
      "additionalProperties": false
    }
  }
}
```

## Example Instances

### Example 1: Simple Append (Budget to Transactions)

```json
{
  "type": "append",
  "order": 1,
  "parameters": {
    "source": {
      "dataset_id": "a1b2c3d4-e5f6-7890-abcd-ef1234567890"
    }
  }
}
```

**Scenario**: Append all budget rows to transaction dataset (User Story 1, TS-01)

---

### Example 2: Filtered Append

```json
{
  "type": "append",
  "order": 2,
  "alias": "append_original_budgets",
  "parameters": {
    "source": {
      "dataset_id": "b2c3d4e5-f6a7-8901-bcde-f12345678901"
    },
    "source_selector": "budget_type = 'original'"
  }
}
```

**Scenario**: Append only "original" budget rows (User Story 2, TS-06)

---

### Example 3: Aggregated Append

```json
{
  "type": "append",
  "order": 3,
  "parameters": {
    "source": {
      "dataset_id": "c3d4e5f6-a7b8-9012-cdef-123456789012",
      "dataset_version": 2
    },
    "aggregation": {
      "group_by": ["account_code", "cost_center_code"],
      "aggregations": [
        {
          "column": "total_budget",
          "expression": "SUM(amount)"
        },
        {
          "column": "budget_count",
          "expression": "COUNT(budget_id)"
        }
      ]
    }
  }
}
```

**Scenario**: Append monthly budget totals grouped by account and cost center (User Story 3, TS-13)

---

### Example 4: Filtered + Aggregated Append

```json
{
  "type": "append",
  "order": 4,
  "parameters": {
    "source": {
      "dataset_id": "d4e5f6a7-b8c9-0123-def1-234567890123"
    },
    "source_selector": "amount > 10000 AND status = 'approved'",
    "aggregation": {
      "group_by": ["department_code"],
      "aggregations": [
        {
          "column": "high_value_total",
          "expression": "SUM(amount)"
        },
        {
          "column": "high_value_count",
          "expression": "COUNT(*)"
        },
        {
          "column": "avg_high_value",
          "expression": "AVG(amount)"
        }
      ]
    }
  }
}
```

**Scenario**: Append aggregated high-value approved transactions by department (User Story 3, TS-15)

---

## Validation Rules

### Dataset Reference Validation

```javascript
// dataset_id must be valid UUID
assert(isValidUUID(parameters.source.dataset_id));

// dataset_id must exist in metadata store
assert(metadataStore.hasDataset(parameters.source.dataset_id));

// If dataset_version specified, must match
if (parameters.source.dataset_version !== undefined) {
  const dataset = metadataStore.getDataset(parameters.source.dataset_id);
  assert(dataset.version === parameters.source.dataset_version);
}
```

### Source Selector Validation

```javascript
if (parameters.source_selector !== undefined) {
  // Must be non-empty string
  assert(parameters.source_selector.trim().length > 0);
  
  // Must parse as valid expression
  assert(expressionParser.parse(parameters.source_selector).isValid());
  
  // All referenced columns must exist in source dataset
  const sourceSchema = metadataStore.getDatasetSchema(parameters.source.dataset_id);
  const referencedColumns = extractColumnReferences(parameters.source_selector);
  for (const col of referencedColumns) {
    assert(sourceSchema.hasColumn(col), `Column '${col}' not in source dataset`);
  }
}
```

### Aggregation Validation

```javascript
if (parameters.aggregation !== undefined) {
  const agg = parameters.aggregation;
  const sourceSchema = metadataStore.getDatasetSchema(parameters.source.dataset_id);
  const workingSchema = currentWorkingDatasetSchema;
  
  // group_by must have at least one column
  assert(agg.group_by.length > 0);
  
  // All group_by columns must exist in source dataset
  for (const col of agg.group_by) {
    assert(sourceSchema.hasColumn(col), `Group-by column '${col}' not in source dataset`);
  }
  
  // aggregations must have at least one entry
  assert(agg.aggregations.length > 0);
  
  // Validate each aggregation
  for (const aggregation of agg.aggregations) {
    // Output column must exist in working dataset
    assert(workingSchema.hasColumn(aggregation.column), 
           `Output column '${aggregation.column}' not in working dataset`);
    
    // Expression must be valid aggregate function
    const parsed = parseAggregateExpression(aggregation.expression);
    assert(parsed.function in ["SUM", "COUNT", "AVG", "MIN_AGG", "MAX_AGG"],
           `Invalid aggregate function: ${parsed.function}`);
    
    // Input column must exist in source dataset
    assert(sourceSchema.hasColumn(parsed.inputColumn),
           `Aggregation input column '${parsed.inputColumn}' not in source dataset`);
  }
}
```

---

## Error Responses

### Dataset Not Found

```json
{
  "error": {
    "code": "APPEND_001",
    "message": "Dataset not found",
    "details": {
      "dataset_id": "550e8400-e29b-41d4-a716-446655440000",
      "operation_order": 2
    }
  }
}
```

### Dataset Version Mismatch

```json
{
  "error": {
    "code": "APPEND_002",
    "message": "Dataset version not found",
    "details": {
      "dataset_id": "550e8400-e29b-41d4-a716-446655440000",
      "requested_version": 5,
      "actual_version": 3
    }
  }
}
```

### Column Mismatch (Extra Columns)

```json
{
  "error": {
    "code": "APPEND_003",
    "message": "Appended rows contain columns not in working dataset",
    "details": {
      "extra_columns": ["budget_type", "fiscal_year"],
      "working_columns": ["account_code", "amount", "description"]
    }
  }
}
```

### Expression Parse Error

```json
{
  "error": {
    "code": "APPEND_004",
    "message": "Failed to parse source_selector expression",
    "details": {
      "expression": "invalid syntax here",
      "parse_error": "Expected comparison operator at position 8"
    }
  }
}
```

### Invalid Aggregation Function

```json
{
  "error": {
    "code": "APPEND_005",
    "message": "Invalid aggregation function",
    "details": {
      "expression": "MEDIAN(amount)",
      "supported_functions": ["SUM", "COUNT", "AVG", "MIN_AGG", "MAX_AGG"]
    }
  }
}
```

### Column Not Found

```json
{
  "error": {
    "code": "APPEND_006",
    "message": "Column referenced in aggregation not found in source dataset",
    "details": {
      "column": "nonexistent_column",
      "context": "group_by",
      "source_columns": ["account_code", "amount", "description"]
    }
  }
}
```

---

## Success Response

```json
{
  "success": true,
  "operation": "append",
  "order": 2,
  "result": {
    "rows_appended": 142,
    "source_dataset_id": "550e8400-e29b-41d4-a716-446655440000",
    "working_dataset_rows_before": 1000,
    "working_dataset_rows_after": 1142,
    "execution_time_ms": 23,
    "aggregated": true,
    "filtered": false
  }
}
```

---

**Contract version**: 1.0.0  
**Last updated**: 2026-02-22  
**Specification**: [spec.md](../spec.md)
