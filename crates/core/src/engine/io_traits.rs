use polars::prelude::{DataFrame, LazyFrame};
use thiserror::Error;

use crate::model::{OutputDestination, ResolvedLocation, TableRef};

#[derive(Debug, Error, Clone, PartialEq, Eq)]
pub enum DataLoaderError {
    #[error("data load failed: {message}")]
    LoadFailed { message: String },
}

#[derive(Debug, Error, Clone, PartialEq, Eq)]
pub enum OutputWriterError {
    #[error("output write failed: {message}")]
    WriteFailed { message: String },
}

pub trait DataLoader {
    fn load(
        &self,
        location: &ResolvedLocation,
        schema: &TableRef,
    ) -> std::result::Result<LazyFrame, DataLoaderError>;
}

pub trait OutputWriter {
    fn write(
        &self,
        frame: &DataFrame,
        destination: &OutputDestination,
    ) -> std::result::Result<(), OutputWriterError>;
}
