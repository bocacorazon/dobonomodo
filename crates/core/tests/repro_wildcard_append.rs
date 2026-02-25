use dobo_core::engine::append::{execute_append, AppendExecutionContext};
use dobo_core::model::{Aggregation, AppendAggregation, AppendOperation, DatasetRef};
use polars::prelude::*;
use uuid::Uuid;

#[test]
fn test_wildcard_failure_without_check() {
    let working = df!(
        "_row_id" => &["w1"],
        "_source_dataset" => &["working"],
        "_operation_seq" => &[1i64],
        "_deleted" => &[false],
        "account_code" => &["4000"],
        "amount" => &[100i64],
        "cnt" => &[1i64]
    )
    .unwrap();

    let source = df!(
        "account_code" => &["4000"],
        "amount" => &[10i64]
    )
    .unwrap();

    let op = AppendOperation {
        source: DatasetRef {
            dataset_id: Uuid::now_v7(),
            dataset_version: None,
        },
        source_selector: None,
        aggregation: Some(AppendAggregation {
            group_by: vec!["account_code".to_owned()],
            aggregations: vec![Aggregation {
                column: "cnt".to_owned(),
                expression: "COUNT(*)".to_owned(),
            }],
        }),
    };

    // This should SUCCEED because execute_append handles the wildcard correctly
    let result = execute_append(&working, &source, &op, &AppendExecutionContext::default());
    assert!(result.is_ok(), "{:?}", result.err());
}
