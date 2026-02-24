# Quickstart: Test Harness

**Feature**: 003-test-harness  
**Date**: 2026-02-22

## Overview

This quickstart guide demonstrates how to create and execute test scenarios using the test harness. The harness validates that the computation engine produces correct results by comparing actual output to expected output.

---

## Prerequisites

- Rust workspace built successfully (`cargo build`)
- Test harness implemented (S02 complete)
- Basic understanding of YAML syntax

---

## Step 1: Create a Simple Test Scenario

Create a file `tests/scenarios/my-first-test.yaml`:

```yaml
name: "My First Test - Passthrough"
description: "Validates that data passes through unchanged"

periods:
  - identifier: "2026-01"
    level: "month"
    start_date: "2026-01-01"
    end_date: "2026-01-31"

input:
  dataset:
    main_table:
      name: orders
      temporal_mode: period
      columns:
        - name: order_id
          type: integer
          nullable: false
        - name: amount
          type: decimal
          nullable: false
  
  data:
    orders:
      rows:
        - { order_id: 1, amount: 100.0, _period: "2026-01" }
        - { order_id: 2, amount: 200.0, _period: "2026-01" }
        - { order_id: 3, amount: 150.0, _period: "2026-01" }

project:
  name: "passthrough"
  materialization: eager
  operations:
    - order: 1
      type: output
      parameters:
        destination: default

expected_output:
  data:
    rows:
      - { order_id: 1, amount: 100.0 }
      - { order_id: 2, amount: 200.0 }
      - { order_id: 3, amount: 150.0 }

config:
  match_mode: exact
  validate_metadata: false
  validate_traceability: false
  snapshot_on_failure: true
```

**Key Points**:
- **Business columns only**: Input data rows contain `order_id` and `amount` (no `_row_id`, `_created_at`, etc.)
- **Temporal column**: Users provide `_period` column matching the period identifier
- **Expected output**: System columns are automatically stripped (unless `validate_metadata: true`)
- **Passthrough project**: Single `output` operation returns data unchanged

---

## Step 2: Run the Test

Execute the test scenario:

```bash
dobo test tests/scenarios/my-first-test.yaml
```

**Expected Output (PASS)**:
```
Test: My First Test - Passthrough
Status: PASS
Duration: 45ms

✓ All 3 expected rows found
✓ No extra rows
✓ No value mismatches
```

---

## Step 3: Test with Deliberate Failure

Modify the scenario to introduce a mismatch. Change the expected output:

```yaml
expected_output:
  data:
    rows:
      - { order_id: 1, amount: 100.0 }
      - { order_id: 2, amount: 999.0 }  # Changed from 200.0
      - { order_id: 3, amount: 150.0 }
```

Run again:

```bash
dobo test tests/scenarios/my-first-test.yaml
```

**Expected Output (FAIL)**:
```
Test: My First Test - Passthrough
Status: FAIL
Duration: 48ms

Data Mismatches (1):
  ✗ Value mismatch at row { order_id: 2 }:
      Column 'amount': expected 999.0, got 200.0

Snapshot saved to: tests/scenarios/.snapshots/my-first-test-passthrough-actual.yaml
```

---

## Step 4: Test Suite Execution

Create a second scenario `tests/scenarios/subset-test.yaml`:

```yaml
name: "Subset Match Test"
description: "Validates subset matching allows extra rows"

periods:
  - identifier: "2026-01"
    level: "month"
    start_date: "2026-01-01"
    end_date: "2026-01-31"

input:
  dataset:
    main_table:
      name: products
      temporal_mode: period
      columns:
        - name: product_id
          type: integer
          nullable: false
        - name: price
          type: decimal

  data:
    products:
      rows:
        - { product_id: 1, price: 10.0, _period: "2026-01" }
        - { product_id: 2, price: 20.0, _period: "2026-01" }
        - { product_id: 3, price: 30.0, _period: "2026-01" }

project:
  name: "passthrough"
  materialization: eager
  operations:
    - order: 1
      type: output
      parameters:
        destination: default

expected_output:
  data:
    rows:
      - { product_id: 1, price: 10.0 }
      - { product_id: 2, price: 20.0 }
      # product_id: 3 is extra, but allowed in subset mode

config:
  match_mode: subset  # Allows extra actual rows
  validate_metadata: false
  validate_traceability: false
```

Run the entire test suite:

```bash
dobo test --suite tests/scenarios
```

**Expected Output**:
```
Test Suite: tests/scenarios
Discovered: 2 scenarios
Running: 2 scenarios

✓ my-first-test.yaml (45ms)
✓ subset-test.yaml (38ms)

Results:
  Passed: 2 / 2 (100%)
  Failed: 0 / 2 (0%)
  Errors: 0 / 2 (0%)
  Duration: 83ms
```

---

## Step 5: Using External Data Files

For larger datasets, use external CSV or Parquet files:

**Create `tests/scenarios/data/orders.csv`**:
```csv
order_id,amount,_period
1,100.0,2026-01
2,200.0,2026-01
3,150.0,2026-01
4,300.0,2026-01
5,250.0,2026-01
```

**Update scenario to reference file**:
```yaml
input:
  data:
    orders:
      file: "data/orders.csv"  # Relative to scenario file
```

This keeps scenario YAML files concise while supporting large datasets.

---

## Step 6: Validating Operations (Future)

Once the engine is implemented (S10), test scenarios can validate complex operations:

```yaml
project:
  name: "discount-calculation"
  materialization: eager
  selectors:
    HIGH_VALUE: "amount > 200"
  operations:
    - order: 1
      type: update
      parameters:
        selector: "{{HIGH_VALUE}}"
        assignments:
          - column: amount
            expression: "amount * 0.9"  # 10% discount
    - order: 2
      type: output
      parameters:
        destination: default

expected_output:
  data:
    rows:
      - { order_id: 1, amount: 100.0 }  # Unchanged (≤ 200)
      - { order_id: 2, amount: 200.0 }  # Unchanged (≤ 200)
      - { order_id: 4, amount: 270.0 }  # Discounted (300 * 0.9)
      - { order_id: 5, amount: 225.0 }  # Discounted (250 * 0.9)
```

---

## Verification Gates

Before proceeding to implementation (tasks.md), verify:

1. ✅ **Constitution Compliance**: Test harness design follows TDD principle (tests test the test harness)
2. ✅ **Schema Validity**: All entities serialize to/from YAML correctly
3. ✅ **Contract Clarity**: CLI interface is well-defined and testable
4. ✅ **Dependencies**: No missing NEEDS CLARIFICATION items in technical context

---

## Next Steps

1. Generate tasks.md using `/speckit.tasks` command (Phase 2 - not part of this planning phase)
2. Implement test harness following TDD approach:
   - Write tests for metadata injection FIRST
   - Write tests for comparison engine FIRST
   - Write tests for YAML parsing FIRST
3. Use passthrough scenario from spec as integration test
4. Validate against all acceptance criteria from spec.md

---

## Common Pitfalls

**Pitfall 1**: Including system columns in input rows
- ❌ `{ order_id: 1, amount: 100.0, _row_id: "123-456", _period: "2026-01" }`
- ✅ `{ order_id: 1, amount: 100.0, _period: "2026-01" }`
- System columns (`_row_id`, `_created_at`, etc.) are auto-injected; only provide temporal columns

**Pitfall 2**: Forgetting temporal columns
- If `temporal_mode: period`, input rows MUST include `_period`
- If `temporal_mode: bitemporal`, input rows MUST include `_period_from` and `_period_to`

**Pitfall 3**: Both rows and file in DataBlock
- DataBlock must have exactly ONE of `rows` or `file`, not both

**Pitfall 4**: Expecting order-sensitive comparison by default
- Default is order-insensitive; set `order_sensitive: true` if order matters

---

## Resources

- **Full Capability Definition**: `/workspace/docs/capabilities/execute-test-scenario.md`
- **Sample Test Scenarios**: `/workspace/docs/architecture/sample-datasets.md` (TS-01 through TS-11)
- **Entity Definitions**: `/workspace/docs/entities/dataset.md`
- **CLI Contract**: `/workspace/specs/003-test-harness/contracts/cli-interface.md`
- **Data Model**: `/workspace/specs/003-test-harness/data-model.md`
