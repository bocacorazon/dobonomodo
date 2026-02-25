// Contract tests for Aggregate Operation JSON Serialization/Deserialization

use dobo_core::engine::ops::aggregate::{AggregateOperation, Aggregation};
use dobo_core::model::expression::Expression;

#[test]
fn test_deserialize_single_group_by() {
    let json = r#"{
        "group_by": ["account_type"],
        "aggregations": [
            {
                "column": "total_amount",
                "expression": { "source": "SUM(amount)" }
            }
        ]
    }"#;

    let op: AggregateOperation = serde_json::from_str(json).expect("Failed to deserialize");

    assert_eq!(op.group_by, vec!["account_type"]);
    assert_eq!(op.aggregations.len(), 1);
    assert_eq!(op.aggregations[0].column, "total_amount");
    assert_eq!(op.aggregations[0].expression.source, "SUM(amount)");
}

#[test]
fn test_deserialize_multiple_group_by() {
    let json = r#"{
        "group_by": ["account_type", "period"],
        "aggregations": [
            {
                "column": "total_amount",
                "expression": { "source": "SUM(amount)" }
            },
            {
                "column": "transaction_count",
                "expression": { "source": "COUNT(transaction_id)" }
            }
        ]
    }"#;

    let op: AggregateOperation = serde_json::from_str(json).expect("Failed to deserialize");

    assert_eq!(op.group_by, vec!["account_type", "period"]);
    assert_eq!(op.aggregations.len(), 2);
    assert_eq!(op.aggregations[0].expression.source, "SUM(amount)");
    assert_eq!(
        op.aggregations[1].expression.source,
        "COUNT(transaction_id)"
    );
}

#[test]
fn test_serialize_aggregate_operation() {
    let op = AggregateOperation {
        group_by: vec!["category".to_string()],
        aggregations: vec![Aggregation {
            column: "total_value".to_string(),
            expression: Expression {
                source: "SUM(value)".to_string(),
            },
        }],
        selector: None,
    };

    let json = serde_json::to_string(&op).expect("Failed to serialize");

    // Verify it can be round-tripped
    let deserialized: AggregateOperation =
        serde_json::from_str(&json).expect("Failed to deserialize");
    assert_eq!(op, deserialized);
}

#[test]
fn test_deserialize_yaml_expression_string() {
    let yaml = r#"
group_by:
  - account_type
aggregations:
  - column: total_amount
    expression: SUM(amount)
"#;

    let op: AggregateOperation = serde_yaml::from_str(yaml).expect("Failed to deserialize");
    assert_eq!(op.group_by, vec!["account_type"]);
    assert_eq!(op.aggregations[0].column, "total_amount");
    assert_eq!(op.aggregations[0].expression.source, "SUM(amount)");
}
