// Contract test for TS-07: Column Projection
// Test Scenario TS-07 from sample-datasets.md

use anyhow::Result;
use dobo_core::engine::io_traits::{OutputWriter, OutputWriterError};
use dobo_core::engine::ops::{execute_output, OutputOperation};
use dobo_core::model::OutputDestination;
use polars::prelude::*;
use std::sync::Mutex;

// Mock OutputWriter for contract testing
struct MockOutputWriter {
    pub written_data: Mutex<Vec<DataFrame>>,
}

impl MockOutputWriter {
    fn new() -> Self {
        Self {
            written_data: Mutex::new(Vec::new()),
        }
    }

    fn get_written_data(&self) -> Vec<DataFrame> {
        self.written_data.lock().unwrap().clone()
    }
}

impl OutputWriter for MockOutputWriter {
    fn write(
        &self,
        frame: &DataFrame,
        _destination: &OutputDestination,
    ) -> std::result::Result<(), OutputWriterError> {
        self.written_data.lock().unwrap().push(frame.clone());
        Ok(())
    }
}

/// T023: Contract test for TS-07 column projection
///
/// Test Scenario: Given a GL transactions dataset, execute output operation
/// with column projection: [journal_id, account_code, amount_local, amount_reporting]
/// and verify output contains only 4 columns per row in the specified order.
#[test]
fn test_ts07_column_projection_contract() {
    // Create GL transactions dataset (sample data)
    let df = df! {
        "journal_id" => &["J001", "J001", "J002", "J002", "J003", "J003", "J004", "J004", "J005", "J005"],
        "line_number" => &[1, 2, 1, 2, 1, 2, 1, 2, 1, 2],
        "posting_date" => &["2024-01-15", "2024-01-15", "2024-01-16", "2024-01-16", "2024-01-17", "2024-01-17", "2024-01-18", "2024-01-18", "2024-01-19", "2024-01-19"],
        "account_code" => &["1000", "2000", "1000", "3000", "1000", "4000", "1000", "5000", "1000", "6000"],
        "amount_local" => &[1000.0, -1000.0, 2000.0, -2000.0, 3000.0, -3000.0, 4000.0, -4000.0, 5000.0, -5000.0],
        "amount_reporting" => &[1100.0, -1100.0, 2200.0, -2200.0, 3300.0, -3300.0, 4400.0, -4400.0, 5500.0, -5500.0],
    }
    .unwrap();

    let working_dataset = df.lazy();

    // Configure output operation with column projection
    let operation = OutputOperation {
        destination: OutputDestination::Location {
            path: "gl_output.csv".to_string(),
        },
        selector: None,
        columns: Some(vec![
            "journal_id".to_string(),
            "account_code".to_string(),
            "amount_local".to_string(),
            "amount_reporting".to_string(),
        ]),
        include_deleted: false,
        register_as_dataset: None,
    };

    let writer = MockOutputWriter::new();

    // Execute output operation
    let result = execute_output(&working_dataset, &operation, &writer, None).unwrap();

    // Verify result metrics
    assert_eq!(result.rows_written, 10, "All 10 rows should be written");
    assert_eq!(
        result.columns_written,
        vec![
            "journal_id",
            "account_code",
            "amount_local",
            "amount_reporting"
        ],
        "Only 4 columns should be written in specified order"
    );

    // Verify actual output data
    let written_data = writer.get_written_data();
    assert_eq!(written_data.len(), 1, "Should write once");

    let output_df = &written_data[0];
    assert_eq!(output_df.height(), 10, "Should have 10 rows");
    assert_eq!(output_df.width(), 4, "Should have exactly 4 columns");

    // Verify column names and order
    let column_names = output_df.get_column_names();
    assert_eq!(
        column_names,
        vec![
            "journal_id",
            "account_code",
            "amount_local",
            "amount_reporting"
        ],
        "Columns should be in specified order"
    );

    // Verify first row data (spot check)
    let journal_id_col = output_df.column("journal_id").unwrap();
    let journal_id_series = journal_id_col.str().unwrap();
    assert_eq!(journal_id_series.get(0).unwrap(), "J001");

    let amount_local_col = output_df.column("amount_local").unwrap();
    let amount_local_series = amount_local_col.f64().unwrap();
    assert_eq!(amount_local_series.get(0).unwrap(), 1000.0);

    println!("✓ TS-07 Contract Test Passed: Column projection works correctly");
}
