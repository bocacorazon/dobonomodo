use std::collections::BTreeMap;
use std::time::Instant;

use anyhow::Result;
use dobo_core::model::DeleteOperationParams;
use dobo_core::operations::delete::execute_delete;
use polars::prelude::{Column, DataFrame, IntoLazy};

#[test]
fn benchmark_delete_operation_sizes() -> Result<()> {
    for size in [10_000_usize, 100_000, 1_000_000] {
        let frame = benchmark_frame(size)?;
        let params = DeleteOperationParams {
            selector: Some("amount >= 0".to_string()),
        };

        let started = Instant::now();
        let result = execute_delete(frame.lazy(), &params, &BTreeMap::new())?.collect()?;
        let elapsed = started.elapsed();

        let deleted = result.column("_deleted")?.bool()?;
        assert_eq!(deleted.get(0), Some(true));
        assert_eq!(result.height(), size);
        eprintln!(
            "delete benchmark size={size} elapsed_ms={}",
            elapsed.as_millis()
        );
    }

    Ok(())
}

fn benchmark_frame(size: usize) -> Result<DataFrame> {
    let row_ids: Vec<i64> = (0..size as i64).collect();
    let deleted: Vec<bool> = vec![false; size];
    let modified_at: Vec<i64> = vec![1; size];
    let amount: Vec<i64> = (0..size as i64).collect();

    DataFrame::new(vec![
        Column::new("_row_id".into(), row_ids.as_slice()),
        Column::new("_deleted".into(), deleted.as_slice()),
        Column::new("_modified_at".into(), modified_at.as_slice()),
        Column::new("amount".into(), amount.as_slice()),
    ])
    .map_err(Into::into)
}
