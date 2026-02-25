use chrono::{TimeZone, Utc};
use dobo_core::engine::ops::{execute_update, Assignment, UpdateExecutionContext, UpdateOperation};
use polars::prelude::{col, df, AnyValue, DataFrame, DataType, IntoLazy, LazyFrame, TimeUnit};
use std::collections::HashMap;

fn base_frame() -> LazyFrame {
    df![
        "id" => [1i64, 2, 3],
        "status" => ["active", "active", "inactive"],
        "amount" => [100.0f64, 200.0, 300.0],
        "count" => [1i64, 2, 3],
        "_updated_at" => [10i64, 20, 30],
    ]
    .expect("base dataframe")
    .lazy()
}

fn context(selectors: HashMap<String, String>) -> UpdateExecutionContext {
    context_with_frame(base_frame(), selectors)
}

fn context_with_frame(
    working_dataset: LazyFrame,
    selectors: HashMap<String, String>,
) -> UpdateExecutionContext {
    UpdateExecutionContext {
        working_dataset,
        selectors,
        run_timestamp: Utc
            .timestamp_opt(1_700_000_000, 0)
            .single()
            .expect("timestamp"),
    }
}

fn values_as_strings(df: &DataFrame, column: &str) -> Vec<String> {
    let series = df.column(column).expect("column").as_materialized_series();
    (0..series.len())
        .map(|index| {
            let value = series.get(index).expect("value");
            match value {
                AnyValue::String(v) => v.to_string(),
                AnyValue::StringOwned(v) => v.to_string(),
                AnyValue::Null => "null".to_string(),
                other => other.to_string(),
            }
        })
        .collect()
}

fn values_as_i64(df: &DataFrame, column: &str) -> Vec<i64> {
    let series = df.column(column).expect("column").as_materialized_series();
    (0..series.len())
        .map(|index| match series.get(index).expect("value") {
            AnyValue::Int8(v) => i64::from(v),
            AnyValue::Int16(v) => i64::from(v),
            AnyValue::Int64(v) => v,
            AnyValue::Int32(v) => i64::from(v),
            AnyValue::UInt8(v) => i64::from(v),
            AnyValue::UInt16(v) => i64::from(v),
            AnyValue::UInt64(v) => v as i64,
            AnyValue::UInt32(v) => i64::from(v),
            AnyValue::Float64(v) => v as i64,
            AnyValue::Float32(v) => v as i64,
            other => panic!("unexpected i64 value: {other:?}"),
        })
        .collect()
}

fn values_as_opt_f64(df: &DataFrame, column: &str) -> Vec<Option<f64>> {
    let series = df.column(column).expect("column").as_materialized_series();
    (0..series.len())
        .map(|index| match series.get(index).expect("value") {
            AnyValue::Float64(v) => Some(v),
            AnyValue::Float32(v) => Some(v as f64),
            AnyValue::Int64(v) => Some(v as f64),
            AnyValue::Int32(v) => Some(f64::from(v)),
            AnyValue::Null => None,
            other => panic!("unexpected f64 value: {other:?}"),
        })
        .collect()
}

fn values_as_datetime_millis(df: &DataFrame, column: &str) -> Vec<i64> {
    let series = df
        .column(column)
        .expect("column")
        .as_materialized_series()
        .cast(&DataType::Int64)
        .expect("cast to int64");
    (0..series.len())
        .map(|index| match series.get(index).expect("value") {
            AnyValue::Int64(v) => v,
            other => panic!("unexpected datetime value: {other:?}"),
        })
        .collect()
}

fn values_as_date_days(df: &DataFrame, column: &str) -> Vec<i32> {
    let series = df
        .column(column)
        .expect("column")
        .as_materialized_series()
        .cast(&DataType::Int32)
        .expect("cast to int32");
    (0..series.len())
        .map(|index| match series.get(index).expect("value") {
            AnyValue::Int32(v) => v,
            other => panic!("unexpected date value: {other:?}"),
        })
        .collect()
}

fn must_error<T, E: std::fmt::Display>(result: std::result::Result<T, E>) -> String {
    match result {
        Ok(_) => panic!("expected error but got success"),
        Err(error) => error.to_string(),
    }
}

#[test]
fn test_update_single_assignment_no_selector() {
    let operation = UpdateOperation {
        selector: None,
        assignments: vec![Assignment {
            column: "status".to_string(),
            expression: "\"processed\"".to_string(),
        }],
    };

    let result = execute_update(&context(HashMap::new()), &operation)
        .expect("execute")
        .collect()
        .expect("collect");

    assert_eq!(
        values_as_strings(&result, "status"),
        vec!["processed", "processed", "processed"]
    );
}

#[test]
fn test_empty_assignments_error() {
    let operation = UpdateOperation {
        selector: None,
        assignments: vec![],
    };

    let error = must_error(execute_update(&context(HashMap::new()), &operation));
    assert!(error.contains("Update operation requires at least one assignment"));
}

#[test]
fn test_updated_at_system_column_update() {
    let operation = UpdateOperation {
        selector: None,
        assignments: vec![Assignment {
            column: "status".to_string(),
            expression: "\"processed\"".to_string(),
        }],
    };

    let ctx = context(HashMap::new());
    let timestamp = ctx.run_timestamp.timestamp_millis();

    let result = execute_update(&ctx, &operation)
        .expect("execute")
        .collect()
        .expect("collect");

    assert_eq!(values_as_i64(&result, "_updated_at"), vec![timestamp; 3]);
}

#[test]
fn test_updated_at_keeps_datetime_dtype_when_input_is_datetime() {
    let operation = UpdateOperation {
        selector: Some("status = \"active\"".to_string()),
        assignments: vec![Assignment {
            column: "status".to_string(),
            expression: "\"processed\"".to_string(),
        }],
    };

    let frame = df![
        "id" => [1i64, 2],
        "status" => ["active", "inactive"],
        "_updated_at" => [10i64, 20],
    ]
    .expect("base dataframe")
    .lazy()
    .with_columns([col("_updated_at")
        .cast(DataType::Datetime(TimeUnit::Milliseconds, None))
        .alias("_updated_at")]);

    let ctx = context_with_frame(frame, HashMap::new());
    let timestamp = ctx.run_timestamp.timestamp_millis();

    let result = execute_update(&ctx, &operation)
        .expect("execute")
        .collect()
        .expect("collect");

    assert!(matches!(
        result.column("_updated_at").expect("column").dtype(),
        DataType::Datetime(TimeUnit::Milliseconds, _)
    ));
    assert_eq!(
        values_as_datetime_millis(&result, "_updated_at"),
        vec![timestamp, 20]
    );
}

#[test]
fn test_updated_at_preserves_microsecond_datetime_unit() {
    let operation = UpdateOperation {
        selector: Some("status = \"active\"".to_string()),
        assignments: vec![Assignment {
            column: "status".to_string(),
            expression: "\"processed\"".to_string(),
        }],
    };

    let frame = df![
        "id" => [1i64, 2],
        "status" => ["active", "inactive"],
        "_updated_at" => [10i64, 20],
    ]
    .expect("base dataframe")
    .lazy()
    .with_columns([col("_updated_at")
        .cast(DataType::Datetime(TimeUnit::Microseconds, None))
        .alias("_updated_at")]);

    let ctx = context_with_frame(frame, HashMap::new());
    let timestamp = ctx.run_timestamp.timestamp_micros();

    let result = execute_update(&ctx, &operation)
        .expect("execute")
        .collect()
        .expect("collect");

    assert!(matches!(
        result.column("_updated_at").expect("column").dtype(),
        DataType::Datetime(TimeUnit::Microseconds, _)
    ));
    assert_eq!(
        values_as_datetime_millis(&result, "_updated_at"),
        vec![timestamp, 20]
    );
}

#[test]
fn test_updated_at_preserves_nanosecond_datetime_unit() {
    let operation = UpdateOperation {
        selector: Some("status = \"active\"".to_string()),
        assignments: vec![Assignment {
            column: "status".to_string(),
            expression: "\"processed\"".to_string(),
        }],
    };

    let frame = df![
        "id" => [1i64, 2],
        "status" => ["active", "inactive"],
        "_updated_at" => [10i64, 20],
    ]
    .expect("base dataframe")
    .lazy()
    .with_columns([col("_updated_at")
        .cast(DataType::Datetime(TimeUnit::Nanoseconds, None))
        .alias("_updated_at")]);

    let ctx = context_with_frame(frame, HashMap::new());
    let timestamp = ctx
        .run_timestamp
        .timestamp_nanos_opt()
        .expect("timestamp nanos");

    let result = execute_update(&ctx, &operation)
        .expect("execute")
        .collect()
        .expect("collect");

    assert!(matches!(
        result.column("_updated_at").expect("column").dtype(),
        DataType::Datetime(TimeUnit::Nanoseconds, _)
    ));
    assert_eq!(
        values_as_datetime_millis(&result, "_updated_at"),
        vec![timestamp, 20]
    );
}

#[test]
fn test_update_with_named_selector() {
    let mut selectors = HashMap::new();
    selectors.insert("active_rows".to_string(), "status = \"active\"".to_string());

    let operation = UpdateOperation {
        selector: Some("{{active_rows}}".to_string()),
        assignments: vec![Assignment {
            column: "status".to_string(),
            expression: "\"processed\"".to_string(),
        }],
    };

    let ctx = context(selectors);
    let timestamp = ctx.run_timestamp.timestamp_millis();

    let result = execute_update(&ctx, &operation)
        .expect("execute")
        .collect()
        .expect("collect");

    assert_eq!(
        values_as_strings(&result, "status"),
        vec!["processed", "processed", "inactive"]
    );
    assert_eq!(
        values_as_i64(&result, "_updated_at"),
        vec![timestamp, timestamp, 30]
    );
}

#[test]
fn test_named_selector_interpolation_within_expression() {
    let mut selectors = HashMap::new();
    selectors.insert("active_rows".to_string(), "status = \"active\"".to_string());

    let operation = UpdateOperation {
        selector: Some("{{active_rows}} AND amount > 150".to_string()),
        assignments: vec![Assignment {
            column: "count".to_string(),
            expression: "count + 100".to_string(),
        }],
    };

    let result = execute_update(&context(selectors), &operation)
        .expect("execute")
        .collect()
        .expect("collect");

    assert_eq!(values_as_i64(&result, "count"), vec![1, 102, 3]);
}

#[test]
fn test_named_selector_interpolation_preserves_boolean_precedence() {
    let mut selectors = HashMap::new();
    selectors.insert(
        "wide_match".to_string(),
        "status = \"inactive\" OR amount > 250".to_string(),
    );

    let operation = UpdateOperation {
        selector: Some("{{wide_match}} AND amount < 260".to_string()),
        assignments: vec![Assignment {
            column: "count".to_string(),
            expression: "count + 100".to_string(),
        }],
    };

    let result = execute_update(&context(selectors), &operation)
        .expect("execute")
        .collect()
        .expect("collect");

    assert_eq!(values_as_i64(&result, "count"), vec![1, 2, 3]);
    assert_eq!(values_as_i64(&result, "_updated_at"), vec![10, 20, 30]);
}

#[test]
fn test_empty_selector_string_applies_to_all_rows() {
    let operation = UpdateOperation {
        selector: Some("".to_string()),
        assignments: vec![Assignment {
            column: "status".to_string(),
            expression: "\"processed\"".to_string(),
        }],
    };

    let result = execute_update(&context(HashMap::new()), &operation)
        .expect("execute")
        .collect()
        .expect("collect");

    assert_eq!(
        values_as_strings(&result, "status"),
        vec!["processed", "processed", "processed"]
    );
}

#[test]
fn test_whitespace_selector_applies_to_all_rows() {
    let operation = UpdateOperation {
        selector: Some("   \n\t  ".to_string()),
        assignments: vec![Assignment {
            column: "status".to_string(),
            expression: "\"processed\"".to_string(),
        }],
    };

    let result = execute_update(&context(HashMap::new()), &operation)
        .expect("execute")
        .collect()
        .expect("collect");

    assert_eq!(
        values_as_strings(&result, "status"),
        vec!["processed", "processed", "processed"]
    );
}

#[test]
fn test_undefined_selector_name_error() {
    let operation = UpdateOperation {
        selector: Some("{{missing_selector}}".to_string()),
        assignments: vec![Assignment {
            column: "status".to_string(),
            expression: "\"processed\"".to_string(),
        }],
    };

    let error = must_error(execute_update(&context(HashMap::new()), &operation));
    assert!(error.contains("Selector 'missing_selector' not defined in Project"));
}

#[test]
fn test_if_function_expression_support() {
    let operation = UpdateOperation {
        selector: None,
        assignments: vec![Assignment {
            column: "status".to_string(),
            expression: "IF(amount > 150, \"high\", \"normal\")".to_string(),
        }],
    };

    let result = execute_update(&context(HashMap::new()), &operation)
        .expect("execute")
        .collect()
        .expect("collect");

    assert_eq!(
        values_as_strings(&result, "status"),
        vec!["normal", "high", "high"]
    );
}

#[test]
fn test_round_floor_ceil_mod_function_support() {
    let operation = UpdateOperation {
        selector: None,
        assignments: vec![
            Assignment {
                column: "amount".to_string(),
                expression: "ROUND(amount / 3, 2)".to_string(),
            },
            Assignment {
                column: "count".to_string(),
                expression: "MOD(count + 1, 2) + CEIL(0.0) + FLOOR(0.0)".to_string(),
            },
        ],
    };

    let result = execute_update(&context(HashMap::new()), &operation)
        .expect("execute")
        .collect()
        .expect("collect");

    assert_eq!(
        values_as_opt_f64(&result, "amount"),
        vec![Some(33.33), Some(66.67), Some(100.0)]
    );
    assert_eq!(values_as_i64(&result, "count"), vec![0, 1, 0]);
}

#[test]
fn test_string_function_support() {
    let operation = UpdateOperation {
        selector: None,
        assignments: vec![
            Assignment {
                column: "normalized".to_string(),
                expression: "UPPER(TRIM(name))".to_string(),
            },
            Assignment {
                column: "suffix".to_string(),
                expression: "RIGHT(LOWER(TRIM(name)), 2)".to_string(),
            },
            Assignment {
                column: "joined".to_string(),
                expression: "CONCAT(LEFT(TRIM(name), 2), \"-\", RIGHT(TRIM(name), 2))".to_string(),
            },
            Assignment {
                column: "marker".to_string(),
                expression: "IF(CONTAINS(text, \"xo\"), REPLACE(text, \"x\", \"z\"), text)"
                    .to_string(),
            },
            Assignment {
                column: "name_len".to_string(),
                expression: "LEN(TRIM(name))".to_string(),
            },
        ],
    };

    let frame = df![
        "name" => [" Alpha ", "Beta"],
        "text" => [" xoxo ", "None"],
        "_updated_at" => [10i64, 20],
    ]
    .expect("input dataframe")
    .lazy();

    let result = execute_update(&context_with_frame(frame, HashMap::new()), &operation)
        .expect("execute")
        .collect()
        .expect("collect");

    assert_eq!(
        values_as_strings(&result, "normalized"),
        vec!["ALPHA", "BETA"]
    );
    assert_eq!(values_as_strings(&result, "suffix"), vec!["ha", "ta"]);
    assert_eq!(values_as_strings(&result, "joined"), vec!["Al-ha", "Be-ta"]);
    assert_eq!(values_as_strings(&result, "marker"), vec![" zozo ", "None"]);
    assert_eq!(values_as_i64(&result, "name_len"), vec![5, 4]);
}

#[test]
fn test_date_function_support() {
    let operation = UpdateOperation {
        selector: None,
        assignments: vec![
            Assignment {
                column: "year_num".to_string(),
                expression: "YEAR(DATE(date_str))".to_string(),
            },
            Assignment {
                column: "month_num".to_string(),
                expression: "MONTH(DATE(date_str))".to_string(),
            },
            Assignment {
                column: "day_plus_two".to_string(),
                expression: "DAY(DATEADD(DATE(date_str), 2))".to_string(),
            },
            Assignment {
                column: "days_until_cutoff".to_string(),
                expression: "DATEDIFF(DATE(\"2026-02-10\"), DATE(date_str))".to_string(),
            },
        ],
    };

    let frame = df![
        "date_str" => ["2026-01-15", "2026-02-01"],
        "_updated_at" => [10i64, 20],
    ]
    .expect("input dataframe")
    .lazy();

    let result = execute_update(&context_with_frame(frame, HashMap::new()), &operation)
        .expect("execute")
        .collect()
        .expect("collect");

    assert_eq!(values_as_i64(&result, "year_num"), vec![2026, 2026]);
    assert_eq!(values_as_i64(&result, "month_num"), vec![1, 2]);
    assert_eq!(values_as_i64(&result, "day_plus_two"), vec![17, 3]);
    assert_eq!(values_as_i64(&result, "days_until_cutoff"), vec![26, 9]);
}

#[test]
fn test_today_function_returns_date_value() {
    let operation = UpdateOperation {
        selector: None,
        assignments: vec![Assignment {
            column: "today_value".to_string(),
            expression: "TODAY()".to_string(),
        }],
    };

    let ctx = context(HashMap::new());
    let expected_days = (ctx.run_timestamp.date_naive()
        - chrono::NaiveDate::from_ymd_opt(1970, 1, 1).expect("epoch date"))
    .num_days() as i32;

    let result = execute_update(&ctx, &operation)
        .expect("execute")
        .collect()
        .expect("collect");

    assert_eq!(
        result.column("today_value").expect("column").dtype(),
        &DataType::Date
    );
    assert_eq!(
        values_as_date_days(&result, "today_value"),
        vec![expected_days, expected_days, expected_days]
    );
}

#[test]
fn test_default_selector_excludes_soft_deleted_rows() {
    let operation = UpdateOperation {
        selector: None,
        assignments: vec![Assignment {
            column: "status".to_string(),
            expression: "\"processed\"".to_string(),
        }],
    };

    let frame = df![
        "id" => [1i64, 2, 3],
        "status" => ["active", "active", "inactive"],
        "_deleted" => [false, true, false],
        "_updated_at" => [10i64, 20, 30],
    ]
    .expect("base dataframe")
    .lazy();
    let ctx = context_with_frame(frame, HashMap::new());
    let timestamp = ctx.run_timestamp.timestamp_millis();

    let result = execute_update(&ctx, &operation)
        .expect("execute")
        .collect()
        .expect("collect");

    assert_eq!(
        values_as_strings(&result, "status"),
        vec!["processed", "active", "processed"]
    );
    assert_eq!(
        values_as_i64(&result, "_updated_at"),
        vec![timestamp, 20, timestamp]
    );
}

#[test]
fn test_explicit_selector_still_excludes_soft_deleted_rows() {
    let operation = UpdateOperation {
        selector: Some("id >= 2".to_string()),
        assignments: vec![Assignment {
            column: "status".to_string(),
            expression: "\"processed\"".to_string(),
        }],
    };

    let frame = df![
        "id" => [1i64, 2, 3],
        "status" => ["active", "active", "inactive"],
        "_deleted" => [false, true, false],
        "_updated_at" => [10i64, 20, 30],
    ]
    .expect("base dataframe")
    .lazy();
    let ctx = context_with_frame(frame, HashMap::new());
    let timestamp = ctx.run_timestamp.timestamp_millis();

    let result = execute_update(&ctx, &operation)
        .expect("execute")
        .collect()
        .expect("collect");

    assert_eq!(
        values_as_strings(&result, "status"),
        vec!["active", "active", "processed"]
    );
    assert_eq!(
        values_as_i64(&result, "_updated_at"),
        vec![10, 20, timestamp]
    );
}

#[test]
fn test_selector_without_interpolation() {
    let operation = UpdateOperation {
        selector: Some("status = \"inactive\"".to_string()),
        assignments: vec![Assignment {
            column: "status".to_string(),
            expression: "\"archived\"".to_string(),
        }],
    };

    let result = execute_update(&context(HashMap::new()), &operation)
        .expect("execute")
        .collect()
        .expect("collect");

    assert_eq!(
        values_as_strings(&result, "status"),
        vec!["active", "active", "archived"]
    );
}

#[test]
fn test_utf8_selector_returns_error_instead_of_panicking() {
    let operation = UpdateOperation {
        selector: Some("søjle > 0".to_string()),
        assignments: vec![Assignment {
            column: "count".to_string(),
            expression: "count + 1".to_string(),
        }],
    };

    let result = std::panic::catch_unwind(|| execute_update(&context(HashMap::new()), &operation));
    assert!(result.is_ok(), "selector parsing must not panic on UTF-8");
}

#[test]
fn test_update_with_selector_filters_rows() {
    let operation = UpdateOperation {
        selector: Some("amount > 150".to_string()),
        assignments: vec![Assignment {
            column: "count".to_string(),
            expression: "count + 10".to_string(),
        }],
    };

    let result = execute_update(&context(HashMap::new()), &operation)
        .expect("execute")
        .collect()
        .expect("collect");

    assert_eq!(values_as_i64(&result, "count"), vec![1, 12, 13]);
}

#[test]
fn test_non_matching_rows_unchanged() {
    let operation = UpdateOperation {
        selector: Some("status = \"inactive\"".to_string()),
        assignments: vec![Assignment {
            column: "amount".to_string(),
            expression: "amount * 2".to_string(),
        }],
    };

    let result = execute_update(&context(HashMap::new()), &operation)
        .expect("execute")
        .collect()
        .expect("collect");

    assert_eq!(
        values_as_opt_f64(&result, "amount"),
        vec![Some(100.0), Some(200.0), Some(600.0)]
    );
}

#[test]
fn test_invalid_selector_expression_error() {
    let operation = UpdateOperation {
        selector: Some("status ?? \"active\"".to_string()),
        assignments: vec![Assignment {
            column: "status".to_string(),
            expression: "\"processed\"".to_string(),
        }],
    };

    let error = must_error(execute_update(&context(HashMap::new()), &operation));
    assert!(error.contains("Failed to compile selector expression"));
}

#[test]
fn test_update_multiple_assignments() {
    let operation = UpdateOperation {
        selector: None,
        assignments: vec![
            Assignment {
                column: "amount".to_string(),
                expression: "amount * 0.5".to_string(),
            },
            Assignment {
                column: "count".to_string(),
                expression: "count + 1".to_string(),
            },
            Assignment {
                column: "status".to_string(),
                expression: "\"processed\"".to_string(),
            },
        ],
    };

    let result = execute_update(&context(HashMap::new()), &operation)
        .expect("execute")
        .collect()
        .expect("collect");

    assert_eq!(
        values_as_opt_f64(&result, "amount"),
        vec![Some(50.0), Some(100.0), Some(150.0)]
    );
    assert_eq!(values_as_i64(&result, "count"), vec![2, 3, 4]);
    assert_eq!(
        values_as_strings(&result, "status"),
        vec!["processed", "processed", "processed"]
    );
}

#[test]
fn test_update_assignment_order_is_sequential() {
    let operation = UpdateOperation {
        selector: Some("id = 1".to_string()),
        assignments: vec![
            Assignment {
                column: "status".to_string(),
                expression: "\"processed\"".to_string(),
            },
            Assignment {
                column: "count".to_string(),
                expression: "if(status = \"processed\", 100, 0)".to_string(),
            },
        ],
    };

    let result = execute_update(&context(HashMap::new()), &operation)
        .expect("execute")
        .collect()
        .expect("collect");

    assert_eq!(values_as_strings(&result, "status"), vec!["processed", "active", "inactive"]);
    assert_eq!(values_as_i64(&result, "count"), vec![100, 2, 3]);
}

#[test]
fn test_update_creates_new_column() {
    let operation = UpdateOperation {
        selector: None,
        assignments: vec![Assignment {
            column: "discount".to_string(),
            expression: "0.1".to_string(),
        }],
    };

    let result = execute_update(&context(HashMap::new()), &operation)
        .expect("execute")
        .collect()
        .expect("collect");

    assert_eq!(
        values_as_opt_f64(&result, "discount"),
        vec![Some(0.1), Some(0.1), Some(0.1)]
    );
}

#[test]
fn test_new_column_has_null_in_non_matching_rows() {
    let operation = UpdateOperation {
        selector: Some("status = \"active\"".to_string()),
        assignments: vec![Assignment {
            column: "discount".to_string(),
            expression: "0.2".to_string(),
        }],
    };

    let result = execute_update(&context(HashMap::new()), &operation)
        .expect("execute")
        .collect()
        .expect("collect");

    assert_eq!(
        values_as_opt_f64(&result, "discount"),
        vec![Some(0.2), Some(0.2), None]
    );
}

#[test]
fn test_undefined_column_in_expression_error() {
    let operation = UpdateOperation {
        selector: None,
        assignments: vec![Assignment {
            column: "amount".to_string(),
            expression: "unknown_column + 1".to_string(),
        }],
    };

    let error = must_error(execute_update(&context(HashMap::new()), &operation));
    let message = error;
    assert!(
        message.contains("Failed to validate update output schema")
            || message.contains("not found")
    );
}

#[test]
fn test_type_mismatch_in_assignment_error() {
    let operation = UpdateOperation {
        selector: None,
        assignments: vec![Assignment {
            column: "amount".to_string(),
            expression: "amount + \"x\"".to_string(),
        }],
    };

    let error = must_error(execute_update(&context(HashMap::new()), &operation));
    let message = error;
    assert!(
        message.contains("Failed to validate update output schema") || message.contains("Schema")
    );
}

#[test]
fn test_invalid_column_name_validation() {
    let operation = UpdateOperation {
        selector: None,
        assignments: vec![Assignment {
            column: "bad-column".to_string(),
            expression: "1".to_string(),
        }],
    };

    let error = must_error(execute_update(&context(HashMap::new()), &operation));
    assert!(error.contains("Invalid column name"));
}

#[test]
fn test_reserved_deleted_column_assignment_validation() {
    let operation = UpdateOperation {
        selector: None,
        assignments: vec![Assignment {
            column: "_deleted".to_string(),
            expression: "true".to_string(),
        }],
    };

    let error = must_error(execute_update(&context(HashMap::new()), &operation));
    assert!(error.contains("Cannot assign to reserved system column '_deleted'"));
}

#[test]
fn test_reserved_updated_at_column_assignment_validation() {
    let operation = UpdateOperation {
        selector: None,
        assignments: vec![Assignment {
            column: "_updated_at".to_string(),
            expression: "123".to_string(),
        }],
    };

    let error = must_error(execute_update(&context(HashMap::new()), &operation));
    assert!(error.contains("Cannot assign to reserved system column '_updated_at'"));
}
