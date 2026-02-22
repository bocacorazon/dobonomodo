# Sample Datasets — Financial / Accounting Domain

**Created**: 2026-02-22  
**Purpose**: Reference data used across all test scenarios during development

---

## Dataset: GL Transactions (`ds-gl`)

The primary Dataset used by most test scenarios. Represents general ledger journal entries joined to chart of accounts and cost centers, with a bitemporal exchange rates lookup.

```yaml
dataset:
  id: "ds-gl"
  name: "GL Transactions"
  version: 1
  status: active
  resolver_id: "test-resolver"
  natural_key_columns: [journal_id, line_number]
  main_table:
    name: transactions
    temporal_mode: period
    columns:
      - { name: journal_id, type: string, nullable: false, description: "Journal entry identifier" }
      - { name: line_number, type: integer, nullable: false, description: "Line within the journal entry" }
      - { name: posting_date, type: date, nullable: false }
      - { name: account_code, type: string, nullable: false, description: "FK to accounts.code" }
      - { name: cost_center_code, type: string, nullable: true, description: "FK to cost_centers.code" }
      - { name: currency, type: string, nullable: false, description: "ISO currency code" }
      - { name: amount_local, type: decimal, nullable: false, description: "Amount in local currency" }
      - { name: amount_reporting, type: decimal, nullable: true, description: "Amount in reporting currency (computed)" }
      - { name: description, type: string, nullable: true }
      - { name: source_system, type: string, nullable: false, description: "Originating system" }
  lookups:
    - alias: accounts
      target:
        type: table
        name: accounts
        temporal_mode: period
        columns:
          - { name: code, type: string, nullable: false }
          - { name: name, type: string, nullable: false }
          - { name: type, type: string, nullable: false, description: "asset|liability|equity|revenue|expense" }
          - { name: category, type: string, nullable: false, description: "e.g., operating, financing, investing" }
          - { name: is_active, type: boolean, nullable: false }
      join_conditions:
        - { source_column: account_code, target_column: code }
    - alias: cost_centers
      target:
        type: table
        name: cost_centers
        temporal_mode: period
        columns:
          - { name: code, type: string, nullable: false }
          - { name: name, type: string, nullable: false }
          - { name: department, type: string, nullable: false }
          - { name: region, type: string, nullable: false, description: "EMEA|APAC|AMER" }
          - { name: is_active, type: boolean, nullable: false }
      join_conditions:
        - { source_column: cost_center_code, target_column: code }
    - alias: fx_rates
      target:
        type: table
        name: exchange_rates
        temporal_mode: bitemporal
        columns:
          - { name: from_currency, type: string, nullable: false }
          - { name: to_currency, type: string, nullable: false }
          - { name: rate, type: decimal, nullable: false }
          - { name: rate_type, type: string, nullable: false, description: "spot|average|closing" }
      join_conditions:
        - { source_column: currency, target_column: from_currency }
```

## Dataset: Budgets (`ds-budgets`)

Used for `append` operation testing — appending budget vs. actual comparisons.

```yaml
dataset:
  id: "ds-budgets"
  name: "Budget Line Items"
  version: 1
  status: active
  resolver_id: "test-resolver"
  natural_key_columns: [budget_id, line_number]
  main_table:
    name: budgets
    temporal_mode: period
    columns:
      - { name: budget_id, type: string, nullable: false }
      - { name: line_number, type: integer, nullable: false }
      - { name: account_code, type: string, nullable: false }
      - { name: cost_center_code, type: string, nullable: false }
      - { name: currency, type: string, nullable: false }
      - { name: amount, type: decimal, nullable: false }
      - { name: budget_type, type: string, nullable: false, description: "original|revised|forecast" }
```

---

## Sample Periods

```yaml
periods:
  - { identifier: "2026-01", level: "month", start_date: "2026-01-01", end_date: "2026-01-31" }
  - { identifier: "2026-02", level: "month", start_date: "2026-02-01", end_date: "2026-02-28" }
  - { identifier: "2026-03", level: "month", start_date: "2026-03-01", end_date: "2026-03-31" }
  - { identifier: "2026-Q1", level: "quarter", start_date: "2026-01-01", end_date: "2026-03-31" }
  - { identifier: "FY2026", level: "year", start_date: "2026-01-01", end_date: "2026-12-31" }
```

---

## Sample Data: `transactions` (period: 2026-01)

| journal_id | line_number | posting_date | account_code | cost_center_code | currency | amount_local | amount_reporting | description | source_system | _period |
|---|---|---|---|---|---|---|---|---|---|---|
| JE-001 | 1 | 2026-01-05 | 4100 | CC-100 | USD | 15000.00 | 15000.00 | Product revenue | ERP | 2026-01 |
| JE-001 | 2 | 2026-01-05 | 1200 | CC-100 | USD | 15000.00 | 15000.00 | AR - product revenue | ERP | 2026-01 |
| JE-002 | 1 | 2026-01-10 | 5100 | CC-200 | EUR | 8500.00 | null | Office supplies | EXPENSE | 2026-01 |
| JE-002 | 2 | 2026-01-10 | 2100 | CC-200 | EUR | -8500.00 | null | AP - office supplies | EXPENSE | 2026-01 |
| JE-003 | 1 | 2026-01-15 | 4200 | CC-300 | GBP | 22000.00 | null | Consulting revenue | ERP | 2026-01 |
| JE-003 | 2 | 2026-01-15 | 1200 | CC-300 | GBP | 22000.00 | null | AR - consulting | ERP | 2026-01 |
| JE-004 | 1 | 2026-01-20 | 5200 | CC-100 | USD | 3200.00 | 3200.00 | Travel expense | EXPENSE | 2026-01 |
| JE-004 | 2 | 2026-01-20 | 2100 | CC-100 | USD | -3200.00 | -3200.00 | AP - travel | EXPENSE | 2026-01 |
| JE-005 | 1 | 2026-01-25 | 4100 | CC-200 | JPY | 2500000.00 | null | Product revenue JP | ERP | 2026-01 |
| JE-005 | 2 | 2026-01-25 | 1200 | CC-200 | JPY | 2500000.00 | null | AR - product JP | ERP | 2026-01 |

## Sample Data: `accounts`

| code | name | type | category | is_active | _period |
|---|---|---|---|---|---|
| 1200 | Accounts Receivable | asset | operating | true | 2026-01 |
| 2100 | Accounts Payable | liability | operating | true | 2026-01 |
| 4100 | Product Revenue | revenue | operating | true | 2026-01 |
| 4200 | Consulting Revenue | revenue | operating | true | 2026-01 |
| 5100 | Office Supplies | expense | operating | true | 2026-01 |
| 5200 | Travel & Entertainment | expense | operating | true | 2026-01 |
| 6100 | Depreciation | expense | non-operating | true | 2026-01 |

## Sample Data: `cost_centers`

| code | name | department | region | is_active | _period |
|---|---|---|---|---|---|
| CC-100 | Sales NA | Sales | AMER | true | 2026-01 |
| CC-200 | Operations EU | Operations | EMEA | true | 2026-01 |
| CC-300 | Consulting UK | Consulting | EMEA | true | 2026-01 |
| CC-400 | Engineering | Engineering | AMER | true | 2026-01 |

## Sample Data: `exchange_rates` (bitemporal)

| from_currency | to_currency | rate | rate_type | _period_from | _period_to |
|---|---|---|---|---|---|
| EUR | USD | 1.0850 | closing | 2025-01-01 | 2026-01-01 |
| EUR | USD | 1.0920 | closing | 2026-01-01 | null |
| GBP | USD | 1.2650 | closing | 2025-01-01 | 2026-01-01 |
| GBP | USD | 1.2710 | closing | 2026-01-01 | null |
| JPY | USD | 0.00667 | closing | 2025-01-01 | 2026-01-01 |
| JPY | USD | 0.00672 | closing | 2026-01-01 | null |
| USD | USD | 1.0000 | closing | 2020-01-01 | null |
| EUR | USD | 1.0800 | average | 2025-01-01 | 2026-01-01 |
| EUR | USD | 1.0900 | average | 2026-01-01 | null |

## Sample Data: `budgets` (period: 2026-01)

| budget_id | line_number | account_code | cost_center_code | currency | amount | budget_type | _period |
|---|---|---|---|---|---|---|---|
| BUD-001 | 1 | 4100 | CC-100 | USD | 20000.00 | original | 2026-01 |
| BUD-001 | 2 | 5100 | CC-200 | EUR | 10000.00 | original | 2026-01 |
| BUD-001 | 3 | 4200 | CC-300 | GBP | 25000.00 | original | 2026-01 |
| BUD-001 | 4 | 5200 | CC-100 | USD | 5000.00 | original | 2026-01 |

---

## Test Scenario Catalogue

Each scenario below maps to one or more specs. They illustrate the expected behaviour using the sample data above.

### TS-01: Period Filtering (S03)

**Goal**: Load `transactions` for period 2026-01, verify only January rows are returned.

```yaml
name: "Period filter - exact match"
periods:
  - { identifier: "2026-01", level: "month", start_date: "2026-01-01", end_date: "2026-01-31" }
input:
  dataset: # ... ds-gl schema ...
  data:
    transactions:
      rows: # all 10 rows above (all have _period: "2026-01")
    accounts:
      rows: # all 7 rows
    cost_centers:
      rows: # all 4 rows
    exchange_rates:
      rows: # all 9 rows
project:
  name: "passthrough"
  materialization: eager
  operations:
    - { order: 1, type: output, parameters: { destination: default } }
expected_output:
  data:
    rows: # all 10 transaction rows, unchanged
config:
  match_mode: exact
  validate_metadata: false
```

### TS-02: Bitemporal AsOf Filtering (S03)

**Goal**: Load `exchange_rates` for period starting 2026-01-01, verify the asOf query returns rates valid as of that date.

**Expected**: EUR/USD closing = 1.0920 (not 1.0850), GBP/USD closing = 1.2710, JPY/USD = 0.00672, USD/USD = 1.0000.

### TS-03: Update Operation — FX Conversion (S04 + S05)

**Goal**: Compute `amount_reporting` by joining `exchange_rates` and multiplying `amount_local * fx_rates.rate`.

```yaml
name: "FX conversion via RuntimeJoin"
# ... periods, input as TS-01 ...
project:
  name: "FX Conversion"
  materialization: eager
  operations:
    - order: 1
      type: update
      parameters:
        joins:
          - alias: fx
            dataset_id: "ds-gl"   # self-reference for lookup table
            on: "transactions.currency = fx.from_currency AND fx.to_currency = \"USD\" AND fx.rate_type = \"closing\""
        assignments:
          - column: amount_reporting
            expression: "transactions.amount_local * fx.rate"
    - { order: 2, type: output, parameters: { destination: default } }
expected_output:
  data:
    rows:
      - { journal_id: "JE-001", line_number: 1, amount_reporting: 15000.00 }    # USD * 1.0
      - { journal_id: "JE-002", line_number: 1, amount_reporting: 9282.00 }     # 8500 * 1.0920
      - { journal_id: "JE-003", line_number: 1, amount_reporting: 27962.00 }    # 22000 * 1.2710
      - { journal_id: "JE-005", line_number: 1, amount_reporting: 16800.00 }    # 2500000 * 0.00672
      # ... all 10 rows with computed amount_reporting
config:
  match_mode: exact
  validate_metadata: false
```

### TS-04: Delete Operation — Remove Inactive (S06)

**Goal**: Soft-delete all transaction lines where the linked account `is_active = false`.

```yaml
name: "Soft delete inactive account lines"
# ... add an inactive account (code 6100) and a transaction referencing it ...
project:
  name: "Remove Inactive"
  materialization: eager
  operations:
    - order: 1
      type: update
      parameters:
        joins:
          - alias: accts
            dataset_id: "ds-gl"
            on: "transactions.account_code = accts.code"
        assignments:
          - column: _account_active
            expression: "accts.is_active"
    - order: 2
      type: delete
      selector: "_account_active = false"
    - { order: 3, type: output, parameters: { destination: default } }
expected_output:
  data:
    rows: # all rows except those referencing inactive accounts
config:
  match_mode: exact
```

### TS-05: Aggregate Operation — Monthly Totals by Account Type (S07)

**Goal**: Group transactions by `accounts.type` and compute `SUM(amount_local)`, appending summary rows.

```yaml
name: "Monthly totals by account type"
project:
  name: "Account Type Totals"
  materialization: eager
  operations:
    - order: 1
      type: update
      parameters:
        joins:
          - alias: accts
            dataset_id: "ds-gl"
            on: "transactions.account_code = accts.code"
        assignments:
          - { column: account_type, expression: "accts.type" }
    - order: 2
      type: aggregate
      parameters:
        group_by: ["account_type"]
        aggregations:
          - { column: total_local, expression: "SUM(transactions.amount_local)" }
          - { column: line_count, expression: "COUNT(transactions.journal_id)" }
    - { order: 3, type: output, parameters: { destination: default } }
expected_output:
  data:
    rows:
      # original 10 rows + 4 summary rows (asset, liability, revenue, expense)
      - { account_type: "revenue", total_local: 39500000.00, line_count: 3 }  # illustrative
      - { account_type: "expense", total_local: 11700.00, line_count: 2 }
      # ...
config:
  match_mode: subset   # we only check the summary rows
```

### TS-06: Append Operation — Budget vs Actual (S08)

**Goal**: Append budget rows into the working dataset alongside actual transactions for comparison.

```yaml
name: "Append budgets for comparison"
project:
  name: "Budget vs Actual"
  materialization: eager
  operations:
    - order: 1
      type: append
      parameters:
        source:
          dataset_id: "ds-budgets"
    - { order: 2, type: output, parameters: { destination: default } }
expected_output:
  data:
    rows: # 10 transaction rows + 4 budget rows = 14 total
config:
  match_mode: exact
```

### TS-07: Output with Column Projection (S09)

**Goal**: Output only `journal_id`, `account_code`, `amount_local`, `amount_reporting` columns.

```yaml
name: "Output column projection"
project:
  name: "Projected Output"
  materialization: eager
  operations:
    - order: 1
      type: output
      parameters:
        destination: default
        columns: [journal_id, account_code, amount_local, amount_reporting]
expected_output:
  data:
    rows:
      - { journal_id: "JE-001", account_code: "4100", amount_local: 15000.00, amount_reporting: 15000.00 }
      # ... only 4 columns per row
config:
  match_mode: exact
```

### TS-08: Named Selector with Interpolation (S04)

**Goal**: Use a named selector `{{EMEA_ONLY}}` to update only EMEA cost center rows.

```yaml
name: "Named selector interpolation"
project:
  name: "EMEA Markup"
  materialization: eager
  selectors:
    EMEA_ONLY: "cost_centers.region = \"EMEA\""
  operations:
    - order: 1
      type: update
      selector: "{{EMEA_ONLY}}"
      parameters:
        joins:
          - alias: cc
            dataset_id: "ds-gl"
            on: "transactions.cost_center_code = cc.code"
        assignments:
          - { column: amount_local, expression: "transactions.amount_local * 1.05" }
    - { order: 2, type: output, parameters: { destination: default } }
expected_output:
  data:
    rows:
      # CC-200 and CC-300 rows have amount_local * 1.05; CC-100 unchanged
config:
  match_mode: exact
```

### TS-09: Trace Validation (S12)

**Goal**: Verify that the update operation produces the expected trace events.

```yaml
name: "Trace events for FX update"
# ... same as TS-03 but with traceability on ...
expected_trace:
  - { operation_order: 1, change_type: updated, row_match: { journal_id: "JE-002", line_number: 1 }, expected_diff: { amount_reporting: { old: null, new: 9282.00 } } }
  - { operation_order: 1, change_type: updated, row_match: { journal_id: "JE-003", line_number: 1 }, expected_diff: { amount_reporting: { old: null, new: 27962.00 } } }
config:
  validate_traceability: true
```

### TS-10: Resolver Rule Evaluation (S11)

**Goal**: Given a Resolver with rules for pre-2025 CSV and post-2025 Parquet, verify correct rule matching and path template rendering for various periods.

*(This scenario tests the Resolver engine in isolation, not through the full pipeline.)*

### TS-11: Activation Validation Failures (S13)

**Goal**: Attempt to activate a Project with invalid expressions, unresolved column references, and missing selectors — verify all `ValidationFailure`s are reported.

*(This scenario tests validation logic, not execution.)*
