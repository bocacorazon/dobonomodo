use anyhow::Result;
use polars::prelude::{DataFrame, LazyFrame};

use crate::model::{OutputDestination, ResolvedLocation, TableRef};

pub trait DataLoader {
    fn load(&self, location: &ResolvedLocation, schema: &TableRef) -> Result<LazyFrame>;
}

pub trait OutputWriter {
    fn write(&self, frame: &DataFrame, destination: &OutputDestination) -> Result<()>;
}
