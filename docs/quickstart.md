# Quickstart

## Delete operation examples

### Selective delete

```yaml
operations:
  - order: 1
    type: delete
    parameters:
      selector: "amount = 0"
```

### Delete all active rows

```yaml
operations:
  - order: 1
    type: delete
    parameters: {}
```

### Output visibility control

```yaml
operations:
  - order: 1
    type: output
    parameters:
      include_deleted: true
```

## Run scenario contracts

```bash
cargo test -p test-resolver delete_selective_scenario_executes
cargo test -p test-resolver delete_all_scenario_executes
cargo test -p test-resolver delete_output_visibility_scenario_executes
```

