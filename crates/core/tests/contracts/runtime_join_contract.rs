use std::fs;
use std::path::PathBuf;

use serde_json::{json, Map, Value};
use uuid::Uuid;

fn contract_path() -> PathBuf {
    PathBuf::from(env!("CARGO_MANIFEST_DIR"))
        .join("..")
        .join("..")
        .join("specs/006-runtime-join/contracts/runtime_join_schema.yaml")
}

fn schema_component(name: &str) -> Value {
    let content = fs::read_to_string(contract_path()).expect("contract file should exist");
    let document: Value = serde_yaml::from_str(&content).expect("contract yaml should parse");
    document
        .pointer(&format!("/components/schemas/{name}"))
        .cloned()
        .unwrap_or_else(|| panic!("schema component '{name}' should exist"))
}

fn as_object<'a>(value: &'a Value, context: &str) -> &'a Map<String, Value> {
    value
        .as_object()
        .unwrap_or_else(|| panic!("{context} should be an object"))
}

fn validate_required_fields(
    payload: &Map<String, Value>,
    schema: &Map<String, Value>,
) -> Result<(), String> {
    let required = schema
        .get("required")
        .and_then(Value::as_array)
        .ok_or_else(|| "schema missing required array".to_string())?;
    for field in required {
        let name = field
            .as_str()
            .ok_or_else(|| "required field name should be a string".to_string())?;
        if !payload.contains_key(name) {
            return Err(format!("missing required field '{name}'"));
        }
    }
    Ok(())
}

fn validate_no_additional_properties(
    payload: &Map<String, Value>,
    schema: &Map<String, Value>,
) -> Result<(), String> {
    let additional_properties = schema
        .get("additionalProperties")
        .and_then(Value::as_bool)
        .unwrap_or(true);
    if additional_properties {
        return Ok(());
    }
    let properties = schema
        .get("properties")
        .and_then(Value::as_object)
        .ok_or_else(|| "schema missing properties object".to_string())?;
    for key in payload.keys() {
        if !properties.contains_key(key) {
            return Err(format!("unexpected additional property '{key}'"));
        }
    }
    Ok(())
}

fn validate_assignment(payload: &Value, schema: &Value) -> Result<(), String> {
    let payload = as_object(payload, "assignment payload");
    let schema = as_object(schema, "assignment schema");
    validate_required_fields(payload, schema)?;
    validate_no_additional_properties(payload, schema)?;

    for field in ["column", "expression"] {
        let value = payload
            .get(field)
            .and_then(Value::as_str)
            .ok_or_else(|| format!("'{field}' must be a string"))?;
        if value.trim().is_empty() {
            return Err(format!("'{field}' must be non-empty"));
        }
    }
    Ok(())
}

fn validate_runtime_join(payload: &Value, schema: &Value) -> Result<(), String> {
    let payload = as_object(payload, "runtime join payload");
    let schema = as_object(schema, "runtime join schema");
    validate_required_fields(payload, schema)?;
    validate_no_additional_properties(payload, schema)?;

    let alias = payload
        .get("alias")
        .and_then(Value::as_str)
        .ok_or_else(|| "'alias' must be a string".to_string())?;
    let alias_chars = alias.chars().collect::<Vec<_>>();
    if alias_chars.is_empty()
        || !(alias_chars[0].is_ascii_alphabetic() || alias_chars[0] == '_')
        || !alias_chars[1..]
            .iter()
            .all(|ch| ch.is_ascii_alphanumeric() || *ch == '_')
    {
        return Err("alias must match ^[a-zA-Z_][a-zA-Z0-9_]*$".to_string());
    }
    if alias.len() > 64 {
        return Err("alias must be <= 64 characters".to_string());
    }

    let dataset_id = payload
        .get("dataset_id")
        .and_then(Value::as_str)
        .ok_or_else(|| "'dataset_id' must be a string".to_string())?;
    Uuid::parse_str(dataset_id).map_err(|_| "dataset_id must be a valid UUID".to_string())?;

    if let Some(dataset_version) = payload.get("dataset_version") {
        let version = dataset_version
            .as_i64()
            .ok_or_else(|| "'dataset_version' must be an integer".to_string())?;
        if version < 1 {
            return Err("dataset_version must be >= 1".to_string());
        }
    }

    let on = payload
        .get("on")
        .and_then(Value::as_str)
        .ok_or_else(|| "'on' must be a string".to_string())?;
    if on.trim().is_empty() {
        return Err("'on' must be non-empty".to_string());
    }

    Ok(())
}

fn validate_update_arguments(
    payload: &Value,
    update_arguments_schema: &Value,
    runtime_join_schema: &Value,
    assignment_schema: &Value,
) -> Result<(), String> {
    let payload = as_object(payload, "update arguments payload");
    let schema = as_object(update_arguments_schema, "update arguments schema");
    validate_required_fields(payload, schema)?;
    validate_no_additional_properties(payload, schema)?;

    let assignments = payload
        .get("assignments")
        .and_then(Value::as_array)
        .ok_or_else(|| "'assignments' must be an array".to_string())?;
    if assignments.is_empty() {
        return Err("assignments must contain at least 1 item".to_string());
    }
    for assignment in assignments {
        validate_assignment(assignment, assignment_schema)?;
    }

    if let Some(joins) = payload.get("joins") {
        let joins = joins
            .as_array()
            .ok_or_else(|| "'joins' must be an array when provided".to_string())?;
        for join in joins {
            validate_runtime_join(join, runtime_join_schema)?;
        }
    }

    Ok(())
}

#[test]
fn runtime_join_schema_accepts_valid_payload() {
    let runtime_join_schema = schema_component("RuntimeJoin");
    let payload = json!({
        "alias": "fx_rates",
        "dataset_id": "550e8400-e29b-41d4-a716-446655440000",
        "dataset_version": 3,
        "on": "gl.currency = fx_rates.from_currency"
    });

    validate_runtime_join(&payload, &runtime_join_schema).expect("payload should satisfy schema");
}

#[test]
fn runtime_join_schema_rejects_invalid_alias_and_extra_property() {
    let runtime_join_schema = schema_component("RuntimeJoin");
    let payload = json!({
        "alias": "9fx",
        "dataset_id": "550e8400-e29b-41d4-a716-446655440000",
        "on": "gl.currency = fx.from_currency",
        "unexpected": true
    });

    let error = validate_runtime_join(&payload, &runtime_join_schema)
        .expect_err("invalid alias and extra property should fail schema validation");
    assert!(
        error.contains("alias must match") || error.contains("additional property"),
        "unexpected validation error: {error}"
    );
}

#[test]
fn update_arguments_schema_enforces_assignments_min_items_and_nested_shapes() {
    let runtime_join_schema = schema_component("RuntimeJoin");
    let assignment_schema = schema_component("Assignment");
    let update_arguments_schema = schema_component("UpdateArguments");

    let invalid_payload = json!({
        "joins": [{
            "alias": "fx",
            "dataset_id": "550e8400-e29b-41d4-a716-446655440000",
            "on": "gl.currency = fx.from_currency"
        }],
        "assignments": []
    });
    let invalid_error = validate_update_arguments(
        &invalid_payload,
        &update_arguments_schema,
        &runtime_join_schema,
        &assignment_schema,
    )
    .expect_err("empty assignments should violate minItems");
    assert!(
        invalid_error.contains("at least 1"),
        "unexpected validation error: {invalid_error}"
    );

    let valid_payload = json!({
        "joins": [{
            "alias": "fx",
            "dataset_id": "550e8400-e29b-41d4-a716-446655440000",
            "on": "gl.currency = fx.from_currency"
        }],
        "assignments": [{
            "column": "amount_reporting",
            "expression": "amount_local * fx.rate"
        }]
    });
    validate_update_arguments(
        &valid_payload,
        &update_arguments_schema,
        &runtime_join_schema,
        &assignment_schema,
    )
    .expect("valid payload should satisfy nested schema constraints");
}
