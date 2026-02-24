//! Contract tests for RuntimeJoin schema validation
//!
//! These tests verify that RuntimeJoin and UpdateArguments structures
//! conform to the OpenAPI contract defined in runtime_join_schema.yaml

use dobo_core::model::{Expression, RuntimeJoin, UpdateArguments};
use serde_json::json;
use uuid::Uuid;

#[test]
fn test_runtime_join_serialization() {
    // Valid RuntimeJoin should serialize/deserialize correctly
    let join = RuntimeJoin {
        alias: "fx".to_string(),
        dataset_id: Uuid::new_v4(),
        dataset_version: Some(2),
        on: Expression {
            source: "transactions.currency = fx.from_currency".to_string(),
        },
    };

    let json_str = serde_json::to_string(&join).unwrap();
    let deserialized: RuntimeJoin = serde_json::from_str(&json_str).unwrap();

    assert_eq!(join.alias, deserialized.alias);
    assert_eq!(join.dataset_id, deserialized.dataset_id);
    assert_eq!(join.dataset_version, deserialized.dataset_version);
}

#[test]
fn test_runtime_join_optional_version() {
    // RuntimeJoin without dataset_version (uses latest)
    let json_value = json!({
        "alias": "customers",
        "dataset_id": "550e8400-e29b-41d4-a716-446655440000",
        "on": "transactions.customer_id = customers.id"
    });

    let join: RuntimeJoin = serde_json::from_value(json_value).unwrap();
    assert_eq!(join.alias, "customers");
    assert!(join.dataset_version.is_none());
}

#[test]
fn test_update_arguments_with_joins() {
    // UpdateArguments with joins array
    let json_value = json!({
        "joins": [
            {
                "alias": "fx",
                "dataset_id": "550e8400-e29b-41d4-a716-446655440000",
                "dataset_version": 3,
                "on": "gl.currency = fx.from_currency"
            }
        ],
        "assignments": [{
            "column": "amount_reporting",
            "expression": "amount_local"
        }]
    });

    let args: UpdateArguments = serde_json::from_value(json_value).unwrap();
    assert_eq!(args.joins.len(), 1);
    assert_eq!(args.joins[0].alias, "fx");
    assert_eq!(args.joins[0].dataset_version, Some(3));
}

#[test]
fn test_update_arguments_empty_joins() {
    // UpdateArguments with empty joins (default behavior)
    let json_value = json!({
        "assignments": [{
            "column": "amount_reporting",
            "expression": "amount_local"
        }]
    });

    let args: UpdateArguments = serde_json::from_value(json_value).unwrap();
    assert_eq!(args.joins.len(), 0);
}

#[test]
fn test_multiple_joins() {
    // Multiple RuntimeJoins in single operation
    let json_value = json!({
        "joins": [
            {
                "alias": "fx",
                "dataset_id": "550e8400-e29b-41d4-a716-446655440000",
                "on": "gl.currency = fx.from_currency"
            },
            {
                "alias": "customers",
                "dataset_id": "650e8400-e29b-41d4-a716-446655440000",
                "dataset_version": 5,
                "on": "gl.customer_id = customers.id"
            }
        ],
        "assignments": [{
            "column": "amount_reporting",
            "expression": "amount_local"
        }]
    });

    let args: UpdateArguments = serde_json::from_value(json_value).unwrap();
    assert_eq!(args.joins.len(), 2);
    assert_eq!(args.joins[0].alias, "fx");
    assert_eq!(args.joins[1].alias, "customers");
    assert!(args.joins[0].dataset_version.is_none());
    assert_eq!(args.joins[1].dataset_version, Some(5));
}

#[test]
fn test_alias_pattern_validation() {
    // Alias should be alphanumeric with underscores
    let valid_aliases = vec!["fx", "exchange_rates", "FX_Rates", "_internal", "fx123"];

    for alias in valid_aliases {
        let json_value = json!({
            "alias": alias,
            "dataset_id": "550e8400-e29b-41d4-a716-446655440000",
            "on": "a = b"
        });

        let result: Result<RuntimeJoin, _> = serde_json::from_value(json_value);
        assert!(result.is_ok(), "Alias '{}' should be valid", alias);
    }
}

#[test]
fn test_runtime_join_deserializes_object_expression_shape() {
    let json_value = json!({
        "alias": "fx",
        "dataset_id": "550e8400-e29b-41d4-a716-446655440000",
        "on": {
            "source": "gl.currency = fx.from_currency"
        }
    });

    let join: RuntimeJoin = serde_json::from_value(json_value).unwrap();
    assert_eq!(join.on.source, "gl.currency = fx.from_currency");
}

#[test]
fn test_update_arguments_require_assignments_field() {
    let json_value = json!({
        "joins": [{
            "alias": "fx",
            "dataset_id": "550e8400-e29b-41d4-a716-446655440000",
            "on": "gl.currency = fx.from_currency"
        }]
    });

    let error = serde_json::from_value::<UpdateArguments>(json_value)
        .expect_err("missing assignments should fail");
    assert!(error.to_string().contains("missing field `assignments`"));
}

#[test]
fn test_update_arguments_reject_empty_assignments() {
    let json_value = json!({
        "joins": [],
        "assignments": []
    });

    let error = serde_json::from_value::<UpdateArguments>(json_value)
        .expect_err("empty assignments should fail");
    assert!(error
        .to_string()
        .contains("assignments must contain at least 1 item"));
}
