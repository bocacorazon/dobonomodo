// Integration tests for Aggregate Operation Execution

use dobo_core::engine::ops::aggregate::{
    execute_aggregate, AggregateOperation, Aggregation, ExecutionContext,
};
use dobo_core::model::expression::Expression;
use polars::prelude::*;
use uuid::Uuid;

#[test]
fn test_sum_aggregate_computes_correct_totals() {
    // Create test data
    let account_type = Series::new(
        "account_type".into(),
        &["checking", "savings", "checking", "savings"],
    );
    let amount = Series::new("amount".into(), &[100, 200, 150, 250]);

    let df = DataFrame::new(vec![account_type.into(), amount.into()])
        .expect("Failed to create DataFrame");

    // Apply SUM aggregation
    let result = df
        .clone()
        .lazy()
        .group_by([col("account_type")])
        .agg([col("amount").sum().alias("total_amount")])
        .collect()
        .expect("Failed to aggregate");

    // Verify totals: checking = 100 + 150 = 250, savings = 200 + 250 = 450
    assert_eq!(result.height(), 2);

    let totals = result.column("total_amount").expect("Missing total_amount");
    let sum_values: Vec<i32> = totals
        .i32()
        .expect("Not i32")
        .into_iter()
        .map(|v| v.unwrap())
        .collect();

    assert!(sum_values.contains(&250));
    assert!(sum_values.contains(&450));
}

#[test]
fn test_count_aggregate_computes_correct_counts() {
    // Create test data with varying group sizes
    let account_type = Series::new(
        "account_type".into(),
        &["checking", "savings", "checking", "savings", "checking"],
    );
    let transaction_id = Series::new("transaction_id".into(), &[1, 2, 3, 4, 5]);

    let df = DataFrame::new(vec![account_type.into(), transaction_id.into()])
        .expect("Failed to create DataFrame");

    // Apply COUNT aggregation
    let result = df
        .clone()
        .lazy()
        .group_by([col("account_type")])
        .agg([col("transaction_id").count().alias("count")])
        .collect()
        .expect("Failed to aggregate");

    // Verify counts: checking = 3, savings = 2
    assert_eq!(result.height(), 2);

    let counts = result.column("count").expect("Missing count");
    let count_values: Vec<u32> = counts
        .u32()
        .expect("Not u32")
        .into_iter()
        .map(|v| v.unwrap())
        .collect();

    assert!(count_values.contains(&3));
    assert!(count_values.contains(&2));
}

#[test]
fn test_original_rows_remain_unchanged() {
    // Create original dataframe
    let account_type = Series::new("account_type".into(), &["checking", "savings"]);
    let amount = Series::new("amount".into(), &[100, 200]);

    let original = DataFrame::new(vec![account_type.clone().into(), amount.clone().into()])
        .expect("Failed to create DataFrame");
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
        original.clone().lazy(),
        None,
        ExecutionContext::new(Uuid::nil(), "transactions"),
    )
    .expect("aggregate should succeed")
    .collect()
    .expect("collect should succeed");

    // Original rows should still be present
    assert_eq!(combined.height(), original_height + 2);

    // First rows should match original data
    let first_two = combined.slice(0, original_height);
    assert_eq!(
        first_two
            .column("amount")
            .unwrap()
            .i32()
            .unwrap()
            .get(0)
            .unwrap(),
        100
    );
    assert_eq!(
        first_two
            .column("amount")
            .unwrap()
            .i32()
            .unwrap()
            .get(1)
            .unwrap(),
        200
    );
}

#[test]
fn test_monthly_totals_by_account_type_scenario() {
    // TS-05 from quickstart: Monthly transaction totals by account type
    let account_type = Series::new(
        "account_type".into(),
        &[
            "checking", "checking", "savings", "savings", "checking", "savings",
        ],
    );
    let period = Series::new(
        "period".into(),
        &[
            "2024-01", "2024-01", "2024-01", "2024-01", "2024-02", "2024-02",
        ],
    );
    let amount = Series::new("amount".into(), &[100, 150, 200, 250, 120, 180]);

    let original = DataFrame::new(vec![account_type.into(), period.into(), amount.into()])
        .expect("Failed to create DataFrame");

    let spec = AggregateOperation {
        group_by: vec!["account_type".to_string(), "period".to_string()],
        aggregations: vec![Aggregation {
            column: "total_amount".to_string(),
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

    // Should have 6 original + 4 summary = 10 rows
    assert_eq!(combined.height(), 10);
}

#[test]
fn test_qualified_column_references_are_supported() {
    let account_type = Series::new(
        "account_type".into(),
        &["checking", "checking", "savings", "savings"],
    );
    let amount_local = Series::new("amount_local".into(), &[100, 50, 200, 25]);
    let journal_id = Series::new("journal_id".into(), &[1, 2, 3, 4]);
    let original = DataFrame::new(vec![
        account_type.into(),
        amount_local.into(),
        journal_id.into(),
    ])
    .expect("Failed to create DataFrame");
    let original_height = original.height();

    let spec = AggregateOperation {
        group_by: vec!["transactions.account_type".to_string()],
        aggregations: vec![
            Aggregation {
                column: "total_amount".to_string(),
                expression: Expression {
                    source: "SUM(transactions.amount_local)".to_string(),
                },
            },
            Aggregation {
                column: "txn_count".to_string(),
                expression: Expression {
                    source: "COUNT(transactions.journal_id)".to_string(),
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
    .expect("aggregate should succeed for qualified references")
    .collect()
    .expect("collect should succeed");

    assert_eq!(combined.height(), original_height + 2);
    let summary = combined.slice(original_height as i64, 2);

    let totals = summary
        .column("total_amount")
        .expect("missing total_amount")
        .cast(&DataType::Int64)
        .expect("total_amount cast");
    let totals: Vec<i64> = totals
        .i64()
        .expect("total_amount is i64")
        .into_iter()
        .flatten()
        .collect();
    assert!(totals.contains(&150));
    assert!(totals.contains(&225));

    let counts = summary
        .column("txn_count")
        .expect("missing txn_count")
        .u32()
        .expect("txn_count is u32");
    let counts: Vec<u32> = counts.into_iter().flatten().collect();
    assert!(counts.contains(&2));
}
