use dobo_core::model::ColumnType;
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
    #[error("Failed to inspect schema for table '{table}'")]
    SchemaInspection {
        table: String,
        #[source]
        source: PolarsError,
    },
    #[error("Table '{table}' is missing required columns: {missing:?}")]
    MissingColumns { table: String, missing: Vec<String> },
    #[error("Table '{table}' contains unexpected columns: {unexpected:?}")]
    UnexpectedColumns {
        table: String,
        unexpected: Vec<String>,
    },
    #[error("Table '{table}' column '{column}' cannot be cast to '{expected_type:?}'")]
    TypeMismatch {
        table: String,
        column: String,
        expected_type: ColumnType,
        #[source]
        source: PolarsError,
    },
    #[error(
        "Inline data type mismatch at row {row_index}, column '{column}': expected {expected_type}, got {actual_type}"
    )]
    InlineValueTypeMismatch {
        row_index: usize,
        column: String,
        expected_type: &'static str,
        actual_type: &'static str,
    },
    #[error(
        "Inline data contains unsupported value type at row {row_index}, column '{column}': {actual_type}"
    )]
    InlineUnsupportedValueType {
        row_index: usize,
        column: String,
        actual_type: &'static str,
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
    #[error("Failed to lock metadata store state: {message}")]
    LockPoisoned { message: String },
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
