use dobo_core::engine::aggregation::{apply_aggregation, build_agg_expressions};
use dobo_core::model::{Aggregation, AppendAggregation};
use polars::df;

#[test]
fn build_agg_expressions_from_append_aggregation() {
    let config = AppendAggregation {
        group_by: vec!["account_code".to_owned()],
        aggregations: vec![
            Aggregation {
                column: "total".to_owned(),
                expression: "SUM(amount)".to_owned(),
            },
            Aggregation {
                column: "cnt".to_owned(),
                expression: "COUNT(*)".to_owned(),
            },
        ],
    };

    let expressions = build_agg_expressions(&config).expect("expressions should build");
    assert_eq!(expressions.len(), 2);
}

#[test]
fn apply_aggregation_groups_and_aggregates_rows() {
    let frame = df!(
        "account_code" => &["4000", "4000", "5000"],
        "amount" => &[10i64, 20, 30]
    )
    .expect("frame");

    let config = AppendAggregation {
        group_by: vec!["account_code".to_owned()],
        aggregations: vec![
            Aggregation {
                column: "total".to_owned(),
                expression: "SUM(amount)".to_owned(),
            },
            Aggregation {
                column: "cnt".to_owned(),
                expression: "COUNT(*)".to_owned(),
            },
        ],
    };

    let result = apply_aggregation(&frame, &config).expect("aggregation should succeed");
    assert_eq!(result.height(), 2);
    assert!(result.column("total").is_ok());
    assert!(result.column("cnt").is_ok());
}
