use std::collections::HashMap;

use anyhow::{anyhow, Result};
use dobo_core::model::{ResolvedLocation, TableRef};
use dobo_core::DataLoader;
use polars::prelude::{DataFrame, IntoLazy, LazyFrame};

pub fn resolver_name() -> &'static str {
    "test-resolver"
}

#[derive(Default)]
pub struct InMemoryDataLoader {
    tables: HashMap<String, DataFrame>,
}

impl InMemoryDataLoader {
    pub fn new() -> Self {
        Self::default()
    }

    pub fn seed_table(&mut self, key: impl Into<String>, frame: DataFrame) {
        self.tables.insert(key.into(), frame);
    }
}

impl DataLoader for InMemoryDataLoader {
    fn load(&self, location: &ResolvedLocation, _schema: &TableRef) -> Result<LazyFrame> {
        let key = location
            .path
            .clone()
            .or_else(|| location.table.clone())
            .ok_or_else(|| anyhow!("resolved location is missing path/table"))?;

        let frame = self
            .tables
            .get(&key)
            .ok_or_else(|| anyhow!("no seeded table for key '{key}'"))?;

        Ok(frame.clone().lazy())
    }
}
