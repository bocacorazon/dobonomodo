# Quickstart: Runtime Join Resolution

**Feature**: 006-runtime-join  
**Audience**: Developers implementing or testing RuntimeJoin functionality  
**Time**: 15-20 minutes

---

## Overview

This guide walks you through implementing and testing a RuntimeJoin in the DobONoMoDo computation engine. You'll create an update operation that joins a GL transactions dataset with a bitemporal exchange rates dataset to compute reporting currency amounts.

**What you'll learn**:
- How to define a RuntimeJoin in YAML
- How to set up the InMemoryDataLoader for testing
- How to verify period filtering works correctly for bitemporal tables
- How to reference joined columns in assignment expressions

---

## Prerequisites

- Rust toolchain (1.75+ recommended)
- DobONoMoDo repository cloned
- Familiarity with Polars LazyFrame API (helpful but not required)

---

## Step 1: Define the RuntimeJoin Structure

Create a new update operation with a RuntimeJoin to the exchange_rates dataset:

```yaml
# Example: FX conversion operation
operation:
  id: "op-fx-conversion"
  seq: 1
  name: "Compute Reporting Currency Amounts"
  type: update
  arguments:
    joins:
      - alias: fx
        dataset_id: "ds-exchange-rates-uuid"  # Replace with actual UUID
        # Omit dataset_version to use latest active
        on: "transactions.currency = fx.from_currency AND fx.to_currency = 'USD' AND fx.rate_type = 'closing'"
    assignments:
      - column: amount_reporting
        expression: "transactions.amount_local * fx.rate"
```

**Key points**:
- `alias: fx` makes joined columns available as `fx.rate`, `fx.from_currency`, etc.
- `on` expression joins on currency match and filters to USD closing rates
- `dataset_version` is omitted, so the latest active version is used at run time

---

## Step 2: Set Up Test Data

Seed the InMemoryDataLoader with sample GL transactions and exchange rates:

```rust
use dobonomodo_core::engine::io_traits::DataLoader;
use dobonomodo_test_resolver::InMemoryDataLoader;

// Create loader and seed transactions
let mut loader = InMemoryDataLoader::new();

// Transactions table (period mode)
loader.seed_table("transactions", "2026-01", vec![
    ("journal_id", "line_number", "currency", "amount_local"),
    ("JE-001", 1, "USD", 15000.00),
    ("JE-002", 1, "EUR", 8500.00),
    ("JE-003", 1, "GBP", 22000.00),
    ("JE-005", 1, "JPY", 2500000.00),
]);

// Exchange rates table (bitemporal mode)
loader.seed_bitemporal_table("exchange_rates", vec![
    ("from_currency", "to_currency", "rate", "rate_type", "_period_from", "_period_to"),
    ("EUR", "USD", 1.0850, "closing", "2025-01-01", "2026-01-01"),  // Old rate
    ("EUR", "USD", 1.0920, "closing", "2026-01-01", null),          // Current rate
    ("GBP", "USD", 1.2650, "closing", "2025-01-01", "2026-01-01"),
    ("GBP", "USD", 1.2710, "closing", "2026-01-01", null),
    ("JPY", "USD", 0.00667, "closing", "2025-01-01", "2026-01-01"),
    ("JPY", "USD", 0.00672, "closing", "2026-01-01", null),
    ("USD", "USD", 1.0000, "closing", "2020-01-01", null),
]);
```

**Period filtering logic**:
- For transactions (period mode): Filter `_period = "2026-01"`
- For exchange_rates (bitemporal mode): Filter `_period_from <= 2026-01-01 AND (_period_to IS NULL OR _period_to > 2026-01-01)`
  - This selects rates effective as of 2026-01-01 (the current rates, not the old ones)

---

## Step 3: Execute the Join Operation

Run the update operation through the engine:

```rust
use dobonomodo_core::engine::join::resolve_and_load_join;
use dobonomodo_core::model::calendar::Period;

// Define run period
let period = Period {
    identifier: "2026-01".to_string(),
    level: "month".to_string(),
    start_date: "2026-01-01".parse().unwrap(),
    end_date: "2026-01-31".parse().unwrap(),
};

// Resolve and load join
let join_lf = resolve_and_load_join(
    &runtime_join,
    &project,
    &metadata_store,
    &resolver,
    &loader,
    &period,
)?;

// Join to working dataset
let working_lf = loader.load_table("transactions", &period)?;
let joined_lf = working_lf.join(
    join_lf,
    &[col("currency")],
    &[col("from_currency")],
    JoinArgs::new(JoinType::Left).with_suffix("_fx"),
)?;

// Evaluate assignment
let result_lf = joined_lf.with_columns([
    (col("amount_local") * col("rate_fx")).alias("amount_reporting")
]);

let result_df = result_lf.collect()?;
```

---

## Step 4: Verify Results

Check that the converted amounts match the expected rates:

```rust
assert_eq!(result_df["amount_reporting"][0], 15000.00);  // USD: 15000 * 1.0
assert_eq!(result_df["amount_reporting"][1], 9282.00);   // EUR: 8500 * 1.0920
assert_eq!(result_df["amount_reporting"][2], 27962.00);  // GBP: 22000 * 1.2710
assert_eq!(result_df["amount_reporting"][3], 16800.00);  // JPY: 2500000 * 0.00672
```

**Why these values?**
- EUR uses rate 1.0920 (2026-01-01 rate), not 1.0850 (2025 rate) - bitemporal asOf filter works!
- GBP uses 1.2710, JPY uses 0.00672 - both current rates
- USD uses 1.0000 (identity rate)

---

## Step 5: Test Multiple Joins

Add a second RuntimeJoin to enrich with customer tier:

```yaml
operation:
  arguments:
    joins:
      - alias: fx
        dataset_id: "ds-exchange-rates-uuid"
        on: "transactions.currency = fx.from_currency AND fx.to_currency = 'USD'"
      - alias: customers
        dataset_id: "ds-customers-uuid"
        on: "transactions.customer_id = customers.id"
    assignments:
      - column: amount_reporting
        expression: "transactions.amount_local * fx.rate"
      - column: customer_tier
        expression: "customers.tier"
      - column: discounted_amount
        expression: "IF(customers.tier = 'gold', amount_reporting * 0.9, amount_reporting)"
```

**Key points**:
- Both `fx` and `customers` aliases are available in all assignment expressions
- Joins are applied sequentially (fx first, then customers)
- Later assignments can reference columns created by earlier assignments

---

## Common Issues & Solutions

### Issue: "Dataset not found"

**Symptom**: RuntimeJoin resolution fails with "Dataset {id} not found"

**Solution**: Ensure the dataset_id exists in MetadataStore. For tests, seed the MetadataStore with the dataset definition:
```rust
metadata_store.insert_dataset(Dataset {
    id: "ds-exchange-rates-uuid".parse().unwrap(),
    name: "Exchange Rates",
    version: 1,
    status: DatasetStatus::Active,
    // ... rest of definition
});
```

---

### Issue: Wrong exchange rate used

**Symptom**: EUR conversion uses 1.0850 instead of 1.0920

**Solution**: Check period filtering. For bitemporal tables, the asOf query must use `_period_from <= period.start_date`. Verify:
- Period.start_date is correctly set to 2026-01-01
- Exchange rates table has `_period_from` and `_period_to` columns (not `_period`)
- The 2026-01-01 rate has `_period_from = 2026-01-01` and `_period_to = null`

---

### Issue: "Unknown column 'fx.rate'"

**Symptom**: Assignment expression compilation fails

**Solution**: Check that:
- The RuntimeJoin alias is defined (`alias: fx`)
- The join dataset has a column named `rate`
- The expression compiler's symbol table includes the join alias mapping

---

## Next Steps

- **Read**: `docs/entities/operation.md` for full RuntimeJoin specification
- **Explore**: `docs/architecture/sample-datasets.md` for more test scenarios
- **Implement**: Contract tests in `crates/core/tests/contracts/runtime_join_contract.rs`
- **Test**: Integration test TS-03 in `crates/core/tests/integration/ts03_fx_conversion.rs`

---

## Reference: Complete TS-03 Test

```rust
#[test]
fn test_ts03_fx_conversion() {
    // Setup
    let mut loader = InMemoryDataLoader::new();
    seed_gl_dataset(&mut loader, "2026-01");
    seed_fx_rates(&mut loader);
    
    let period = Period::new("2026-01", "2026-01-01", "2026-01-31");
    let project = create_fx_conversion_project();
    
    // Execute
    let engine = Engine::new(loader, metadata_store, resolver);
    let run = engine.execute_project(&project, &period)?;
    
    // Verify
    assert_eq!(run.output["amount_reporting"]["JE-002"], 9282.00);  // EUR
    assert_eq!(run.output["amount_reporting"]["JE-003"], 27962.00); // GBP
    assert_eq!(run.output["amount_reporting"]["JE-005"], 16800.00); // JPY
}
```

**Success!** You've implemented and tested a RuntimeJoin that performs FX conversion using a bitemporal exchange rates table.
