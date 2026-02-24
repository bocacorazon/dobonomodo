use std::collections::BTreeMap;

use anyhow::Result;
use dobo_core::model::DeleteOperationParams;
use dobo_core::operations::delete::execute_delete;
use polars::prelude::{Column, DataFrame, IntoLazy};

fn base_frame() -> Result<DataFrame> {
    DataFrame::new(vec![
        Column::new("_row_id".into(), ["r1", "r2", "r3"].as_ref()),
        Column::new("_deleted".into(), [false, false, false].as_ref()),
        Column::new("_modified_at".into(), [10_i64, 10, 10].as_ref()),
        Column::new("amount".into(), [0_i64, 100, 200].as_ref()),
    ])
    .map_err(Into::into)
}

#[test]
fn test_delete_with_selector_marks_matching_rows() -> Result<()> {
    let frame = base_frame()?;
    let params = DeleteOperationParams {
        selector: Some("amount = 0".to_string()),
    };
    let selectors = BTreeMap::new();

    let result = execute_delete(frame.lazy(), &params, &selectors)?.collect()?;
    let deleted = result.column("_deleted")?.bool()?;
    let deleted_values: Vec<Option<bool>> = deleted.into_iter().collect();

    assert_eq!(deleted_values, vec![Some(true), Some(false), Some(false)]);
    Ok(())
}

#[test]
fn test_delete_updates_modified_at_timestamp() -> Result<()> {
    let frame = base_frame()?;
    let params = DeleteOperationParams {
        selector: Some("amount = 0".to_string()),
    };
    let selectors = BTreeMap::new();

    let result = execute_delete(frame.lazy(), &params, &selectors)?.collect()?;
    let modified = result.column("_modified_at")?.i64()?;
    let modified_values: Vec<Option<i64>> = modified.into_iter().collect();

    assert!(modified_values[0].unwrap_or_default() > 10);
    assert_eq!(modified_values[1], Some(10));
    assert_eq!(modified_values[2], Some(10));
    Ok(())
}

#[test]
fn test_delete_with_zero_matches_leaves_unchanged() -> Result<()> {
    let frame = base_frame()?;
    let params = DeleteOperationParams {
        selector: Some("amount < 0".to_string()),
    };
    let selectors = BTreeMap::new();

    let result = execute_delete(frame.lazy(), &params, &selectors)?.collect()?;
    let deleted = result.column("_deleted")?.bool()?;
    let modified = result.column("_modified_at")?.i64()?;

    let deleted_values: Vec<Option<bool>> = deleted.into_iter().collect();
    let modified_values: Vec<Option<i64>> = modified.into_iter().collect();

    assert_eq!(deleted_values, vec![Some(false), Some(false), Some(false)]);
    assert_eq!(modified_values, vec![Some(10), Some(10), Some(10)]);
    Ok(())
}

#[test]
fn test_delete_already_deleted_rows_no_op() -> Result<()> {
    let frame = DataFrame::new(vec![
        Column::new("_row_id".into(), ["r1", "r2"].as_ref()),
        Column::new("_deleted".into(), [true, false].as_ref()),
        Column::new("_modified_at".into(), [99_i64, 10].as_ref()),
        Column::new("amount".into(), [0_i64, 100].as_ref()),
    ])?;

    let params = DeleteOperationParams {
        selector: Some("amount = 0".to_string()),
    };
    let selectors = BTreeMap::new();

    let result = execute_delete(frame.lazy(), &params, &selectors)?.collect()?;
    let deleted = result.column("_deleted")?.bool()?;
    let modified = result.column("_modified_at")?.i64()?;

    let deleted_values: Vec<Option<bool>> = deleted.into_iter().collect();
    let modified_values: Vec<Option<i64>> = modified.into_iter().collect();

    assert_eq!(deleted_values, vec![Some(true), Some(false)]);
    assert_eq!(modified_values, vec![Some(99), Some(10)]);
    Ok(())
}

#[test]
fn test_delete_without_selector_marks_all_active_rows() -> Result<()> {
    let frame = DataFrame::new(vec![
        Column::new("_row_id".into(), ["r1", "r2", "r3"].as_ref()),
        Column::new("_deleted".into(), [false, true, false].as_ref()),
        Column::new("_modified_at".into(), [10_i64, 99, 10].as_ref()),
        Column::new("amount".into(), [0_i64, 50, 200].as_ref()),
    ])?;

    let params = DeleteOperationParams { selector: None };
    let selectors = BTreeMap::new();

    let result = execute_delete(frame.lazy(), &params, &selectors)?.collect()?;
    let deleted = result.column("_deleted")?.bool()?;
    let deleted_values: Vec<Option<bool>> = deleted.into_iter().collect();

    assert_eq!(deleted_values, vec![Some(true), Some(true), Some(true)]);
    Ok(())
}

#[test]
fn test_delete_no_selector_preserves_already_deleted() -> Result<()> {
    let frame = DataFrame::new(vec![
        Column::new("_row_id".into(), ["r1", "r2"].as_ref()),
        Column::new("_deleted".into(), [true, false].as_ref()),
        Column::new("_modified_at".into(), [99_i64, 10].as_ref()),
        Column::new("amount".into(), [0_i64, 100].as_ref()),
    ])?;

    let params = DeleteOperationParams { selector: None };
    let selectors = BTreeMap::new();

    let result = execute_delete(frame.lazy(), &params, &selectors)?.collect()?;
    let modified = result.column("_modified_at")?.i64()?;
    let modified_values: Vec<Option<i64>> = modified.into_iter().collect();

    assert_eq!(modified_values[0], Some(99));
    assert!(modified_values[1].unwrap_or_default() > 10);
    Ok(())
}
