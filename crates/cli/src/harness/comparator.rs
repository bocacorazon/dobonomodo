use anyhow::{bail, Result};
use dobo_core::model::{
    DataMismatch, MatchMode, MismatchType, TraceAssertion, TraceChangeType, TraceMismatch,
    TraceMismatchType,
};
use polars::prelude::*;
use std::collections::HashMap;

type Row = HashMap<String, serde_json::Value>;
type Rows = Vec<Row>;

/// Compare actual output to expected output
pub fn compare_output(
    actual: &DataFrame,
    expected: &DataFrame,
    mode: MatchMode,
    validate_metadata: bool,
    order_sensitive: bool,
) -> Result<Vec<DataMismatch>> {
    // Strip system columns unless validating metadata
    let actual_clean = if validate_metadata {
        actual.clone()
    } else {
        strip_system_columns(actual)?
    };

    let expected_clean = if validate_metadata {
        expected.clone()
    } else {
        strip_system_columns(expected)?
    };

    ensure_matching_schema(&actual_clean, &expected_clean)?;

    match mode {
        MatchMode::Exact => compare_exact(&actual_clean, &expected_clean, order_sensitive),
        MatchMode::Subset => compare_subset(&actual_clean, &expected_clean, order_sensitive),
    }
}

fn ensure_matching_schema(actual: &DataFrame, expected: &DataFrame) -> Result<()> {
    let actual_columns: Vec<String> = actual
        .get_column_names()
        .iter()
        .map(|name| name.to_string())
        .collect();
    let expected_columns: Vec<String> = expected
        .get_column_names()
        .iter()
        .map(|name| name.to_string())
        .collect();

    if actual_columns != expected_columns {
        bail!(
            "Schema mismatch: expected columns {:?}, actual columns {:?}",
            expected_columns,
            actual_columns
        );
    }

    Ok(())
}

/// Strip system columns (those starting with _) from DataFrame
pub fn strip_system_columns(df: &DataFrame) -> Result<DataFrame> {
    let business_cols: Vec<String> = df
        .get_column_names()
        .iter()
        .filter(|col| !col.starts_with('_'))
        .map(|s| s.to_string())
        .collect();

    Ok(df.select(business_cols)?)
}

/// Compare in exact mode (all rows must match, no extras allowed)
/// T096: Add order_sensitive support
fn compare_exact(
    actual: &DataFrame,
    expected: &DataFrame,
    order_sensitive: bool,
) -> Result<Vec<DataMismatch>> {
    let mut mismatches = Vec::new();

    if order_sensitive {
        // Order-sensitive comparison: check row-by-row in order
        let actual_rows = dataframe_to_rows(actual)?;
        let expected_rows = dataframe_to_rows(expected)?;

        let max_len = actual_rows.len().max(expected_rows.len());

        for idx in 0..max_len {
            match (actual_rows.get(idx), expected_rows.get(idx)) {
                (Some(actual_row), Some(expected_row)) => {
                    if !rows_equal(actual_row, expected_row) {
                        // Row values differ
                        let differing_columns = find_differing_columns(expected_row, actual_row);
                        mismatches.push(DataMismatch {
                            mismatch_type: MismatchType::ValueMismatch,
                            expected: Some(expected_row.clone()),
                            actual: Some(actual_row.clone()),
                            differing_columns,
                        });
                    }
                }
                (Some(actual_row), None) => {
                    // Extra row in actual
                    mismatches.push(DataMismatch {
                        mismatch_type: MismatchType::ExtraRow,
                        expected: None,
                        actual: Some(actual_row.clone()),
                        differing_columns: vec![],
                    });
                }
                (None, Some(expected_row)) => {
                    // Missing row in actual
                    mismatches.push(DataMismatch {
                        mismatch_type: MismatchType::MissingRow,
                        expected: Some(expected_row.clone()),
                        actual: None,
                        differing_columns: vec![],
                    });
                }
                (None, None) => unreachable!(),
            }
        }
    } else {
        // Order-insensitive exact comparison with multiplicity and value-diff pairing.
        let (mut expected_unmatched, mut actual_unmatched) =
            split_unmatched_rows(dataframe_to_rows(expected)?, dataframe_to_rows(actual)?);

        while !expected_unmatched.is_empty() && !actual_unmatched.is_empty() {
            let (expected_idx, actual_idx, differing_columns) =
                best_value_mismatch_pair(&expected_unmatched, &actual_unmatched);
            let expected_row = expected_unmatched.swap_remove(expected_idx);
            let actual_row = actual_unmatched.swap_remove(actual_idx);

            mismatches.push(DataMismatch {
                mismatch_type: MismatchType::ValueMismatch,
                expected: Some(expected_row),
                actual: Some(actual_row),
                differing_columns,
            });
        }

        for row in expected_unmatched {
            mismatches.push(DataMismatch {
                mismatch_type: MismatchType::MissingRow,
                expected: Some(row),
                actual: None,
                differing_columns: vec![],
            });
        }

        for row in actual_unmatched {
            mismatches.push(DataMismatch {
                mismatch_type: MismatchType::ExtraRow,
                expected: None,
                actual: Some(row),
                differing_columns: vec![],
            });
        }
    }

    Ok(mismatches)
}

/// Compare in subset mode (expected rows must exist in actual, extra rows tolerated)
fn compare_subset(
    actual: &DataFrame,
    expected: &DataFrame,
    order_sensitive: bool,
) -> Result<Vec<DataMismatch>> {
    let (mut expected_unmatched, mut actual_unmatched) = if order_sensitive {
        split_unmatched_rows_ordered_subset(
            dataframe_to_rows(expected)?,
            dataframe_to_rows(actual)?,
        )
    } else {
        split_unmatched_rows(dataframe_to_rows(expected)?, dataframe_to_rows(actual)?)
    };

    let mut mismatches = Vec::new();

    while let Some((expected_idx, actual_idx, differing_columns)) =
        best_non_exact_value_mismatch_pair(&expected_unmatched, &actual_unmatched)
    {
        let expected_row = expected_unmatched.swap_remove(expected_idx);
        let actual_row = actual_unmatched.swap_remove(actual_idx);

        mismatches.push(DataMismatch {
            mismatch_type: MismatchType::ValueMismatch,
            expected: Some(expected_row),
            actual: Some(actual_row),
            differing_columns,
        });
    }

    for row in expected_unmatched {
        mismatches.push(DataMismatch {
            mismatch_type: MismatchType::MissingRow,
            expected: Some(row),
            actual: None,
            differing_columns: vec![],
        });
    }

    Ok(mismatches)
}

fn split_unmatched_rows(left_rows: Rows, right_rows: Rows) -> (Rows, Rows) {
    let mut right_consumed = vec![false; right_rows.len()];
    let mut left_unmatched = Vec::new();

    for left_row in left_rows {
        let matched_idx = right_rows
            .iter()
            .enumerate()
            .find(|(idx, right_row)| !right_consumed[*idx] && rows_equal(&left_row, right_row))
            .map(|(idx, _)| idx);

        if let Some(idx) = matched_idx {
            right_consumed[idx] = true;
        } else {
            left_unmatched.push(left_row);
        }
    }

    let right_unmatched = right_rows
        .into_iter()
        .enumerate()
        .filter_map(|(idx, row)| (!right_consumed[idx]).then_some(row))
        .collect();

    (left_unmatched, right_unmatched)
}

fn split_unmatched_rows_ordered_subset(expected_rows: Rows, actual_rows: Rows) -> (Rows, Rows) {
    let mut actual_consumed = vec![false; actual_rows.len()];
    let mut expected_unmatched = Vec::new();
    let mut search_start = 0;

    for expected_row in expected_rows {
        let matched_idx = (search_start..actual_rows.len())
            .find(|idx| rows_equal(&expected_row, &actual_rows[*idx]));

        if let Some(idx) = matched_idx {
            actual_consumed[idx] = true;
            search_start = idx + 1;
        } else {
            expected_unmatched.push(expected_row);
        }
    }

    let actual_unmatched = actual_rows
        .into_iter()
        .enumerate()
        .filter_map(|(idx, row)| (!actual_consumed[idx]).then_some(row))
        .collect();

    (expected_unmatched, actual_unmatched)
}

fn best_value_mismatch_pair(
    expected_rows: &[Row],
    actual_rows: &[Row],
) -> (usize, usize, Vec<String>) {
    let mut best_pair: Option<(usize, usize, Vec<String>)> = None;

    for (expected_idx, expected_row) in expected_rows.iter().enumerate() {
        for (actual_idx, actual_row) in actual_rows.iter().enumerate() {
            let differing_columns = find_differing_columns(expected_row, actual_row);

            match &best_pair {
                Some((_, _, current_diff)) if current_diff.len() <= differing_columns.len() => {}
                _ => {
                    best_pair = Some((expected_idx, actual_idx, differing_columns));
                }
            }
        }
    }

    best_pair.expect("best pair exists when both row lists are non-empty")
}

fn best_non_exact_value_mismatch_pair(
    expected_rows: &[Row],
    actual_rows: &[Row],
) -> Option<(usize, usize, Vec<String>)> {
    let mut best_pair: Option<(usize, usize, Vec<String>)> = None;

    for (expected_idx, expected_row) in expected_rows.iter().enumerate() {
        for (actual_idx, actual_row) in actual_rows.iter().enumerate() {
            let differing_columns = find_differing_columns(expected_row, actual_row);
            if differing_columns.is_empty() {
                continue;
            }

            match &best_pair {
                Some((_, _, current_diff)) if current_diff.len() <= differing_columns.len() => {}
                _ => {
                    best_pair = Some((expected_idx, actual_idx, differing_columns));
                }
            }
        }
    }

    best_pair
}

/// Convert DataFrame to vector of row HashMaps
fn dataframe_to_rows(df: &DataFrame) -> Result<Rows> {
    let mut rows = Vec::new();
    let height = df.height();
    let columns = df.get_columns();

    for row_idx in 0..height {
        let mut row = HashMap::new();
        for col in columns {
            let col_name = col.name().to_string();
            let value = column_value_at(col, row_idx);
            row.insert(col_name, value);
        }
        rows.push(row);
    }

    Ok(rows)
}

/// Extract value from column at specific index
fn column_value_at(col: &Column, idx: usize) -> serde_json::Value {
    let series = col.as_materialized_series();

    // Handle different data types
    if let Ok(ca) = series.bool() {
        ca.get(idx)
            .map(serde_json::Value::Bool)
            .unwrap_or(serde_json::Value::Null)
    } else if let Ok(ca) = series.i64() {
        ca.get(idx)
            .map(serde_json::Value::from)
            .unwrap_or(serde_json::Value::Null)
    } else if let Ok(ca) = series.f64() {
        ca.get(idx)
            .map(serde_json::Value::from)
            .unwrap_or(serde_json::Value::Null)
    } else if let Ok(ca) = series.str() {
        ca.get(idx)
            .map(|s| serde_json::Value::String(s.to_string()))
            .unwrap_or(serde_json::Value::Null)
    } else {
        // Fallback to string representation
        serde_json::Value::String(format!("{:?}", series.get(idx).unwrap()))
    }
}

/// Compare two row HashMaps for equality
fn rows_equal(a: &Row, b: &Row) -> bool {
    if a.len() != b.len() {
        return false;
    }

    for (key, val_a) in a {
        match b.get(key) {
            Some(val_b) => {
                // Special handling for floats (with tolerance)
                if !values_equal(val_a, val_b) {
                    return false;
                }
            }
            None => return false,
        }
    }

    true
}

/// Compare two JSON values for equality (with float tolerance)
fn values_equal(a: &serde_json::Value, b: &serde_json::Value) -> bool {
    match (a, b) {
        (serde_json::Value::Number(n1), serde_json::Value::Number(n2)) => {
            if let (Some(f1), Some(f2)) = (n1.as_f64(), n2.as_f64()) {
                (f1 - f2).abs() < 1e-10
            } else {
                n1 == n2
            }
        }
        _ => a == b,
    }
}

/// Find columns that differ between two rows
fn find_differing_columns(expected: &Row, actual: &Row) -> Vec<String> {
    let mut differing = Vec::new();

    for (key, expected_val) in expected {
        if let Some(actual_val) = actual.get(key) {
            if !values_equal(expected_val, actual_val) {
                differing.push(key.clone());
            }
        } else {
            differing.push(key.clone());
        }
    }

    // Check for keys in actual that aren't in expected
    for key in actual.keys() {
        if !expected.contains_key(key) {
            differing.push(key.clone());
        }
    }

    differing.sort();
    differing.dedup();
    differing
}

/// Validate trace events against expected trace assertions
/// T073: Create validate_trace_events() function
pub fn validate_trace_events(
    actual_trace: &[serde_json::Value],
    assertions: &[TraceAssertion],
) -> Result<Vec<TraceMismatch>> {
    let mut mismatches = Vec::new();
    let mut matched_events = vec![false; actual_trace.len()];

    // For each assertion, find matching trace event
    for assertion in assertions {
        // T074: Implement trace event matching by operation_order and change_type
        let matching_index =
            find_matching_trace_event_index(actual_trace, assertion, &matched_events);

        match matching_index {
            Some(index) => {
                matched_events[index] = true;
                let event = &actual_trace[index];

                // Event found - validate row_match and expected_diff
                // T075: Add row_match validation
                if !validate_row_match(event, &assertion.row_match) {
                    mismatches.push(TraceMismatch {
                        operation_order: assertion.operation_order,
                        mismatch_type: TraceMismatchType::DiffMismatch,
                        expected: assertion.clone(),
                        actual: Some(event.clone()),
                    });
                    continue;
                }

                // T076: Add expected_diff validation for Updated change_type
                if let Some(ref expected_diff) = assertion.expected_diff {
                    if !validate_expected_diff(event, expected_diff) {
                        mismatches.push(TraceMismatch {
                            operation_order: assertion.operation_order,
                            mismatch_type: TraceMismatchType::DiffMismatch,
                            expected: assertion.clone(),
                            actual: Some(event.clone()),
                        });
                    }
                }
            }
            None => {
                // T077: Create TraceMismatch instances for missing events
                mismatches.push(TraceMismatch {
                    operation_order: assertion.operation_order,
                    mismatch_type: TraceMismatchType::MissingEvent,
                    expected: assertion.clone(),
                    actual: None,
                });
            }
        }
    }

    for (index, event) in actual_trace.iter().enumerate() {
        if !matched_events[index] {
            mismatches.push(TraceMismatch {
                operation_order: extract_operation_order(event),
                mismatch_type: TraceMismatchType::ExtraEvent,
                expected: placeholder_assertion_for_extra_event(event),
                actual: Some(event.clone()),
            });
        }
    }

    Ok(mismatches)
}

/// Find trace event matching assertion by operation_order and change_type
fn find_matching_trace_event_index(
    trace: &[serde_json::Value],
    assertion: &TraceAssertion,
    matched_events: &[bool],
) -> Option<usize> {
    for (idx, event) in trace.iter().enumerate() {
        if matched_events[idx] {
            continue;
        }

        // Extract operation_order from event
        let event_op_order = event
            .get("operation_order")
            .and_then(|v| v.as_i64())
            .map(|v| v as i32);

        // Extract change_type from event
        let event_change_type = event.get("change_type").and_then(|v| v.as_str());

        // Match assertion change_type to string
        let assertion_change_type_str = match assertion.change_type {
            dobo_core::model::TraceChangeType::Created => "created",
            dobo_core::model::TraceChangeType::Updated => "updated",
            dobo_core::model::TraceChangeType::Deleted => "deleted",
        };

        if event_op_order == Some(assertion.operation_order)
            && event_change_type == Some(assertion_change_type_str)
        {
            return Some(idx);
        }
    }

    None
}

fn extract_operation_order(event: &serde_json::Value) -> i32 {
    event
        .get("operation_order")
        .and_then(|value| value.as_i64())
        .map(|value| value as i32)
        .unwrap_or(-1)
}

fn placeholder_assertion_for_extra_event(event: &serde_json::Value) -> TraceAssertion {
    let change_type = match event
        .get("change_type")
        .and_then(|value| value.as_str())
        .map(|value| value.to_ascii_lowercase())
        .as_deref()
    {
        Some("updated") => TraceChangeType::Updated,
        Some("deleted") => TraceChangeType::Deleted,
        _ => TraceChangeType::Created,
    };

    TraceAssertion {
        operation_order: extract_operation_order(event),
        change_type,
        row_match: HashMap::new(),
        expected_diff: None,
    }
}

/// Validate that event row matches the assertion row_match criteria
fn validate_row_match(
    event: &serde_json::Value,
    row_match: &HashMap<String, serde_json::Value>,
) -> bool {
    // Get the row data from event (typically in "after" or "before" field)
    let row_data = event
        .get("after")
        .or_else(|| event.get("before"))
        .or_else(|| event.get("row"));

    if let Some(row) = row_data {
        // Check all row_match criteria
        for (key, expected_value) in row_match {
            let actual_value = row.get(key);
            match actual_value {
                Some(val) if values_equal(val, expected_value) => continue,
                _ => return false,
            }
        }
        true
    } else {
        false
    }
}

/// Validate expected_diff for Updated events
fn validate_expected_diff(
    event: &serde_json::Value,
    expected_diff: &HashMap<String, serde_json::Value>,
) -> bool {
    // Get the diff from event
    let diff = event.get("diff");

    if let Some(diff_obj) = diff {
        // Check all expected diff entries
        for (key, expected_value) in expected_diff {
            let actual_value = diff_obj.get(key);
            match actual_value {
                Some(val) if values_equal(val, expected_value) => continue,
                _ => return false,
            }
        }
        true
    } else {
        false
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use serde_json::json;

    #[test]
    fn exact_unordered_reports_value_mismatch_with_differing_columns() {
        let actual = df! {
            "id" => &[1i64],
            "value" => &[20i64],
        }
        .unwrap();
        let expected = df! {
            "id" => &[1i64],
            "value" => &[10i64],
        }
        .unwrap();

        let mismatches = compare_output(&actual, &expected, MatchMode::Exact, true, false).unwrap();
        assert_eq!(mismatches.len(), 1);
        assert_eq!(mismatches[0].mismatch_type, MismatchType::ValueMismatch);
        assert!(mismatches[0]
            .differing_columns
            .contains(&"value".to_string()));
    }

    #[test]
    fn exact_unordered_detects_duplicate_row_cardinality_mismatch() {
        let actual = df! {
            "id" => &[1i64],
        }
        .unwrap();
        let expected = df! {
            "id" => &[1i64, 1i64],
        }
        .unwrap();

        let mismatches = compare_output(&actual, &expected, MatchMode::Exact, true, false).unwrap();
        assert_eq!(mismatches.len(), 1);
        assert_eq!(mismatches[0].mismatch_type, MismatchType::MissingRow);
    }

    #[test]
    fn compare_output_returns_error_on_schema_mismatch() {
        let actual = df! {
            "id" => &[1i64],
            "value" => &[20i64],
        }
        .unwrap();
        let expected = df! {
            "id" => &[1i64],
            "amount" => &[20i64],
        }
        .unwrap();

        let error = compare_output(&actual, &expected, MatchMode::Exact, true, false)
            .unwrap_err()
            .to_string();
        assert!(error.contains("Schema mismatch"));
    }

    #[test]
    fn subset_mode_honors_order_sensitive_flag() {
        let actual = df! {
            "id" => &[2i64, 1i64],
            "value" => &[20i64, 10i64],
        }
        .unwrap();
        let expected = df! {
            "id" => &[1i64, 2i64],
            "value" => &[10i64, 20i64],
        }
        .unwrap();

        let order_sensitive =
            compare_output(&actual, &expected, MatchMode::Subset, true, true).unwrap();
        let order_insensitive =
            compare_output(&actual, &expected, MatchMode::Subset, true, false).unwrap();

        assert_eq!(order_insensitive.len(), 0);
        assert_eq!(order_sensitive.len(), 1);
        assert_eq!(order_sensitive[0].mismatch_type, MismatchType::MissingRow);
    }

    #[test]
    fn subset_mode_reports_value_mismatch_with_differing_columns() {
        let actual = df! {
            "id" => &[1i64],
            "value" => &[20i64],
        }
        .unwrap();
        let expected = df! {
            "id" => &[1i64],
            "value" => &[10i64],
        }
        .unwrap();

        let mismatches =
            compare_output(&actual, &expected, MatchMode::Subset, true, false).unwrap();
        assert_eq!(mismatches.len(), 1);
        assert_eq!(mismatches[0].mismatch_type, MismatchType::ValueMismatch);
        assert_eq!(mismatches[0].differing_columns, vec!["value".to_string()]);
    }

    #[test]
    fn trace_validation_detects_extra_events() {
        let actual_trace = vec![json!({
            "operation_order": 4,
            "change_type": "created",
            "after": {"id": "A1"}
        })];

        let mismatches = validate_trace_events(&actual_trace, &[]).unwrap();
        assert_eq!(mismatches.len(), 1);
        assert_eq!(mismatches[0].mismatch_type, TraceMismatchType::ExtraEvent);
        assert_eq!(mismatches[0].operation_order, 4);
    }
}
