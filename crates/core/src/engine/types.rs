use std::collections::HashMap;

use chrono::{DateTime, Utc};
use polars::prelude::{DataFrame, LazyFrame, PolarsError};
use thiserror::Error;

use super::ops::{execute_update, UpdateError, UpdateExecutionContext, UpdateOperation};
pub use crate::model::TableRef;

#[derive(Debug, Error)]
pub enum ScenarioHarnessError {
    #[error("Failed to execute update operation: {0}")]
    Update(#[from] UpdateError),
    #[error("Failed to collect scenario output: {0}")]
    Collect(#[from] PolarsError),
}

pub fn execute_update_scenario(
    operation: &UpdateOperation,
    working_dataset: LazyFrame,
    selectors: HashMap<String, String>,
    run_timestamp: DateTime<Utc>,
) -> Result<DataFrame, ScenarioHarnessError> {
    let context = UpdateExecutionContext {
        working_dataset,
        selectors,
        run_timestamp,
    };
    let output = execute_update(&context, operation)?;
    output.collect().map_err(ScenarioHarnessError::from)
}
