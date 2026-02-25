use std::collections::BTreeMap;

use anyhow::Result;
use dobo_core::model::OutputOperationParams;
use dobo_core::operations::output::execute_output;
use polars::prelude::{Column, DataFrame, IntoLazy};

fn base_frame() -> Result<DataFrame> {
    DataFrame::new(vec![
        Column::new("_row_id".into(), ["r1", "r2", "r3"].as_ref()),
        Column::new("_deleted".into(), [true, false, false].as_ref()),
        Column::new("_modified_at".into(), [10_i64, 10, 10].as_ref()),
        Column::new("amount".into(), [0_i64, 100, 200].as_ref()),
    ])
    .map_err(Into::into)
}

#[test]
fn test_output_excludes_deleted_rows_by_default() -> Result<()> {
    let frame = base_frame()?;
    let params = OutputOperationParams::default();
    let selectors = BTreeMap::new();

    let result = execute_output(frame.lazy(), &params, &selectors)?.collect()?;

    assert_eq!(result.height(), 2);
    let deleted = result.column("_deleted")?.bool()?;
    assert!(deleted.into_iter().all(|value| value == Some(false)));
    Ok(())
}

#[test]
fn test_output_includes_deleted_when_requested() -> Result<()> {
    let frame = base_frame()?;
    let params = OutputOperationParams {
        include_deleted: true,
        ..OutputOperationParams::default()
    };
    let selectors = BTreeMap::new();

    let result = execute_output(frame.lazy(), &params, &selectors)?.collect()?;

    assert_eq!(result.height(), 3);
    let deleted = result.column("_deleted")?.bool()?;
    assert!(deleted.into_iter().any(|value| value == Some(true)));
    Ok(())
}
