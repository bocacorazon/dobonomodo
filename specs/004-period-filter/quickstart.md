# Quickstart: Using Period Filter

## Overview
The Period Filter is applied automatically by the Engine Worker when loading data for a Run.

## Usage

### 1. Define Dataset with Temporal Mode

```yaml
main_table:
  name: "orders"
  temporal_mode: "period"
```

### 2. Configure Run Period

When starting a run, provide the period identifier:

```bash
dobo run --project my-project --period "2024-01"
```

### 3. Engine Execution (Internal)

The engine will:
1. Load the dataset schema.
2. Identify the `temporal_mode`.
3. Resolve the period identifier "2024-01" to start/end dates.
4. Apply the appropriate filter to the `LazyFrame`.

## Verification

To verify the filter works:

1. Create a test dataset with multiple periods.
2. Create a test scenario `tests/scenarios/period-filter.yaml`.
3. Run `dobo test tests/scenarios/period-filter.yaml`.
