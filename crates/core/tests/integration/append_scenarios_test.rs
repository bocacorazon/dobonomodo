use dobo_core::engine::append::{execute_append, AppendExecutionContext};
use dobo_core::model::{AppendOperation, DatasetRef, Expression};
use polars::df;
use std::collections::HashSet;
use uuid::Uuid;

fn source_dataset_id() -> Uuid {
    Uuid::parse_str("550e8400-e29b-41d4-a716-446655440000").expect("uuid")
}

fn working_frame(rows: usize) -> polars::prelude::DataFrame {
    let row_ids = (0..rows)
        .map(|i| format!("working-{i}"))
        .collect::<Vec<_>>();
    let source = vec!["working".to_owned(); rows];
    let operation_seq = vec![1i64; rows];
    let deleted = vec![false; rows];
    let account_codes = vec!["4000".to_owned(); rows];
    let amounts = vec![1000i64; rows];
    let journal_ids = vec![Some("J1".to_owned()); rows];
    let descriptions = vec![Some("tx".to_owned()); rows];
    let budget_type = vec![None::<String>; rows];
    let row_count = vec![None::<i64>; rows];

    df!(
        "_row_id" => row_ids,
        "_source_dataset" => source,
        "_operation_seq" => operation_seq,
        "_deleted" => deleted,
        "account_code" => account_codes,
        "amount" => amounts,
        "journal_id" => journal_ids,
        "description" => descriptions,
        "budget_type" => budget_type,
        "row_count" => row_count
    )
    .expect("working frame")
}

fn assert_appended_system_columns(
    frame: &polars::prelude::DataFrame,
    existing_rows: usize,
    appended_rows: usize,
    expected_source_dataset: &str,
    expected_operation_seq: i64,
) {
    let row_ids = frame
        .column("_row_id")
        .expect("_row_id")
        .str()
        .expect("str");
    let source_dataset = frame
        .column("_source_dataset")
        .expect("_source_dataset")
        .str()
        .expect("str");
    let operation_seq = frame
        .column("_operation_seq")
        .expect("_operation_seq")
        .i64()
        .expect("i64");
    let deleted = frame
        .column("_deleted")
        .expect("_deleted")
        .bool()
        .expect("bool");

    let mut unique_row_ids = HashSet::new();
    for idx in existing_rows..(existing_rows + appended_rows) {
        let row_id = row_ids.get(idx).expect("row_id should be non-null");
        assert!(!row_id.is_empty());
        assert!(unique_row_ids.insert(row_id.to_owned()));
        assert_eq!(source_dataset.get(idx), Some(expected_source_dataset));
        assert_eq!(operation_seq.get(idx), Some(expected_operation_seq));
        assert_eq!(deleted.get(idx), Some(false));
    }
    assert_eq!(unique_row_ids.len(), appended_rows);
}

#[test]
fn ts01_append_budget_rows_to_transactions() {
    let working = working_frame(10);
    let source = df!(
        "account_code" => &["4000", "4100", "4200", "4300"],
        "amount" => &[10i64, 20, 30, 40],
        "budget_type" => &[Some("original"), Some("original"), Some("original"), Some("original")]
    )
    .expect("source frame");

    let op = AppendOperation {
        source: DatasetRef {
            dataset_id: source_dataset_id(),
            dataset_version: None,
        },
        source_selector: None,
        aggregation: None,
    };
    let result = execute_append(
        &working,
        &source,
        &op,
        &AppendExecutionContext {
            operation_seq: 2,
            ..Default::default()
        },
    )
    .expect("append should succeed");
    assert_eq!(result.rows_appended, 4);
    assert_eq!(result.frame.height(), 14);
    assert_appended_system_columns(&result.frame, 10, 4, &source_dataset_id().to_string(), 2);
}

#[test]
fn ts02_subset_columns_append_successfully() {
    let working = working_frame(2);
    let source = df!(
        "account_code" => &["5000"],
        "amount" => &[150i64]
    )
    .expect("source frame");
    let op = AppendOperation {
        source: DatasetRef {
            dataset_id: source_dataset_id(),
            dataset_version: None,
        },
        source_selector: None,
        aggregation: None,
    };

    let result = execute_append(
        &working,
        &source,
        &op,
        &AppendExecutionContext {
            operation_seq: 2,
            ..Default::default()
        },
    )
    .expect("append should succeed");
    assert_eq!(result.frame.height(), 3);
}

#[test]
fn ts03_missing_columns_are_filled_with_null() {
    let working = working_frame(1);
    let source = df!(
        "account_code" => &["5000"],
        "amount" => &[150i64]
    )
    .expect("source frame");
    let op = AppendOperation {
        source: DatasetRef {
            dataset_id: source_dataset_id(),
            dataset_version: None,
        },
        source_selector: None,
        aggregation: None,
    };
    let result = execute_append(
        &working,
        &source,
        &op,
        &AppendExecutionContext {
            operation_seq: 2,
            ..Default::default()
        },
    )
    .expect("append should succeed");

    let appended_idx = result.frame.height() - 1;
    let journal = result
        .frame
        .column("journal_id")
        .expect("journal_id")
        .str()
        .expect("string")
        .get(appended_idx);
    assert!(journal.is_none());
}

#[test]
fn ts06_filter_only_original_budget_rows() {
    let working = working_frame(10);
    let source = df!(
        "account_code" => &[
            "4000", "4000", "4000", "4000", "4000", "4000",
            "4000", "4000", "4000", "4000", "4000", "4000"
        ],
        "amount" => &[1i64,2,3,4,5,6,7,8,9,10,11,12],
        "budget_type" => &[
            Some("original"), Some("original"), Some("original"), Some("original"),
            Some("revised"), Some("revised"), Some("revised"), Some("revised"),
            Some("forecast"), Some("forecast"), Some("forecast"), Some("forecast")
        ]
    )
    .expect("source frame");
    let op = AppendOperation {
        source: DatasetRef {
            dataset_id: source_dataset_id(),
            dataset_version: None,
        },
        source_selector: Some(Expression::from("budget_type = 'original'")),
        aggregation: None,
    };

    let result = execute_append(
        &working,
        &source,
        &op,
        &AppendExecutionContext {
            operation_seq: 2,
            ..Default::default()
        },
    )
    .expect("append should succeed");
    assert_eq!(result.rows_appended, 4);
}

#[test]
fn ts07_filter_by_numeric_comparison() {
    let working = working_frame(0);
    let source = df!(
        "account_code" => &["4000", "4000", "4000"],
        "amount" => &[9000i64, 15000, 20000]
    )
    .expect("source frame");
    let op = AppendOperation {
        source: DatasetRef {
            dataset_id: source_dataset_id(),
            dataset_version: None,
        },
        source_selector: Some(Expression::from("amount > 10000")),
        aggregation: None,
    };
    let result = execute_append(
        &working,
        &source,
        &op,
        &AppendExecutionContext {
            operation_seq: 2,
            ..Default::default()
        },
    )
    .expect("append should succeed");
    assert_eq!(result.rows_appended, 2);
}

#[test]
fn ts08_highly_selective_filter() {
    let working = working_frame(0);
    let amounts = (0..100).collect::<Vec<i64>>();
    let source = df!(
        "account_code" => vec!["4000"; 100],
        "amount" => amounts
    )
    .expect("source frame");
    let op = AppendOperation {
        source: DatasetRef {
            dataset_id: source_dataset_id(),
            dataset_version: None,
        },
        source_selector: Some(Expression::from("amount >= 95")),
        aggregation: None,
    };
    let result = execute_append(
        &working,
        &source,
        &op,
        &AppendExecutionContext {
            operation_seq: 2,
            ..Default::default()
        },
    )
    .expect("append should succeed");
    assert_eq!(result.rows_appended, 5);
}

#[test]
fn ts13_aggregate_budget_rows_by_account_sum_amount() {
    let working = working_frame(0);
    let source = df!(
        "account_code" => &["4000","4000","5000","5000","5000","6000","6000","6000","6000","7000","7000","7000"],
        "amount" => &[1i64,2,3,4,5,6,7,8,9,10,11,12]
    )
    .expect("source frame");
    let op: AppendOperation = serde_yaml::from_str(
        r#"
source:
  dataset_id: "550e8400-e29b-41d4-a716-446655440000"
aggregation:
  group_by:
    - account_code
  aggregations:
    - column: amount
      expression: "SUM(amount)"
"#,
    )
    .expect("append operation");

    let result = execute_append(
        &working,
        &source,
        &op,
        &AppendExecutionContext {
            operation_seq: 2,
            ..Default::default()
        },
    )
    .expect("append should succeed");
    assert_eq!(result.rows_appended, 4);
}

#[test]
fn ts14_aggregate_100_rows_with_sum_and_count() {
    let working = working_frame(0);
    let source = df!(
        "account_code" => (0..100).map(|i| format!("A{}", i % 5)).collect::<Vec<_>>(),
        "amount" => (0..100).map(|i| i as i64).collect::<Vec<_>>(),
        "row_count" => vec![1i64; 100]
    )
    .expect("source frame");
    let op: AppendOperation = serde_yaml::from_str(
        r#"
source:
  dataset_id: "550e8400-e29b-41d4-a716-446655440000"
aggregation:
  group_by:
    - account_code
  aggregations:
    - column: amount
      expression: "SUM(amount)"
    - column: row_count
      expression: "COUNT(*)"
"#,
    )
    .expect("append operation");
    let result = execute_append(
        &working,
        &source,
        &op,
        &AppendExecutionContext {
            operation_seq: 2,
            ..Default::default()
        },
    )
    .expect("append should succeed");
    assert_eq!(result.rows_appended, 5);
}

#[test]
fn ts15_source_selector_filters_before_aggregation() {
    let working = working_frame(0);
    let source = df!(
        "account_code" => (0..100).map(|i| format!("A{}", i % 5)).collect::<Vec<_>>(),
        "amount" => (0..100).map(|i| i as i64).collect::<Vec<_>>()
    )
    .expect("source frame");
    let op: AppendOperation = serde_yaml::from_str(
        r#"
source:
  dataset_id: "550e8400-e29b-41d4-a716-446655440000"
source_selector: "amount >= 50"
aggregation:
  group_by:
    - account_code
  aggregations:
    - column: amount
      expression: "SUM(amount)"
"#,
    )
    .expect("append operation");
    let result = execute_append(
        &working,
        &source,
        &op,
        &AppendExecutionContext {
            operation_seq: 2,
            ..Default::default()
        },
    )
    .expect("append should succeed");
    assert_eq!(result.source_rows_after_selector, 50);
    assert_eq!(result.rows_appended, 5);
}
