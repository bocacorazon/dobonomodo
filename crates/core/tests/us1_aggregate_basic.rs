// Unit tests for User Story 1: Basic Aggregate Grouping

use polars::prelude::*;

#[test]
fn test_group_by_single_column_produces_correct_groups() {
    // Create test dataframe with 6 rows, 2 distinct account types
    let account_type = Series::new(
        "account_type".into(),
        &[
            "checking", "savings", "checking", "savings", "checking", "savings",
        ],
    );
    let amount = Series::new("amount".into(), &[100, 200, 150, 250, 120, 180]);

    let df = DataFrame::new(vec![account_type.into(), amount.into()])
        .expect("Failed to create DataFrame");

    // Group by account_type
    let grouped = df
        .clone()
        .lazy()
        .group_by([col("account_type")])
        .agg([col("amount").sum().alias("total")])
        .collect()
        .expect("Failed to group");

    // Should produce 2 groups
    assert_eq!(grouped.height(), 2);

    // Verify both groups exist
    let groups = grouped
        .column("account_type")
        .expect("Missing account_type column");
    assert!(groups
        .str()
        .expect("Not a string")
        .into_iter()
        .any(|v| v == Some("checking")));
    assert!(groups
        .str()
        .expect("Not a string")
        .into_iter()
        .any(|v| v == Some("savings")));
}

#[test]
fn test_group_by_multiple_columns_produces_correct_combinations() {
    // Create test dataframe with combinations of account_type and period
    let account_type = Series::new(
        "account_type".into(),
        &["checking", "checking", "savings", "savings"],
    );
    let period = Series::new(
        "period".into(),
        &["2024-01", "2024-02", "2024-01", "2024-02"],
    );
    let amount = Series::new("amount".into(), &[100, 150, 200, 250]);

    let df = DataFrame::new(vec![account_type.into(), period.into(), amount.into()])
        .expect("Failed to create DataFrame");

    // Group by both columns
    let grouped = df
        .clone()
        .lazy()
        .group_by([col("account_type"), col("period")])
        .agg([col("amount").sum().alias("total")])
        .collect()
        .expect("Failed to group");

    // Should produce 4 distinct groups (2 account_types × 2 periods)
    assert_eq!(grouped.height(), 4);
}
