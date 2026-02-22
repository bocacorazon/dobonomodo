use polars::error::PolarsError;
use std::path::PathBuf;
use thiserror::Error;
use uuid::Uuid;

#[derive(Debug, Error)]
pub enum LoaderError {
    #[error("Failed to create DataFrame from rows")]
    DataFrameBuild {
        #[source]
        source: PolarsError,
    },
    #[error("Failed to load CSV data from '{path}'")]
    CsvLoad {
        path: PathBuf,
        #[source]
        source: PolarsError,
    },
    #[error("Failed to load Parquet data from '{path}'")]
    ParquetLoad {
        path: PathBuf,
        #[source]
        source: PolarsError,
    },
    #[error("Table '{table}' not found in test data (available: {available:?})")]
    TableNotFound {
        table: String,
        available: Vec<String>,
    },
}

#[derive(Debug, Error)]
pub enum MetadataError {
    #[error("Dataset {id} not found in test metadata store")]
    DatasetNotFound { id: Uuid },
    #[error("Dataset {id} version mismatch: requested {requested}, found {found}")]
    DatasetVersionMismatch {
        id: Uuid,
        requested: i32,
        found: i32,
    },
    #[error("Project {id} not found in test metadata store")]
    ProjectNotFound { id: Uuid },
    #[error("Resolver '{id}' not found in test metadata store")]
    ResolverNotFound { id: String },
}

#[derive(Debug, Error)]
pub enum TraceError {
    #[error("Failed to lock trace events mutex: {message}")]
    LockPoisoned { message: String },
}

#[derive(Debug, Error)]
pub enum InjectionError {
    #[error("Table '{table}' row {row_index} is missing required temporal column '{column}'")]
    MissingTemporalColumn {
        table: String,
        row_index: usize,
        column: &'static str,
    },
}
