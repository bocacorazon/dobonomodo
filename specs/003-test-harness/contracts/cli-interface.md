# Test Harness CLI Contract

**Feature**: 003-test-harness  
**Date**: 2026-02-22  
**Type**: CLI Interface Specification

## Overview

This document defines the CLI interface contract for the test harness. The harness is invoked through the `dobo test` command with two primary modes: single-scenario execution and test suite execution.

---

## Command: dobo test (single scenario)

**Synopsis**:
```bash
dobo test <scenario.yaml> [OPTIONS]
```

**Description**:
Execute a single test scenario YAML file.

**Arguments**:
- `<scenario.yaml>`: Path to the test scenario YAML file (required)

**Options**:
- `--verbose, -v`: Enable verbose output (show all mismatches, not just summary)
- `--no-snapshot`: Disable snapshot_on_failure (override scenario config)
- `--output <format>`: Output format: `human` (default), `json`, `junit`

**Exit Codes**:
- `0`: Test passed
- `1`: Test failed (data or trace mismatches)
- `2`: Test error (parse error, execution failure, file not found)

**Output Format (human - pass)**:
```
Test: Passthrough Test
Status: PASS
Duration: 123ms

 All 2 expected rows found
 No extra rows
 No value mismatches
```

**Output Format (human - fail)**:
```
Test: Discount Calculation
Status: FAIL
Duration: 234ms

Data Mismatches (3):
  ✗ Missing row: { order_number: "ORD-001", customer_id: "C1", amount: 90.00, region: "EMEA" }
  ✗ Extra row: { order_number: "ORD-999", customer_id: "C9", amount: 500.00, region: "NA" }
  ✗ Value mismatch at row { order_number: "ORD-002" }:
      Column 'amount': expected 200.00, got 180.00

Snapshot saved to: tests/scenarios/.snapshots/discount-calculation-actual.yaml
```

---

## Command: dobo test --suite (suite execution)

**Synopsis**:
```bash
dobo test --suite <directory> [OPTIONS]
```

**Description**:
Execute all test scenarios discovered in a directory (recursively).

**Exit Codes**:
- `0`: All tests passed
- `1`: One or more tests failed
- `2`: One or more tests errored

**Output Format (human)**:
```
Test Suite: tests/scenarios
Discovered: 12 scenarios
Running: 12 scenarios

 passthrough.yaml (45ms)
 discount-calculation.yaml (123ms)
 invalid-project.yaml (12ms) - FAIL
 malformed.yaml (3ms) - ERROR

Results:
  Passed: 10 / 12 (83%)
  Failed: 1 / 12 (8%)
  Errors: 1 / 12 (8%)
  Duration: 1.2s
```

---

## File Conventions

**Scenario Discovery**:
- Default search path: `tests/scenarios/`
- Recursive search: `**/*.yaml` or `**/*.yml`
- Ignored patterns: `.*` (hidden files), `_*` (underscore-prefixed)

**Snapshot Output**:
- Location: `tests/scenarios/.snapshots/`
- Naming: `<scenario-name>-actual.yaml` (kebab-case from scenario name)
- Format: DataBlock YAML (inline rows)
