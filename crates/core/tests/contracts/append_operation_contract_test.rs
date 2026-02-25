use dobo_core::model::{AppendOperation, OperationInstance, OperationKind};
use uuid::Uuid;

#[test]
fn deserialize_simple_append_operation_from_yaml() {
    let yaml = r#"
type: append
order: 2
parameters:
  source:
    dataset_id: "550e8400-e29b-41d4-a716-446655440000"
"#;
    let op: OperationInstance = serde_yaml::from_str(yaml).expect("operation should deserialize");
    assert_eq!(op.kind, OperationKind::Append);

    let append = op
        .append_parameters()
        .expect("append parameters should deserialize");
    assert_eq!(
        append.source.dataset_id,
        Uuid::parse_str("550e8400-e29b-41d4-a716-446655440000").expect("uuid")
    );
    assert!(append.source_selector.is_none());
    assert!(append.aggregation.is_none());
}

#[test]
fn deserialize_simple_append_operation_from_json() {
    let json = r#"{
      "type": "append",
      "order": 3,
      "parameters": {
        "source": { "dataset_id": "550e8400-e29b-41d4-a716-446655440000", "dataset_version": 2 }
      }
    }"#;
    let op: OperationInstance = serde_json::from_str(json).expect("operation should deserialize");
    let append = op
        .append_parameters()
        .expect("append parameters should deserialize");
    assert_eq!(append.source.dataset_version, Some(2));
}

#[test]
fn deserialize_append_operation_with_source_selector_from_yaml() {
    let yaml = r#"
source:
  dataset_id: "550e8400-e29b-41d4-a716-446655440000"
source_selector: "budget_type = 'original'"
"#;
    let append: AppendOperation =
        serde_yaml::from_str(yaml).expect("append operation should deserialize");
    assert_eq!(
        append.source_selector.as_ref().expect("selector").source,
        "budget_type = 'original'"
    );
}

#[test]
fn deserialize_append_operation_with_aggregation_from_yaml() {
    let yaml = r#"
source:
  dataset_id: "550e8400-e29b-41d4-a716-446655440000"
aggregation:
  group_by:
    - account_code
  aggregations:
    - column: total_budget
      expression: "SUM(amount)"
"#;
    let append: AppendOperation =
        serde_yaml::from_str(yaml).expect("append operation should deserialize");
    let aggregation = append.aggregation.expect("aggregation");
    assert_eq!(aggregation.group_by, vec!["account_code"]);
    assert_eq!(aggregation.aggregations.len(), 1);
}

#[test]
fn aggregation_expressions_match_expected_pattern_shape() {
    let valid = [
        "SUM(amount)",
        "COUNT(*)",
        "AVG(amount)",
        "MIN_AGG(amount)",
        "MAX_AGG(amount)",
    ];
    for expression in valid {
        let parsed = dobo_core::dsl::aggregation::parse_aggregation(expression);
        assert!(parsed.is_ok(), "expected valid expression: {expression}");
    }
}

#[test]
fn invalid_aggregation_expression_is_rejected() {
    let parsed = dobo_core::dsl::aggregation::parse_aggregation("MEDIAN(amount)");
    assert!(parsed.is_err());
    let parsed = dobo_core::dsl::aggregation::parse_aggregation("SUM(amount) junk");
    assert!(parsed.is_err());
}
