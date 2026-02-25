# Resolver Engine API Contract

**Feature**: Resolver Rule Evaluation Engine  
**Version**: 1.0.0  
**Date**: 2026-02-22

## Overview

This document defines the public API contract for the Resolver Rule Evaluation Engine. The resolver engine is a library function in `crates/core`, not an HTTP API, so contracts are defined as Rust function signatures and behavior specifications.

---

## Public API

### resolve()

**Module**: `crates/core/src/resolver/engine.rs`

**Signature**:
```rust
pub fn resolve(
    request: ResolutionRequest,
    resolver: Resolver,
    calendar: Calendar,
    periods: Vec<Period>,
) -> Result<ResolutionResult, ResolutionError>
```

**Description**: Evaluates resolver rules against the provided context, expands periods if needed, renders location templates, and returns resolved locations with diagnostics.

**Parameters**:
- `request`: Resolution request containing dataset, table, and period context
- `resolver`: The resolver to evaluate (selected via precedence by caller)
- `calendar`: Calendar hierarchy for period expansion
- `periods`: Pre-loaded periods for the calendar (must include requested period and children)

**Returns**:
- `Ok(ResolutionResult)`: Successful resolution with locations and diagnostics
- `Err(ResolutionError)`: Resolution failure (no matching rule, expansion failure, template error)

**Guarantees**:
1. **Deterministic**: Same inputs produce same outputs (same locations in same order)
2. **First-match**: Only the first matching rule is used
3. **Complete expansion**: All child periods at data_level are included
4. **Ordered output**: Locations sorted by period sequence
5. **Traceability**: Every result includes resolver_id and rule_name

**Error Conditions**:
- `NoMatchingRule`: No rules matched the request context
- `PeriodExpansionFailed`: Calendar hierarchy cannot expand to data_level
- `TemplateRenderFailed`: Template contains unknown or unresolvable tokens
- `InvalidExpression`: `when_expression` has syntax errors

---

## Input Types

### ResolutionRequest

```rust
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
pub struct ResolutionRequest {
    pub dataset_id: String,
    pub table_name: String,
    pub period_id: Uuid,
    pub project_id: Option<String>,
}
```

**Validation**:
- `dataset_id`: Non-empty string
- `table_name`: Non-empty string
- `period_id`: Valid UUID that exists in provided periods
- `project_id`: Optional, used for resolver precedence selection

**Example**:
```json
{
  "dataset_id": "sales_dataset",
  "table_name": "daily_transactions",
  "period_id": "550e8400-e29b-41d4-a716-446655440000",
  "project_id": "reporting_project"
}
```

---

## Output Types

### ResolutionResult

```rust
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
pub struct ResolutionResult {
    pub locations: Vec<ResolvedLocation>,
    pub diagnostic: ResolutionDiagnostic,
}
```

**Guarantees**:
- `locations` is non-empty on success
- `locations` are ordered by period sequence
- `diagnostic` always present (success or failure path)

**Example**:
```json
{
  "locations": [
    {
      "datasource_id": "s3_prod",
      "path": "/data/sales/2024-01/daily_transactions.parquet",
      "table": null,
      "schema": null,
      "period_identifier": "2024-01",
      "resolver_id": "sales_resolver_v2",
      "rule_name": "post_cutover_s3"
    },
    {
      "datasource_id": "s3_prod",
      "path": "/data/sales/2024-02/daily_transactions.parquet",
      "table": null,
      "schema": null,
      "period_identifier": "2024-02",
      "resolver_id": "sales_resolver_v2",
      "rule_name": "post_cutover_s3"
    }
  ],
  "diagnostic": {
    "resolver_id": "sales_resolver_v2",
    "resolver_source": "DatasetReference",
    "evaluated_rules": [
      {
        "rule_name": "post_cutover_s3",
        "matched": true,
        "reason": "when: period >= '2024-01' evaluated to true",
        "evaluated_expression": "period >= '2024-01'"
      }
    ],
    "outcome": "Success",
    "expanded_periods": ["2024-01", "2024-02"]
  }
}
```

---

### ResolvedLocation (Extended)

```rust
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
pub struct ResolvedLocation {
    pub datasource_id: String,
    pub path: Option<String>,
    pub table: Option<String>,
    pub schema: Option<String>,
    pub period_identifier: Option<String>,
    pub resolver_id: String,        // NEW: traceability
    pub rule_name: String,           // NEW: traceability
}
```

**Field population by strategy type**:

| Strategy | datasource_id | path | table | schema | period_identifier |
|----------|---------------|------|-------|--------|-------------------|
| Path     | ✓            | ✓    | -     | -      | ✓                 |
| Table    | ✓            | -    | ✓     | Optional | ✓               |
| Catalog  | ✓            | ✓ (endpoint) | - | -  | ✓                 |

**Note**: `resolver_id` and `rule_name` always populated for all strategies.

---

### ResolutionDiagnostic

```rust
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
pub struct ResolutionDiagnostic {
    pub resolver_id: String,
    pub resolver_source: ResolverSource,
    pub evaluated_rules: Vec<RuleDiagnostic>,
    pub outcome: DiagnosticOutcome,
    pub expanded_periods: Vec<String>,
}
```

**Guarantees**:
- `evaluated_rules` includes every rule in the resolver (in order)
- `expanded_periods` lists period identifiers in sequence order
- `outcome` accurately reflects success/failure reason

---

### RuleDiagnostic

```rust
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
pub struct RuleDiagnostic {
    pub rule_name: String,
    pub matched: bool,
    pub reason: String,
    pub evaluated_expression: Option<String>,
}
```

**Reason format examples**:
- `"condition evaluated to true"`
- `"when: period >= '2024-Q1' evaluated to true"`
- `"when: table == 'inventory' evaluated to false (table='sales')"`
- `"no when condition (unconditional match)"`
- `"earlier rule already matched (rule not evaluated)"`

---

### ResolverSource (Enum)

```rust
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
#[serde(rename_all = "snake_case")]
pub enum ResolverSource {
    ProjectOverride,
    DatasetReference,
    SystemDefault,
}
```

---

### DiagnosticOutcome (Enum)

```rust
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
#[serde(rename_all = "snake_case")]
pub enum DiagnosticOutcome {
    Success,
    NoMatchingRule,
    PeriodExpansionFailure,
    TemplateRenderError,
}
```

---

## Error Types

### ResolutionError

```rust
#[derive(Debug, thiserror::Error)]
pub enum ResolutionError {
    #[error("no rules matched: {0}")]
    NoMatchingRule(ResolutionDiagnostic),
    
    #[error("period expansion failed: {reason}")]
    PeriodExpansionFailed { reason: String, diagnostic: ResolutionDiagnostic },
    
    #[error("template render failed: {reason}")]
    TemplateRenderFailed { reason: String, diagnostic: ResolutionDiagnostic },
    
    #[error("invalid expression in rule '{rule_name}': {reason}")]
    InvalidExpression { rule_name: String, reason: String },
}
```

**Note**: All errors include diagnostic information for troubleshooting.

---

## Behavior Contracts

### 1. First-Match Semantics

**Contract**: Resolution evaluates rules in order and stops at the first match.

**Test Case**:
```rust
// Given: Resolver with 2 rules
// Rule 1: when="period < '2024-Q1'" → legacy_db
// Rule 2: no condition → new_s3
// When: Resolve for period='2023-Q4'
// Then: Only Rule 1 matches, Rule 2 not evaluated
assert_eq!(result.locations[0].datasource_id, "legacy_db");
assert_eq!(result.diagnostic.evaluated_rules[0].matched, true);
assert_eq!(result.diagnostic.evaluated_rules[1].matched, false);
assert_eq!(result.diagnostic.evaluated_rules[1].reason, "earlier rule already matched (rule not evaluated)");
```

---

### 2. Period Expansion

**Contract**: When data_level is finer than requested period level, expand to all child periods at data_level.

**Test Case**:
```rust
// Given: Calendar with year→quarter→month hierarchy
// Rule: data_level="month"
// When: Resolve for period='2024-Q1' (quarter)
// Then: Return 3 locations (Jan, Feb, Mar)
assert_eq!(result.locations.len(), 3);
assert_eq!(result.locations[0].period_identifier, Some("2024-01".to_string()));
assert_eq!(result.locations[1].period_identifier, Some("2024-02".to_string()));
assert_eq!(result.locations[2].period_identifier, Some("2024-03".to_string()));
assert_eq!(result.diagnostic.expanded_periods, vec!["2024-01", "2024-02", "2024-03"]);
```

---

### 3. No Expansion for "any" Level

**Contract**: When data_level="any", return exactly one location without expansion.

**Test Case**:
```rust
// Given: Rule with data_level="any"
// When: Resolve for period='2024-Q1'
// Then: Return 1 location for 2024-Q1 (not expanded to months)
assert_eq!(result.locations.len(), 1);
assert_eq!(result.locations[0].period_identifier, Some("2024-Q1".to_string()));
assert_eq!(result.diagnostic.expanded_periods, vec!["2024-Q1"]);
```

---

### 4. Deterministic Ordering

**Contract**: Locations are ordered by period sequence number.

**Test Case**:
```rust
// Given: Year 2024 with 12 months
// Rule: data_level="month"
// When: Resolve for period='2024' (year)
// Then: Locations ordered Jan→Feb→...→Dec
for i in 0..12 {
    assert_eq!(
        result.locations[i].period_identifier,
        Some(format!("2024-{:02}", i + 1))
    );
}
```

---

### 5. Template Token Substitution

**Contract**: Templates are rendered with context values; unknown tokens cause errors.

**Test Case (Success)**:
```rust
// Given: Path template="/data/{period_id}/{table_name}.parquet"
// Context: period_id="2024-01", table_name="sales"
// Then: Rendered path="/data/2024-01/sales.parquet"
assert_eq!(result.locations[0].path, Some("/data/2024-01/sales.parquet".to_string()));
```

**Test Case (Failure)**:
```rust
// Given: Path template="/data/{unknown_token}/file.parquet"
// Then: Error with unknown token reported
match resolve(...) {
    Err(ResolutionError::TemplateRenderFailed { reason, .. }) => {
        assert!(reason.contains("unknown_token"));
    }
    _ => panic!("expected TemplateRenderFailed"),
}
```

---

### 6. Traceability

**Contract**: Every ResolvedLocation includes resolver_id and rule_name.

**Test Case**:
```rust
// Given: Resolver id="sales_resolver_v2", rule name="s3_path"
// When: Successful resolution
// Then: All locations include metadata
for location in result.locations {
    assert_eq!(location.resolver_id, "sales_resolver_v2");
    assert_eq!(location.rule_name, "s3_path");
}
```

---

## Expression Syntax

### Supported Operators

**Comparison**:
- `==` (equality)
- `!=` (inequality)
- `<`, `>`, `<=`, `>=` (ordering, for strings and period identifiers)

**Logical**:
- `AND` (conjunction)
- `OR` (disjunction)
- `NOT` (negation)

**Literals**:
- String literals: `'value'` or `"value"`
- Boolean literals: `true`, `false`

**Context Variables**:
- `period` (period identifier as string, e.g., "2024-Q1")
- `table` (table name string)
- `dataset` (dataset ID string)

### Expression Examples

```
period >= '2024-Q1'
table == 'sales' AND period < '2024-Q3'
NOT (period == '2023-Q4')
dataset == 'prod_data' OR dataset == 'staging_data'
```

### Invalid Expressions

```
period > 100                  // Type error: period is string, not number
unknown_var == 'value'        // Unknown variable
period >= '2024-Q1' &&        // Incomplete expression
(table == 'sales'             // Unbalanced parentheses
```

---

## Template Syntax

### Supported Tokens

| Token | Source | Example Value |
|-------|--------|---------------|
| `{period_id}` | Period.identifier | `2024-01` |
| `{period_name}` | Period.name | `January 2024` |
| `{table_name}` | ResolutionRequest.table_name | `daily_transactions` |
| `{dataset_id}` | ResolutionRequest.dataset_id | `sales_dataset` |
| `{datasource_id}` | ResolutionStrategy.datasource_id | `s3_prod` |

### Template Examples

**Path strategy**:
```
/data/{dataset_id}/{period_id}/{table_name}.parquet
→ /data/sales_dataset/2024-01/daily_transactions.parquet
```

**Table strategy**:
```
table: {table_name}_{period_id}
→ daily_transactions_2024-01

schema: {dataset_id}_schema
→ sales_dataset_schema
```

**Catalog strategy**:
```
endpoint: https://catalog.example.com/datasets/{dataset_id}/tables/{table_name}?period={period_id}
→ https://catalog.example.com/datasets/sales_dataset/tables/daily_transactions?period=2024-01
```

---

## Version History

| Version | Date | Changes |
|---------|------|---------|
| 1.0.0 | 2026-02-22 | Initial contract definition |

---

## Compliance

This contract defines the behavior required to satisfy:
- **FR-001** through **FR-012**: All functional requirements
- **SC-001** through **SC-006**: All success criteria
- **US1**, **US2**, **US3**: User story acceptance scenarios
