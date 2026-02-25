use dobo_core::engine::io_traits::{OutputWriter, OutputWriterError};
use dobo_core::engine::ops::output::{execute_output, OutputOperation};
use dobo_core::model::OutputDestination;
use polars::prelude::*;
use std::time::Instant;

struct NoopWriter;

impl OutputWriter for NoopWriter {
    fn write(
        &self,
        _frame: &DataFrame,
        _destination: &OutputDestination,
    ) -> std::result::Result<(), OutputWriterError> {
        Ok(())
    }
}

#[test]
#[ignore]
fn benchmark_large_output_execution() {
    let row_count = 100_000;
    let ids: Vec<i64> = (0..row_count).map(|value| value as i64).collect();
    let amounts: Vec<i64> = (0..row_count).map(|value| value as i64 * 10).collect();

    let df = df! {
        "id" => ids,
        "amount" => amounts,
    }
    .unwrap();

    let operation = OutputOperation {
        destination: OutputDestination::Location {
            path: "memory://benchmark".to_string(),
        },
        selector: Some(col("amount").gt(lit(100))),
        columns: Some(vec!["id".to_string(), "amount".to_string()]),
        include_deleted: true,
        register_as_dataset: None,
    };

    let start = Instant::now();
    let result = execute_output(&df.lazy(), &operation, &NoopWriter, None).unwrap();
    assert!(result.rows_written > 0);
    assert!(start.elapsed().as_millis() > 0);
}
