use std::fs;
use std::path::PathBuf;

use chrono::{TimeZone, Utc};
use dobo_core::engine::ops::UpdateOperation;
use dobo_core::engine::types::execute_update_scenario;
use polars::prelude::{DataFrame, LazyFrame};
use std::collections::HashMap;

#[allow(dead_code)]
pub fn fixture_path(file_name: &str) -> PathBuf {
    PathBuf::from(env!("CARGO_MANIFEST_DIR"))
        .join("tests")
        .join("fixtures")
        .join(file_name)
}

#[allow(dead_code)]
pub fn read_fixture(file_name: &str) -> String {
    let path = fixture_path(file_name);
    fs::read_to_string(path).expect("fixture should be readable")
}

#[allow(dead_code)]
pub struct UpdateScenarioHarness {
    selectors: HashMap<String, String>,
}

impl UpdateScenarioHarness {
    #[allow(dead_code)]
    pub fn new(selectors: HashMap<String, String>) -> Self {
        Self { selectors }
    }

    #[allow(dead_code)]
    pub fn run_update_operation(
        &self,
        operation: UpdateOperation,
        working_dataset: LazyFrame,
    ) -> DataFrame {
        let run_timestamp = Utc
            .timestamp_opt(1_700_000_000, 0)
            .single()
            .expect("timestamp");
        execute_update_scenario(
            &operation,
            working_dataset,
            self.selectors.clone(),
            run_timestamp,
        )
        .expect("execute scenario update")
    }
}
