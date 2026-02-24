use dobo_core::engine::ops::aggregate::{
    add_system_metadata, execute_aggregate, validate_aggregate_spec, AggregateError,
    AggregateOperation, Aggregation, ExecutionContext,
};
use dobo_core::model::expression::Expression;
use polars::prelude::*;
use uuid::Uuid;

#[test]
fn aggregate_adds_metadata_and_nulls_non_aggregated_columns() {
    let account_type = Series::new("account_type".into(), &["checking", "savings", "checking"]);
    let amount = Series::new("amount".into(), &[100, 200, 50]);
    let note = Series::new("note".into(), &["a", "b", "c"]);
    let original = DataFrame::new(vec![account_type.into(), amount.into(), note.into()])
        .expect("dataframe should be valid");
    let original_height = original.height();

    let spec = AggregateOperation {
        group_by: vec!["account_type".to_string()],
        aggregations: vec![Aggregation {
            column: "total".to_string(),
            expression: Expression {
                source: "SUM(amount)".to_string(),
            },
        }],
        selector: None,
    };

    let combined = execute_aggregate(
        &spec,
        original.lazy(),
        None,
        ExecutionContext::new(Uuid::nil(), "transactions"),
    )
    .expect("aggregate should succeed")
    .collect()
    .expect("collect should succeed");

    let summary = combined.slice(original_height as i64, combined.height() - original_height);
    assert_eq!(summary.height(), 2);
    assert_eq!(
        summary
            .column("note")
            .expect("summary should include note")
            .null_count(),
        summary.height()
    );

    for name in [
        "_row_id",
        "_created_at",
        "_updated_at",
        "_source_dataset_id",
        "_source_table",
        "_deleted",
        "_period",
    ] {
        assert!(summary.column(name).is_ok(), "missing system column {name}");
    }

    assert!(summary
        .column("_deleted")
        .expect("_deleted should exist")
        .bool()
        .expect("_deleted should be bool")
        .into_iter()
        .all(|value| value == Some(false)));
}

#[test]
fn aggregate_supports_avg_min_agg_and_max_agg() {
    let account_type = Series::new("account_type".into(), &["checking", "checking", "savings"]);
    let amount = Series::new("amount".into(), &[10, 20, 30]);
    let original = DataFrame::new(vec![account_type.into(), amount.into()])
        .expect("dataframe should be valid");
    let original_height = original.height();

    let spec = AggregateOperation {
        group_by: vec!["account_type".to_string()],
        aggregations: vec![
            Aggregation {
                column: "avg_amount".to_string(),
                expression: Expression {
                    source: "AVG(amount)".to_string(),
                },
            },
            Aggregation {
                column: "min_amount".to_string(),
                expression: Expression {
                    source: "MIN_AGG(amount)".to_string(),
                },
            },
            Aggregation {
                column: "max_amount".to_string(),
                expression: Expression {
                    source: "MAX_AGG(amount)".to_string(),
                },
            },
        ],
        selector: None,
    };

    let combined = execute_aggregate(
        &spec,
        original.lazy(),
        None,
        ExecutionContext::new(Uuid::nil(), "transactions"),
    )
    .expect("aggregate should succeed")
    .collect()
    .expect("collect should succeed");

    let summary = combined.slice(original_height as i64, combined.height() - original_height);
    let avg_values: Vec<f64> = summary
        .column("avg_amount")
        .expect("avg_amount should exist")
        .f64()
        .expect("avg_amount should be float")
        .into_iter()
        .flatten()
        .collect();
    assert!(avg_values
        .iter()
        .any(|value| (*value - 15.0).abs() < f64::EPSILON));
    assert!(avg_values
        .iter()
        .any(|value| (*value - 30.0).abs() < f64::EPSILON));

    let min_column = summary
        .column("min_amount")
        .expect("min_amount should exist")
        .cast(&DataType::Int64)
        .expect("min_amount should cast to i64");
    let min_values: Vec<i64> = min_column
        .i64()
        .expect("min_amount should be i64")
        .into_iter()
        .flatten()
        .collect();
    assert!(min_values.contains(&10));
    assert!(min_values.contains(&30));

    let max_column = summary
        .column("max_amount")
        .expect("max_amount should exist")
        .cast(&DataType::Int64)
        .expect("max_amount should cast to i64");
    let max_values: Vec<i64> = max_column
        .i64()
        .expect("max_amount should be i64")
        .into_iter()
        .flatten()
        .collect();
    assert!(max_values.contains(&20));
    assert!(max_values.contains(&30));
}

#[test]
fn aggregate_applies_selector_before_grouping() {
    let account_type = Series::new("account_type".into(), &["checking", "checking", "savings"]);
    let amount = Series::new("amount".into(), &[50, 150, 200]);
    let original = DataFrame::new(vec![account_type.into(), amount.into()])
        .expect("dataframe should be valid");
    let original_height = original.height();

    let spec = AggregateOperation {
        group_by: vec!["account_type".to_string()],
        aggregations: vec![Aggregation {
            column: "total".to_string(),
            expression: Expression {
                source: "SUM(amount)".to_string(),
            },
        }],
        selector: None,
    };

    let combined = execute_aggregate(
        &spec,
        original.lazy(),
        Some(col("amount").gt(lit(100))),
        ExecutionContext::new(Uuid::nil(), "transactions"),
    )
    .expect("aggregate should succeed")
    .collect()
    .expect("collect should succeed");

    let summary = combined.slice(original_height as i64, combined.height() - original_height);
    let total_column = summary
        .column("total")
        .expect("total should exist")
        .cast(&DataType::Int64)
        .expect("total should cast to i64");
    let totals: Vec<i64> = total_column
        .i64()
        .expect("total should be i64")
        .into_iter()
        .flatten()
        .collect();
    assert!(totals.contains(&150));
    assert!(totals.contains(&200));
    assert!(!totals.contains(&50));
}

#[test]
fn aggregate_with_zero_row_selector_adds_no_summary_rows() {
    let account_type = Series::new("account_type".into(), &["checking", "savings"]);
    let amount = Series::new("amount".into(), &[10, 20]);
    let original = DataFrame::new(vec![account_type.into(), amount.into()])
        .expect("dataframe should be valid");
    let original_height = original.height();

    let spec = AggregateOperation {
        group_by: vec!["account_type".to_string()],
        aggregations: vec![Aggregation {
            column: "total".to_string(),
            expression: Expression {
                source: "SUM(amount)".to_string(),
            },
        }],
        selector: None,
    };

    let combined = execute_aggregate(
        &spec,
        original.lazy(),
        Some(col("amount").gt(lit(1_000))),
        ExecutionContext::new(Uuid::nil(), "transactions"),
    )
    .expect("aggregate should succeed")
    .collect()
    .expect("collect should succeed");

    assert_eq!(combined.height(), original_height);
    assert_eq!(
        combined
            .column("total")
            .expect("total should exist")
            .null_count(),
        original_height
    );
}

#[test]
fn aggregate_handles_null_group_and_aggregation_values() {
    let group = StringChunked::from_iter_options(
        "account_type".into(),
        vec![Some("checking"), None, None].into_iter(),
    )
    .into_series();
    let amount = Int32Chunked::from_iter_options(
        "amount".into(),
        vec![Some(10), Some(20), None].into_iter(),
    )
    .into_series();
    let original = DataFrame::new(vec![group.into(), amount.into()]).expect("df should be valid");
    let original_height = original.height();

    let spec = AggregateOperation {
        group_by: vec!["account_type".to_string()],
        aggregations: vec![Aggregation {
            column: "total".to_string(),
            expression: Expression {
                source: "SUM(amount)".to_string(),
            },
        }],
        selector: None,
    };

    let combined = execute_aggregate(
        &spec,
        original.lazy(),
        None,
        ExecutionContext::new(Uuid::nil(), "transactions"),
    )
    .expect("aggregate should succeed")
    .collect()
    .expect("collect should succeed");

    let summary = combined.slice(original_height as i64, combined.height() - original_height);
    let groups = summary
        .column("account_type")
        .expect("group column should exist")
        .str()
        .expect("group column should be utf8");
    let totals = summary
        .column("total")
        .expect("total should exist")
        .cast(&DataType::Int64)
        .expect("total should cast to i64");
    let totals = totals.i64().expect("total should be i64");

    let mut found_null_group = false;
    for row_idx in 0..summary.height() {
        if groups.get(row_idx).is_none() {
            found_null_group = true;
            assert_eq!(totals.get(row_idx), Some(20));
        }
    }
    assert!(
        found_null_group,
        "expected a summary row for null group values"
    );
}

#[test]
fn aggregate_invalid_definition_returns_explicit_error() {
    let account_type = Series::new("account_type".into(), &["checking", "savings"]);
    let amount = Series::new("amount".into(), &[10, 20]);
    let original = DataFrame::new(vec![account_type.into(), amount.into()])
        .expect("dataframe should be valid");

    let spec = AggregateOperation {
        group_by: vec!["account_type".to_string()],
        aggregations: vec![Aggregation {
            column: "total".to_string(),
            expression: Expression {
                source: "SUM(missing_column)".to_string(),
            },
        }],
        selector: None,
    };

    let err = match execute_aggregate(
        &spec,
        original.lazy(),
        None,
        ExecutionContext::new(Uuid::nil(), "transactions"),
    ) {
        Ok(_) => panic!("invalid aggregation should fail"),
        Err(err) => err,
    };

    assert_eq!(
        err,
        AggregateError::UnknownAggregationColumn("missing_column".to_string())
    );
}

#[test]
fn aggregate_spec_rejects_empty_group_by() {
    let err = validate_aggregate_spec(&AggregateOperation {
        group_by: vec![],
        aggregations: vec![Aggregation {
            column: "total".to_string(),
            expression: Expression {
                source: "SUM(amount)".to_string(),
            },
        }],
        selector: None,
    })
    .expect_err("empty group_by should fail");
    assert_eq!(err, AggregateError::EmptyGroupBy);
}

#[test]
fn aggregate_spec_rejects_empty_aggregations() {
    let err = validate_aggregate_spec(&AggregateOperation {
        group_by: vec!["account_type".to_string()],
        aggregations: vec![],
        selector: None,
    })
    .expect_err("empty aggregations should fail");
    assert_eq!(err, AggregateError::EmptyAggregations);
}

#[test]
fn aggregate_spec_rejects_whitespace_group_by_column() {
    let err = validate_aggregate_spec(&AggregateOperation {
        group_by: vec!["  ".to_string()],
        aggregations: vec![Aggregation {
            column: "total".to_string(),
            expression: Expression {
                source: "SUM(amount)".to_string(),
            },
        }],
        selector: None,
    })
    .expect_err("whitespace-only group_by should fail");
    assert_eq!(
        err,
        AggregateError::InvalidIdentifier("group_by column".to_string())
    );
}

#[test]
fn aggregate_spec_rejects_whitespace_aggregation_output_column() {
    let err = validate_aggregate_spec(&AggregateOperation {
        group_by: vec!["account_type".to_string()],
        aggregations: vec![Aggregation {
            column: "   ".to_string(),
            expression: Expression {
                source: "SUM(amount)".to_string(),
            },
        }],
        selector: None,
    })
    .expect_err("whitespace-only output column should fail");
    assert_eq!(
        err,
        AggregateError::InvalidIdentifier("aggregation output column".to_string())
    );
}

#[test]
fn aggregate_spec_rejects_duplicate_group_by_columns() {
    let err = validate_aggregate_spec(&AggregateOperation {
        group_by: vec!["account_type".to_string(), "account_type".to_string()],
        aggregations: vec![Aggregation {
            column: "total".to_string(),
            expression: Expression {
                source: "SUM(amount)".to_string(),
            },
        }],
        selector: None,
    })
    .expect_err("duplicate group_by should fail");
    assert_eq!(
        err,
        AggregateError::DuplicateGroupByColumn("account_type".to_string())
    );
}

#[test]
fn aggregate_spec_rejects_system_column_output() {
    let err = validate_aggregate_spec(&AggregateOperation {
        group_by: vec!["account_type".to_string()],
        aggregations: vec![Aggregation {
            column: "_row_id".to_string(),
            expression: Expression {
                source: "SUM(amount)".to_string(),
            },
        }],
        selector: None,
    })
    .expect_err("system output column should fail");
    assert_eq!(
        err,
        AggregateError::SystemColumnConflict("_row_id".to_string())
    );
}

#[test]
fn aggregate_spec_rejects_duplicate_output_columns() {
    let err = validate_aggregate_spec(&AggregateOperation {
        group_by: vec!["account_type".to_string()],
        aggregations: vec![
            Aggregation {
                column: "total".to_string(),
                expression: Expression {
                    source: "SUM(amount)".to_string(),
                },
            },
            Aggregation {
                column: "total".to_string(),
                expression: Expression {
                    source: "COUNT(*)".to_string(),
                },
            },
        ],
        selector: None,
    })
    .expect_err("duplicate output columns should fail");
    assert_eq!(
        err,
        AggregateError::DuplicateAggregationColumn("total".to_string())
    );
}

#[test]
fn aggregate_rejects_invalid_expression_and_context() {
    let frame = DataFrame::new(vec![
        Series::new("account_type".into(), &["checking"]).into(),
        Series::new("amount".into(), &[10]).into(),
    ])
    .expect("dataframe should be valid");

    let invalid_expression = AggregateOperation {
        group_by: vec!["account_type".to_string()],
        aggregations: vec![Aggregation {
            column: "total".to_string(),
            expression: Expression {
                source: "NOT_A_FUNCTION(amount)".to_string(),
            },
        }],
        selector: None,
    };
    let invalid_error = match execute_aggregate(
        &invalid_expression,
        frame.clone().lazy(),
        None,
        ExecutionContext::new(Uuid::nil(), "transactions"),
    ) {
        Ok(_) => panic!("unknown aggregate function should fail"),
        Err(err) => err,
    };
    assert_eq!(
        invalid_error,
        AggregateError::InvalidExpression("NOT_A_FUNCTION(amount)".to_string())
    );

    let invalid_context = AggregateOperation {
        group_by: vec!["account_type".to_string()],
        aggregations: vec![Aggregation {
            column: "total".to_string(),
            expression: Expression {
                source: "SUM(amount) + 1".to_string(),
            },
        }],
        selector: None,
    };
    let context_error = match execute_aggregate(
        &invalid_context,
        frame.lazy(),
        None,
        ExecutionContext::new(Uuid::nil(), "transactions"),
    ) {
        Ok(_) => panic!("aggregate function in invalid context should fail"),
        Err(err) => err,
    };
    assert_eq!(
        context_error,
        AggregateError::InvalidAggregateContext("SUM".to_string())
    );
}

#[test]
fn aggregate_rejects_unknown_group_by_column() {
    let frame = DataFrame::new(vec![Series::new("amount".into(), &[10]).into()])
        .expect("dataframe should be valid");
    let err = match execute_aggregate(
        &AggregateOperation {
            group_by: vec!["missing_group".to_string()],
            aggregations: vec![Aggregation {
                column: "total".to_string(),
                expression: Expression {
                    source: "SUM(amount)".to_string(),
                },
            }],
            selector: None,
        },
        frame.lazy(),
        None,
        ExecutionContext::new(Uuid::nil(), "transactions"),
    ) {
        Ok(_) => panic!("unknown group_by should fail"),
        Err(err) => err,
    };
    assert_eq!(
        err,
        AggregateError::UnknownGroupByColumn("missing_group".to_string())
    );
}

#[test]
fn aggregate_metadata_overwrites_existing_system_columns() {
    let summary = DataFrame::new(vec![
        Series::new("_row_id".into(), &["stale-row-id"]).into(),
        Series::new("_created_at".into(), &["stale-created"]).into(),
        Series::new("_updated_at".into(), &["stale-updated"]).into(),
        Series::new("_source_dataset_id".into(), &["stale-dataset"]).into(),
        Series::new("_source_table".into(), &["stale-table"]).into(),
        Series::new("_deleted".into(), &[true]).into(),
    ])
    .expect("summary should be valid");
    let context = ExecutionContext::new(Uuid::from_u128(42), "transactions");

    let updated = add_system_metadata(summary, &context).expect("metadata should be injected");
    assert_ne!(
        updated
            .column("_row_id")
            .expect("row id")
            .str()
            .expect("row id should be string")
            .get(0),
        Some("stale-row-id")
    );
    assert_eq!(
        updated
            .column("_source_table")
            .expect("source table")
            .str()
            .expect("source table should be string")
            .get(0),
        Some("transactions")
    );
    assert_eq!(
        updated
            .column("_deleted")
            .expect("deleted")
            .bool()
            .expect("deleted should be bool")
            .get(0),
        Some(false)
    );
}
