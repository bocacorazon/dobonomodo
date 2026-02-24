use thiserror::Error;
use uuid::Uuid;

#[derive(Debug, Error, PartialEq, Eq)]
pub enum AppendError {
    #[error("dataset not found: {dataset_id}")]
    DatasetNotFound { dataset_id: Uuid },
    #[error("dataset version not found: {dataset_id} version {version}")]
    DatasetVersionNotFound { dataset_id: Uuid, version: i32 },
    #[error("metadata access error for {entity}: {message}")]
    MetadataAccessError { entity: String, message: String },
    #[error("column mismatch; extra columns: {extra_columns:?}")]
    ColumnMismatch { extra_columns: Vec<String> },
    #[error("failed to parse expression '{expression}': {error}")]
    ExpressionParseError { expression: String, error: String },
    #[error("aggregation error: {message}")]
    AggregationError { message: String },
    #[error("column not found: {column} ({context})")]
    ColumnNotFound { column: String, context: String },
    #[error("resolver not found for dataset: {dataset_id}")]
    ResolverNotFound { dataset_id: Uuid },
    #[error("failed to load source data: {message}")]
    DataLoadError { message: String },
}
