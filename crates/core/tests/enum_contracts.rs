use dobo_core::model::{
    ColumnType, OperationKind, ProjectStatus, RunStatus, StrategyType, TemporalMode, TriggerType,
};

#[test]
fn enums_roundtrip_with_serde_json() {
    let run_status = RunStatus::Queued;
    let encoded = serde_json::to_string(&run_status).expect("encode should work");
    let decoded: RunStatus = serde_json::from_str(&encoded).expect("decode should work");
    assert_eq!(decoded, RunStatus::Queued);

    let project_status = ProjectStatus::Draft;
    let encoded = serde_json::to_string(&project_status).expect("encode should work");
    let decoded: ProjectStatus = serde_json::from_str(&encoded).expect("decode should work");
    assert_eq!(decoded, ProjectStatus::Draft);

    let operation_kind = OperationKind::Output;
    let encoded = serde_json::to_string(&operation_kind).expect("encode should work");
    let decoded: OperationKind = serde_json::from_str(&encoded).expect("decode should work");
    assert_eq!(decoded, OperationKind::Output);

    let temporal_mode = TemporalMode::Period;
    let encoded = serde_json::to_string(&temporal_mode).expect("encode should work");
    let decoded: TemporalMode = serde_json::from_str(&encoded).expect("decode should work");
    assert_eq!(decoded, TemporalMode::Period);

    let column_type = ColumnType::String;
    let encoded = serde_json::to_string(&column_type).expect("encode should work");
    let decoded: ColumnType = serde_json::from_str(&encoded).expect("decode should work");
    assert_eq!(decoded, ColumnType::String);

    let strategy_type = StrategyType::Path;
    let encoded = serde_json::to_string(&strategy_type).expect("encode should work");
    let decoded: StrategyType = serde_json::from_str(&encoded).expect("decode should work");
    assert_eq!(decoded, StrategyType::Path);

    let trigger_type = TriggerType::Manual;
    let encoded = serde_json::to_string(&trigger_type).expect("encode should work");
    let decoded: TriggerType = serde_json::from_str(&encoded).expect("decode should work");
    assert_eq!(decoded, TriggerType::Manual);
}
