use anyhow::Result;
use chrono::Utc;
use polars::prelude::*;
use serde::{Deserialize, Serialize};
use std::collections::HashSet;
use std::time::Instant;
use thiserror::Error;
use uuid::Uuid;

use crate::engine::io_traits::OutputWriter;
use crate::model::metadata_store::MetadataStore;
use crate::model::{
    ColumnDef as ModelColumnDef, ColumnType as ModelColumnType, Dataset, DatasetStatus,
    OutputDestination as ModelOutputDestination, TableRef, TemporalMode as ModelTemporalMode,
};

/// Errors that can occur during output operation execution.
#[derive(Debug, Error)]
pub enum OutputError {
    #[error("Selector evaluation failed: {0}")]
    SelectorError(String),

    #[error(
        "Column projection failed: missing columns {missing:?}. Available columns: {available:?}"
    )]
    ColumnProjectionError {
        missing: Vec<String>,
        available: Vec<String>,
    },

    #[error("Column projection cannot be empty")]
    EmptyColumnProjection,

    #[error("Write operation failed: {0}")]
    WriteFailed(#[from] anyhow::Error),

    #[error("Dataset registration failed: {0}")]
    RegistrationFailed(String),

    #[error("Invalid output schema: {0}")]
    InvalidSchema(String),

    #[error("DataFrame is empty (no columns)")]
    EmptyDataFrame,

    #[error("Dataset registration requested but no metadata or registration store provided")]
    MissingMetadataStore,

    #[error("Invalid dataset name: {0}")]
    InvalidDatasetName(String),

    #[error("Invalid output destination: {0}")]
    InvalidDestination(String),

    #[error("Polars error: {0}")]
    PolarsError(#[from] polars::error::PolarsError),
}

fn registration_warning_reason(error: &OutputError) -> &'static str {
    match error {
        OutputError::RegistrationFailed(_) => "registration_backend_error",
        OutputError::InvalidSchema(_) | OutputError::EmptyDataFrame => {
            "invalid_registration_schema"
        }
        _ => "registration_error",
    }
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct OutputWarning {
    pub code: &'static str,
    pub dataset_name: String,
    pub reason: &'static str,
}

pub trait OutputWarningLogger {
    fn warn(&self, warning: OutputWarning);
}

struct StderrOutputWarningLogger;

impl OutputWarningLogger for StderrOutputWarningLogger {
    fn warn(&self, warning: OutputWarning) {
        eprintln!(
            "level=warn code={} dataset_name={} reason={}",
            warning.code, warning.dataset_name, warning.reason
        );
    }
}

/// Registration adapter for dataset-registration backends.
pub trait DatasetRegistrationStore {
    fn get_dataset_by_name(&self, name: &str) -> Result<Option<Dataset>>;
    fn register_dataset(&self, dataset: Dataset) -> Result<Uuid>;
}

struct MetadataStoreRegistrationAdapter<'a> {
    metadata_store: &'a dyn MetadataStore,
}

impl<'a> DatasetRegistrationStore for MetadataStoreRegistrationAdapter<'a> {
    fn get_dataset_by_name(&self, name: &str) -> Result<Option<Dataset>> {
        self.metadata_store.get_dataset_by_name(name)
    }

    fn register_dataset(&self, dataset: Dataset) -> Result<Uuid> {
        self.metadata_store.register_dataset(dataset)
    }
}

/// Configuration for an output operation.
#[derive(Debug, Clone, PartialEq)]
pub struct OutputOperation {
    pub destination: ModelOutputDestination,
    pub selector: Option<Expr>,
    pub columns: Option<Vec<String>>,
    pub include_deleted: bool,
    pub register_as_dataset: Option<String>,
}

/// Result of executing an output operation.
#[derive(Debug, Clone, PartialEq)]
pub struct OutputResult {
    pub rows_written: usize,
    pub columns_written: Vec<String>,
    pub dataset_id: Option<Uuid>,
    pub write_duration_ms: u64,
}

/// Schema extracted from output DataFrame.
#[derive(Debug, Clone, PartialEq)]
pub struct OutputSchema {
    pub columns: Vec<ColumnDef>,
    pub temporal_mode: TemporalMode,
}

/// Definition of a single column.
#[derive(Debug, Clone, PartialEq)]
pub struct ColumnDef {
    pub name: String,
    pub data_type: ColumnType,
    pub nullable: bool,
}

/// Temporal mode for datasets.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
#[serde(rename_all = "lowercase")]
pub enum TemporalMode {
    None,
    Period,
    Bitemporal,
}

/// Column data types.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
#[serde(rename_all = "lowercase")]
pub enum ColumnType {
    String,
    Integer,
    Decimal,
    Date,
    DateTime,
    Boolean,
    Uuid,
}

fn validate_columns(schema: &Schema, columns: &[String]) -> Result<(), OutputError> {
    if columns.is_empty() {
        return Err(OutputError::EmptyColumnProjection);
    }

    let available: Vec<String> = schema.iter_names().map(|name| name.to_string()).collect();
    let available_set: HashSet<&str> = available.iter().map(String::as_str).collect();

    let missing: Vec<String> = columns
        .iter()
        .filter(|column| !available_set.contains(column.as_str()))
        .cloned()
        .collect();

    if missing.is_empty() {
        Ok(())
    } else {
        Err(OutputError::ColumnProjectionError { missing, available })
    }
}

fn validate_selector(working_dataset: &LazyFrame, selector: &Expr) -> Result<(), OutputError> {
    let selector_schema = working_dataset
        .clone()
        .select([selector.clone().alias("__selector__")])
        .collect_schema()
        .map_err(|error| OutputError::SelectorError(error.to_string()))?;

    let selector_type = selector_schema.get("__selector__").ok_or_else(|| {
        OutputError::SelectorError(
            "selector validation failed: selector output column was not found".to_string(),
        )
    })?;

    if selector_type == &DataType::Boolean {
        Ok(())
    } else {
        Err(OutputError::SelectorError(format!(
            "selector expression must evaluate to boolean, got {selector_type:?}"
        )))
    }
}

fn validate_destination(destination: &ModelOutputDestination) -> Result<(), OutputError> {
    match destination {
        ModelOutputDestination::Table {
            datasource_id,
            table,
            ..
        } => {
            if datasource_id.trim().is_empty() {
                return Err(OutputError::InvalidDestination(
                    "table destination requires non-empty datasource_id".to_string(),
                ));
            }
            if table.trim().is_empty() {
                return Err(OutputError::InvalidDestination(
                    "table destination requires non-empty table".to_string(),
                ));
            }
            Ok(())
        }
        ModelOutputDestination::Location { path } => {
            if path.trim().is_empty() {
                return Err(OutputError::InvalidDestination(
                    "location destination requires non-empty path".to_string(),
                ));
            }
            Ok(())
        }
    }
}

fn destination_table_name(destination: &ModelOutputDestination) -> String {
    match destination {
        ModelOutputDestination::Table { table, .. } => table.clone(),
        ModelOutputDestination::Location { path } => path.clone(),
    }
}

fn map_polars_type(dtype: &DataType) -> Result<ColumnType, OutputError> {
    let mapped = if dtype.is_string() || dtype.is_categorical() || dtype.is_enum() {
        ColumnType::String
    } else if dtype.is_integer() {
        ColumnType::Integer
    } else if dtype.is_float() || dtype.is_decimal() {
        ColumnType::Decimal
    } else {
        match dtype {
            DataType::Date => ColumnType::Date,
            DataType::Datetime(_, _) => ColumnType::DateTime,
            DataType::Boolean => ColumnType::Boolean,
            _ => {
                return Err(OutputError::InvalidSchema(format!(
                    "unsupported polars data type for dataset registration: {dtype:?}"
                )))
            }
        }
    };
    Ok(mapped)
}

fn to_model_column_type(column_type: &ColumnType) -> ModelColumnType {
    match column_type {
        ColumnType::String => ModelColumnType::String,
        ColumnType::Integer => ModelColumnType::Integer,
        ColumnType::Decimal => ModelColumnType::Decimal,
        ColumnType::Date => ModelColumnType::Date,
        ColumnType::DateTime => ModelColumnType::Timestamp,
        ColumnType::Boolean => ModelColumnType::Boolean,
        ColumnType::Uuid => ModelColumnType::String,
    }
}

fn to_model_temporal_mode(mode: &TemporalMode) -> Option<ModelTemporalMode> {
    match mode {
        TemporalMode::None => None,
        TemporalMode::Period => Some(ModelTemporalMode::Period),
        TemporalMode::Bitemporal => Some(ModelTemporalMode::Bitemporal),
    }
}

/// Extract schema information from a DataFrame for dataset registration.
pub fn extract_schema(df: &DataFrame) -> Result<OutputSchema, OutputError> {
    let schema = df.schema();
    if schema.is_empty() {
        return Err(OutputError::EmptyDataFrame);
    }

    let columns: Vec<ColumnDef> = schema
        .iter()
        .map(|(name, dtype)| {
            Ok(ColumnDef {
                name: name.to_string(),
                data_type: map_polars_type(dtype)?,
                nullable: true,
            })
        })
        .collect::<std::result::Result<Vec<_>, OutputError>>()?;

    let names: HashSet<&str> = columns.iter().map(|column| column.name.as_str()).collect();
    let temporal_mode = if names.contains("_period") {
        TemporalMode::Period
    } else if names.contains("_valid_from") && names.contains("_valid_to") {
        TemporalMode::Bitemporal
    } else {
        TemporalMode::None
    };

    Ok(OutputSchema {
        columns,
        temporal_mode,
    })
}

fn register_dataset(
    register_name: &str,
    destination: &ModelOutputDestination,
    output_df: &DataFrame,
    registration_store: &dyn DatasetRegistrationStore,
) -> Result<Uuid, OutputError> {
    let schema = extract_schema(output_df)?;
    if schema.columns.is_empty() {
        return Err(OutputError::InvalidSchema(
            "registered dataset must have at least one column".to_string(),
        ));
    }

    let current_timestamp = Utc::now().to_rfc3339();
    let existing = registration_store
        .get_dataset_by_name(register_name)
        .map_err(|error| OutputError::RegistrationFailed(error.to_string()))?;
    let next_version = existing.as_ref().map_or(1, |dataset| dataset.version + 1);

    let model_columns: Vec<ModelColumnDef> = schema
        .columns
        .iter()
        .map(|column| ModelColumnDef {
            name: column.name.clone(),
            column_type: to_model_column_type(&column.data_type),
            nullable: Some(column.nullable),
            description: None,
        })
        .collect();

    let dataset = Dataset {
        id: Uuid::now_v7(),
        name: register_name.to_string(),
        description: Some("Generated by output operation".to_string()),
        owner: "system".to_string(),
        version: next_version,
        status: DatasetStatus::Active,
        resolver_id: existing.as_ref().and_then(|item| item.resolver_id.clone()),
        main_table: TableRef {
            name: destination_table_name(destination),
            temporal_mode: to_model_temporal_mode(&schema.temporal_mode),
            columns: model_columns,
        },
        lookups: Vec::new(),
        natural_key_columns: Vec::new(),
        created_at: Some(current_timestamp.clone()),
        updated_at: Some(current_timestamp),
    };

    registration_store
        .register_dataset(dataset)
        .map_err(|error| OutputError::RegistrationFailed(error.to_string()))
}

/// Execute an output operation using selector filtering, deleted row rules, projection,
/// write dispatch, and optional post-write dataset registration.
pub fn execute_output(
    working_dataset: &LazyFrame,
    operation: &OutputOperation,
    output_writer: &dyn OutputWriter,
    metadata_store: Option<&dyn MetadataStore>,
) -> Result<OutputResult, OutputError> {
    execute_output_with_registry(working_dataset, operation, output_writer, metadata_store)
}

/// Execute an output operation using metadata context only.
///
/// If the provided `MetadataStore` supports dataset registration, this path
/// performs registration after a successful write.
pub fn execute_output_with_registry(
    working_dataset: &LazyFrame,
    operation: &OutputOperation,
    output_writer: &dyn OutputWriter,
    metadata_store: Option<&dyn MetadataStore>,
) -> Result<OutputResult, OutputError> {
    let warning_logger = StderrOutputWarningLogger;
    execute_output_with_registry_and_warning_logger(
        working_dataset,
        operation,
        output_writer,
        metadata_store,
        &warning_logger,
    )
}

pub fn execute_output_with_registry_and_warning_logger(
    working_dataset: &LazyFrame,
    operation: &OutputOperation,
    output_writer: &dyn OutputWriter,
    metadata_store: Option<&dyn MetadataStore>,
    warning_logger: &dyn OutputWarningLogger,
) -> Result<OutputResult, OutputError> {
    let metadata_registration_adapter =
        metadata_store.map(|store| MetadataStoreRegistrationAdapter {
            metadata_store: store,
        });

    execute_output_with_registration_store_and_warning_logger(
        working_dataset,
        operation,
        output_writer,
        metadata_store,
        metadata_registration_adapter
            .as_ref()
            .map(|adapter| adapter as &dyn DatasetRegistrationStore),
        warning_logger,
    )
}

/// Execute an output operation with explicit dataset registration adapter support.
pub fn execute_output_with_registration_store(
    working_dataset: &LazyFrame,
    operation: &OutputOperation,
    output_writer: &dyn OutputWriter,
    metadata_store: Option<&dyn MetadataStore>,
    registration_store: Option<&dyn DatasetRegistrationStore>,
) -> Result<OutputResult, OutputError> {
    let warning_logger = StderrOutputWarningLogger;
    execute_output_with_registration_store_and_warning_logger(
        working_dataset,
        operation,
        output_writer,
        metadata_store,
        registration_store,
        &warning_logger,
    )
}

pub fn execute_output_with_registration_store_and_warning_logger(
    working_dataset: &LazyFrame,
    operation: &OutputOperation,
    output_writer: &dyn OutputWriter,
    metadata_store: Option<&dyn MetadataStore>,
    registration_store: Option<&dyn DatasetRegistrationStore>,
    warning_logger: &dyn OutputWarningLogger,
) -> Result<OutputResult, OutputError> {
    let source_schema = working_dataset.clone().collect_schema()?;

    let mut output_frame = working_dataset.clone();

    if let Some(selector) = operation.selector.as_ref() {
        validate_selector(working_dataset, selector)?;
        output_frame = output_frame.filter(selector.clone());
    }

    let has_deleted_column = source_schema.iter_names().any(|name| name == "_deleted");
    if !operation.include_deleted && has_deleted_column {
        output_frame =
            output_frame.filter(col("_deleted").neq(lit(true)).or(col("_deleted").is_null()));
    }

    if let Some(columns) = operation.columns.as_ref() {
        validate_columns(source_schema.as_ref(), columns)?;
        let projected_columns: Vec<Expr> = columns.iter().map(|name| col(name.as_str())).collect();
        output_frame = output_frame.select(projected_columns);
    }

    let registration_name = if let Some(register_name) = operation.register_as_dataset.as_deref() {
        if register_name.trim().is_empty() {
            return Err(OutputError::InvalidDatasetName(
                "register_as_dataset cannot be empty".to_string(),
            ));
        }

        if metadata_store.is_none() && registration_store.is_none() {
            return Err(OutputError::MissingMetadataStore);
        }
        Some(register_name)
    } else {
        None
    };

    validate_destination(&operation.destination)?;

    let output_df = output_frame.collect().map_err(OutputError::PolarsError)?;

    let write_start = Instant::now();
    output_writer
        .write(&output_df, &operation.destination)
        .map_err(OutputError::WriteFailed)?;
    let write_duration_ms = write_start.elapsed().as_millis() as u64;

    let columns_written: Vec<String> = output_df
        .get_column_names()
        .iter()
        .map(|name| name.to_string())
        .collect();
    let rows_written = output_df.height();

    let mut dataset_id = None;
    if let Some(register_name) = registration_name {
        if let Some(registration_store) = registration_store {
            match register_dataset(
                register_name,
                &operation.destination,
                &output_df,
                registration_store,
            ) {
                Ok(registered_id) => {
                    dataset_id = Some(registered_id);
                }
                Err(error) => {
                    let warning = OutputWarning {
                        code: "output.registration.failed",
                        dataset_name: register_name.to_string(),
                        reason: registration_warning_reason(&error),
                    };
                    warning_logger.warn(warning);
                }
            }
        } else if let Some(metadata_store) = metadata_store {
            let metadata_registration_adapter = MetadataStoreRegistrationAdapter { metadata_store };
            match register_dataset(
                register_name,
                &operation.destination,
                &output_df,
                &metadata_registration_adapter,
            ) {
                Ok(registered_id) => {
                    dataset_id = Some(registered_id);
                }
                Err(error) => {
                    let warning = OutputWarning {
                        code: "output.registration.failed",
                        dataset_name: register_name.to_string(),
                        reason: registration_warning_reason(&error),
                    };
                    warning_logger.warn(warning);
                }
            }
        }
    }

    Ok(OutputResult {
        rows_written,
        columns_written,
        dataset_id,
        write_duration_ms,
    })
}
