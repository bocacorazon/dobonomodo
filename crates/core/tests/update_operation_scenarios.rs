mod common;

use std::fs;
use std::path::PathBuf;

use common::UpdateScenarioHarness;
use dobo_core::engine::ops::UpdateOperation;
use polars::prelude::{df, AnyValue, DataFrame, IntoLazy};
use serde::Deserialize;
use serde_yaml::Value;
use std::collections::HashMap;

fn read_scenario(name: &str) -> Value {
    let path = PathBuf::from(env!("CARGO_MANIFEST_DIR"))
        .join("tests")
        .join("integration")
        .join("scenarios")
        .join(name);

    let content = fs::read_to_string(path).expect("scenario file should exist");
    serde_yaml::from_str(&content).expect("scenario should be valid yaml")
}

#[derive(Deserialize)]
struct InputTable<T> {
    columns: Vec<String>,
    rows: Vec<T>,
}

#[derive(Deserialize)]
struct Scenario<T> {
    selectors: Option<HashMap<String, String>>,
    operation: UpdateOperation,
    input: InputTable<T>,
    expected: InputTable<T>,
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

fn values_as_i64(df: &DataFrame, column: &str) -> Vec<i64> {
    let series = df.column(column).expect("column").as_materialized_series();
    (0..series.len())
        .map(|index| match series.get(index).expect("value") {
            AnyValue::Int64(v) => v,
            AnyValue::Int32(v) => i64::from(v),
            AnyValue::UInt64(v) => v as i64,
            AnyValue::UInt32(v) => i64::from(v),
            other => panic!("unexpected i64 value: {other:?}"),
        })
        .collect()
}

fn values_as_strings(df: &DataFrame, column: &str) -> Vec<String> {
    let series = df.column(column).expect("column").as_materialized_series();
    (0..series.len())
        .map(|index| match series.get(index).expect("value") {
            AnyValue::String(v) => v.to_string(),
            AnyValue::StringOwned(v) => v.to_string(),
            AnyValue::Null => "null".to_string(),
            other => other.to_string(),
        })
        .collect()
}

#[test]
fn ts03_fx_conversion_executes_expected_update_rows() {
    let scenario = read_scenario("ts03_fx_conversion.yaml");
    let parsed: Scenario<(i64, f64, Option<f64>, i64)> =
        serde_yaml::from_value(scenario).expect("typed scenario");

    assert_eq!(parsed.operation.selector, None);
    assert_eq!(parsed.input.columns, parsed.expected.columns);

    let (order_ids, amount_usd, amount_eur, updated_at): (Vec<_>, Vec<_>, Vec<_>, Vec<_>) =
        parsed.input.rows.into_iter().fold(
            (Vec::new(), Vec::new(), Vec::new(), Vec::new()),
            |mut acc, row| {
                acc.0.push(row.0);
                acc.1.push(row.1);
                acc.2.push(row.2);
                acc.3.push(row.3);
                acc
            },
        );

    let harness = UpdateScenarioHarness::new(parsed.selectors.unwrap_or_default());
    let output = harness.run_update_operation(
        parsed.operation,
        df![
            "order_id" => order_ids,
            "amount_usd" => amount_usd,
            "amount_eur" => amount_eur,
            "_updated_at" => updated_at,
        ]
        .expect("input dataframe")
        .lazy(),
    );

    let expected_amount_eur: Vec<Option<f64>> =
        parsed.expected.rows.iter().map(|row| row.2).collect();
    let expected_updated_at: Vec<i64> = parsed.expected.rows.iter().map(|row| row.3).collect();
    assert_eq!(
        values_as_opt_f64(&output, "amount_eur"),
        expected_amount_eur
    );
    assert_eq!(values_as_i64(&output, "_updated_at"), expected_updated_at);
}

#[test]
fn ts08_named_selector_executes_expected_update_rows() {
    let scenario = read_scenario("ts08_named_selector.yaml");
    let parsed: Scenario<(i64, String, i64)> =
        serde_yaml::from_value(scenario).expect("typed scenario");

    assert_eq!(
        parsed.operation.selector.as_deref(),
        Some("{{active_rows}}")
    );
    assert_eq!(parsed.input.columns, parsed.expected.columns);

    let (ids, status, updated_at): (Vec<_>, Vec<_>, Vec<_>) =
        parsed
            .input
            .rows
            .into_iter()
            .fold((Vec::new(), Vec::new(), Vec::new()), |mut acc, row| {
                acc.0.push(row.0);
                acc.1.push(row.1);
                acc.2.push(row.2);
                acc
            });

    let harness = UpdateScenarioHarness::new(parsed.selectors.unwrap_or_default());
    let output = harness.run_update_operation(
        parsed.operation,
        df![
            "id" => ids,
            "status" => status,
            "_updated_at" => updated_at,
        ]
        .expect("input dataframe")
        .lazy(),
    );

    let expected_status: Vec<String> = parsed
        .expected
        .rows
        .iter()
        .map(|row| row.1.clone())
        .collect();
    let expected_updated_at: Vec<i64> = parsed.expected.rows.iter().map(|row| row.2).collect();
    assert_eq!(values_as_strings(&output, "status"), expected_status);
    assert_eq!(values_as_i64(&output, "_updated_at"), expected_updated_at);
}
