# Quickstart: Resolver Rule Evaluation Engine

**Feature**: Resolver Rule Evaluation Engine  
**Branch**: 012-resolver-engine  
**Date**: 2026-02-22

## Overview

This guide walks you through using the Resolver Rule Evaluation Engine to resolve data locations from rules and calendar hierarchies.

---

## Prerequisites

- Rust 1.93.1+ installed
- DobONoMoDo workspace cloned
- Basic understanding of the DobONoMoDo data model (Calendar, Period, Resolver)

---

## Quick Example

```rust
use dobo_core::resolver::engine;
use dobo_core::model::{Resolver, Calendar, Period, resolver::ResolutionRequest};
use uuid::Uuid;

// 1. Create a resolution request
let request = ResolutionRequest {
    dataset_id: "sales_dataset".to_string(),
    table_name: "daily_transactions".to_string(),
    period_id: Uuid::parse_str("550e8400-e29b-41d4-a716-446655440000").unwrap(),
    project_id: None,
};

// 2. Load metadata (from database, files, or fixtures)
let resolver = load_resolver("sales_resolver_v2");
let calendar = load_calendar("fiscal_calendar");
let periods = load_periods_for_calendar(calendar.id);

// 3. Resolve locations
let result = engine::resolve(request, resolver, calendar, periods)?;

// 4. Use resolved locations
for location in result.locations {
    println!("Loading data from: {:?}", location.path);
}

// 5. Inspect diagnostics
println!("Resolver source: {:?}", result.diagnostic.resolver_source);
for rule_diag in result.diagnostic.evaluated_rules {
    println!("Rule '{}': matched={}, reason={}",
        rule_diag.rule_name, rule_diag.matched, rule_diag.reason);
}
```

---

## Step-by-Step Tutorial

### Step 1: Define a Resolver

Create a resolver YAML file with rules:

```yaml
# resolvers/sales_resolver.yaml
id: sales_resolver_v2
name: Sales Data Resolver
version: 2
status: active
rules:
  - name: post_cutover_s3
    when: period >= '2024-01'
    data_level: month
    strategy:
      type: path
      datasource_id: s3_prod
      path: /data/sales/{period_id}/{table_name}.parquet

  - name: legacy_database
    data_level: month
    strategy:
      type: table
      datasource_id: postgres_legacy
      table: "{table_name}_{period_id}"
      schema: sales_schema
```

**Explanation**:
- **Rule 1** (`post_cutover_s3`): Matches periods >= 2024-01, uses S3 storage
- **Rule 2** (`legacy_database`): No condition (catch-all), uses PostgreSQL table
- Rules are evaluated in order; first match wins

---

### Step 2: Define a Calendar

Create a calendar with hierarchy:

```yaml
# calendars/fiscal_calendar.yaml
id: 550e8400-e29b-41d4-a716-446655440001
name: Fiscal Calendar
status: active
is_default: true
levels:
  - name: year
    parent_level: null
    identifier_pattern: "^\\d{4}$"

  - name: quarter
    parent_level: year
    identifier_pattern: "^\\d{4}-Q[1-4]$"

  - name: month
    parent_level: quarter
    identifier_pattern: "^\\d{4}-(0[1-9]|1[0-2])$"
```

**Explanation**:
- Three levels: year → quarter → month
- Identifier patterns validate period naming
- `parent_level` defines hierarchy relationships

---

### Step 3: Create Periods

Periods are instances of calendar levels:

```yaml
# periods/2024_q1.yaml
- id: 550e8400-e29b-41d4-a716-446655440010
  identifier: "2024-Q1"
  name: "Q1 2024"
  calendar_id: 550e8400-e29b-41d4-a716-446655440001
  year: 2024
  sequence: 1
  start_date: "2024-01-01"
  end_date: "2024-03-31"
  status: open
  parent_id: 550e8400-e29b-41d4-a716-446655440002  # 2024 year

- id: 550e8400-e29b-41d4-a716-446655440011
  identifier: "2024-01"
  name: "January 2024"
  calendar_id: 550e8400-e29b-41d4-a716-446655440001
  year: 2024
  sequence: 1
  start_date: "2024-01-01"
  end_date: "2024-01-31"
  status: open
  parent_id: 550e8400-e29b-41d4-a716-446655440010  # 2024-Q1

# ... similarly for 2024-02, 2024-03
```

**Explanation**:
- `parent_id` links periods in the hierarchy tree
- `sequence` determines ordering within parent

---

### Step 4: Load Metadata

In your code, deserialize the YAML:

```rust
use dobo_core::model::{Resolver, Calendar, Period};
use std::fs;

fn load_resolver(name: &str) -> Resolver {
    let yaml = fs::read_to_string(format!("resolvers/{}.yaml", name)).unwrap();
    serde_yaml::from_str(&yaml).unwrap()
}

fn load_calendar(name: &str) -> Calendar {
    let yaml = fs::read_to_string(format!("calendars/{}.yaml", name)).unwrap();
    serde_yaml::from_str(&yaml).unwrap()
}

fn load_periods_for_calendar(calendar_id: Uuid) -> Vec<Period> {
    let yaml = fs::read_to_string("periods/2024_q1.yaml").unwrap();
    serde_yaml::from_str(&yaml).unwrap()
}
```

---

### Step 5: Resolve Locations

Call the resolver engine:

```rust
use dobo_core::resolver::engine;
use dobo_core::model::resolver::ResolutionRequest;

let request = ResolutionRequest {
    dataset_id: "sales_dataset".to_string(),
    table_name: "daily_transactions".to_string(),
    period_id: q1_2024_id,  // 2024-Q1 period UUID
    project_id: None,
};

let result = engine::resolve(request, resolver, calendar, periods)?;

println!("Resolved {} locations", result.locations.len());
```

**Expected Output**:
```
Resolved 3 locations
```

---

### Step 6: Inspect Results

Print resolved locations:

```rust
for location in &result.locations {
    println!("Period: {}", location.period_identifier.as_ref().unwrap());
    println!("Path: {}", location.path.as_ref().unwrap());
    println!("Resolver: {} / Rule: {}", location.resolver_id, location.rule_name);
    println!();
}
```

**Expected Output**:
```
Period: 2024-01
Path: /data/sales/2024-01/daily_transactions.parquet
Resolver: sales_resolver_v2 / Rule: post_cutover_s3

Period: 2024-02
Path: /data/sales/2024-02/daily_transactions.parquet
Resolver: sales_resolver_v2 / Rule: post_cutover_s3

Period: 2024-03
Path: /data/sales/2024-03/daily_transactions.parquet
Resolver: sales_resolver_v2 / Rule: post_cutover_s3
```

---

### Step 7: View Diagnostics

Understand why rules matched:

```rust
let diag = &result.diagnostic;

println!("Resolver source: {:?}", diag.resolver_source);
println!("Expanded periods: {:?}", diag.expanded_periods);
println!("\nRule evaluation:");

for rule_diag in &diag.evaluated_rules {
    println!("  Rule '{}': matched={}", rule_diag.rule_name, rule_diag.matched);
    println!("    Reason: {}", rule_diag.reason);
    if let Some(expr) = &rule_diag.evaluated_expression {
        println!("    Expression: {}", expr);
    }
}
```

**Expected Output**:
```
Resolver source: DatasetReference
Expanded periods: ["2024-01", "2024-02", "2024-03"]

Rule evaluation:
  Rule 'post_cutover_s3': matched=true
    Reason: when: period >= '2024-01' evaluated to true
    Expression: period >= '2024-01'
  Rule 'legacy_database': matched=false
    Reason: earlier rule already matched (rule not evaluated)
```

---

## Common Use Cases

### Use Case 1: Pre/Post Cutover Routing

**Scenario**: Route data before 2024-Q1 to legacy DB, 2024-Q1+ to S3.

**Resolver**:
```yaml
rules:
  - name: new_s3_storage
    when: period >= '2024-Q1'
    data_level: month
    strategy:
      type: path
      datasource_id: s3_prod
      path: /data/{period_id}/{table_name}.parquet

  - name: legacy_db
    data_level: month
    strategy:
      type: table
      datasource_id: postgres_legacy
      table: "{table_name}_{period_id}"
```

**Test**:
- Request period `2023-Q4` → Resolves to `postgres_legacy`
- Request period `2024-Q1` → Resolves to `s3_prod`

---

### Use Case 2: No Period Expansion

**Scenario**: Resolve aggregate table that doesn't partition by period.

**Resolver**:
```yaml
rules:
  - name: aggregate_table
    data_level: any
    strategy:
      type: table
      datasource_id: postgres_analytics
      table: "{table_name}_summary"
```

**Test**:
- Request any period → Returns 1 location (no expansion)

---

### Use Case 3: Catalog-Based Discovery

**Scenario**: Query external data catalog for location.

**Resolver**:
```yaml
rules:
  - name: catalog_lookup
    data_level: month
    strategy:
      type: catalog
      endpoint: "https://catalog.example.com/resolve"
      method: POST
      params:
        dataset: "{dataset_id}"
        table: "{table_name}"
        period: "{period_id}"
      headers:
        Authorization: "Bearer {api_token}"
```

---

## Testing

Run unit tests:

```bash
cd /workspace
cargo test --package dobo-core --test resolver_us1_first_match
cargo test --package dobo-core --test resolver_us2_period_expansion
cargo test --package dobo-core --test resolver_us3_diagnostics
```

Run contract tests:

```bash
cargo test --package dobo-core --test contracts/resolver_engine_contract
```

---

## Troubleshooting

### Error: NoMatchingRule

**Symptom**: `ResolutionError::NoMatchingRule` returned.

**Cause**: No rule's `when` condition evaluated to true.

**Solution**: Check diagnostic for each rule's evaluation reason. Add a catch-all rule (no `when` condition) at the end.

---

### Error: PeriodExpansionFailed

**Symptom**: `ResolutionError::PeriodExpansionFailed` with reason "no path to data_level".

**Cause**: Requested period level cannot expand to `data_level` (missing hierarchy link).

**Solution**: Verify calendar hierarchy includes path from requested level to `data_level`. Example: year→quarter→month hierarchy cannot expand "week" to "month".

---

### Error: TemplateRenderFailed

**Symptom**: `ResolutionError::TemplateRenderFailed` with reason "unknown token: {invalid}".

**Cause**: Template contains token not in supported set.

**Solution**: Use only valid tokens: `{period_id}`, `{period_name}`, `{table_name}`, `{dataset_id}`, `{datasource_id}`.

---

## Next Steps

- **Implement feature**: Follow `/workspace/specs/012-resolver-engine/plan.md` for implementation plan
- **Write tests**: See `/workspace/specs/012-resolver-engine/contracts/resolver-engine-api.md` for test contracts
- **Integrate with pipeline**: Call resolver engine from `engine-worker` before data loading

---

## Reference

- **Spec**: `/workspace/specs/012-resolver-engine/spec.md`
- **Plan**: `/workspace/specs/012-resolver-engine/plan.md`
- **Research**: `/workspace/specs/012-resolver-engine/research.md`
- **Data Model**: `/workspace/specs/012-resolver-engine/data-model.md`
- **Contract**: `/workspace/specs/012-resolver-engine/contracts/resolver-engine-api.md`
