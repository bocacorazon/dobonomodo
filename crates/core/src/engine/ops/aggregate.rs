use chrono::{DateTime, Utc};
use polars::prelude::*;
use serde::{Deserialize, Serialize};
use std::collections::HashSet;
use uuid::Uuid;

use crate::model::expression::Expression;

const SYSTEM_COLUMNS: [&str; 7] = [
    "_row_id",
    "_created_at",
    "_updated_at",
    "_source_dataset_id",
    "_source_table",
    "_deleted",
    "_period",
];

/// Runtime metadata used when adding summary rows.
#[derive(Debug, Clone)]
pub struct ExecutionContext {
    pub execution_time: DateTime<Utc>,
    pub source_dataset_id: Uuid,
    pub source_table: String,
}

impl ExecutionContext {
    pub fn new(source_dataset_id: Uuid, source_table: impl Into<String>) -> Self {
        Self {
            execution_time: Utc::now(),
            source_dataset_id,
            source_table: source_table.into(),
        }
    }
}

/// Configuration for an aggregate operation that groups rows and computes summary values.
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
pub struct AggregateOperation {
    /// Columns to group by.
    pub group_by: Vec<String>,
    /// List of aggregation expressions to compute.
    pub aggregations: Vec<Aggregation>,
    /// Optional selector to filter rows before aggregation.
    #[serde(default)]
    pub selector: Option<serde_json::Value>,
}

/// Single aggregation expression to compute during aggregate operation.
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
pub struct Aggregation {
    /// Output column name for the aggregated result.
    pub column: String,
    /// Aggregate expression (for example, `SUM(amount)` or `COUNT(*)`).
    pub expression: Expression,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
enum AggregateFunction {
    Sum,
    Count,
    Avg,
    MinAgg,
    MaxAgg,
}

/// Errors that can occur during aggregate operations.
#[derive(Debug, Clone, PartialEq, Eq, thiserror::Error)]
pub enum AggregateError {
    /// Empty group_by list.
    #[error("group_by list cannot be empty")]
    EmptyGroupBy,
    /// Empty aggregations list.
    #[error("aggregations list cannot be empty")]
    EmptyAggregations,
    /// Duplicate column in group_by.
    #[error("duplicate group_by column: {0}")]
    DuplicateGroupByColumn(String),
    /// Unknown column referenced in group_by.
    #[error("unknown column in group_by: {0}")]
    UnknownGroupByColumn(String),
    /// Unknown source column referenced in aggregation expression.
    #[error("unknown column in aggregation expression: {0}")]
    UnknownAggregationColumn(String),
    /// System column conflict in group_by or aggregation output.
    #[error("aggregation output column conflicts with system column: {0}")]
    SystemColumnConflict(String),
    /// Duplicate aggregation output column.
    #[error("duplicate aggregation output column: {0}")]
    DuplicateAggregationColumn(String),
    /// Invalid aggregate expression syntax or function.
    #[error("invalid aggregate expression: {0}")]
    InvalidExpression(String),
    /// Aggregate function used in an invalid expression context.
    #[error("aggregate function not allowed in this context: {0}")]
    InvalidAggregateContext(String),
    /// Invalid identifier in aggregate specification.
    #[error("invalid identifier: {0}")]
    InvalidIdentifier(String),
    /// Error during execution.
    #[error("execution error: {0}")]
    ExecutionError(String),
}

fn normalize_identifier(value: &str, context: &str) -> Result<String, AggregateError> {
    let normalized = value.trim();
    if normalized.is_empty() {
        return Err(AggregateError::InvalidIdentifier(context.to_string()));
    }
    Ok(normalized.to_string())
}

fn normalize_column_reference(reference: &str) -> Result<String, AggregateError> {
    let normalized = normalize_identifier(reference, "column reference")?;
    if let Some((_, column)) = normalized.rsplit_once('.') {
        normalize_identifier(column, "column reference")
    } else {
        Ok(normalized)
    }
}

fn resolve_column_reference(reference: &str, schema: &SchemaRef) -> Option<String> {
    let normalized = reference.trim();
    if normalized.is_empty() {
        return None;
    }
    if schema.get(normalized).is_some() {
        return Some(normalized.to_string());
    }
    normalized.rsplit_once('.').and_then(|(_, column)| {
        let column = column.trim();
        (!column.is_empty() && schema.get(column).is_some()).then(|| column.to_string())
    })
}

/// Validate operation shape without schema context.
pub fn validate_aggregate_spec(spec: &AggregateOperation) -> Result<(), AggregateError> {
    if spec.group_by.is_empty() {
        return Err(AggregateError::EmptyGroupBy);
    }
    if spec.aggregations.is_empty() {
        return Err(AggregateError::EmptyAggregations);
    }

    let mut seen_group_by: HashSet<String> = HashSet::new();
    for column in &spec.group_by {
        let normalized = normalize_identifier(column, "group_by column")?;
        if SYSTEM_COLUMNS.contains(&normalized.as_str()) && normalized != "_period" {
            return Err(AggregateError::SystemColumnConflict(normalized));
        }
        if !seen_group_by.insert(normalized.clone()) {
            return Err(AggregateError::DuplicateGroupByColumn(normalized));
        }
    }

    let mut seen_output_columns: HashSet<String> = HashSet::new();
    for aggregation in &spec.aggregations {
        let normalized = normalize_identifier(&aggregation.column, "aggregation output column")?;
        if SYSTEM_COLUMNS.contains(&normalized.as_str()) {
            return Err(AggregateError::SystemColumnConflict(normalized));
        }
        if !seen_output_columns.insert(normalized.clone()) {
            return Err(AggregateError::DuplicateAggregationColumn(normalized));
        }
    }

    Ok(())
}

/// Validate operation against resolved dataset schema.
pub fn validate_aggregate_compile(
    spec: &AggregateOperation,
    schema: &SchemaRef,
) -> Result<(), AggregateError> {
    for group_by in &spec.group_by {
        if resolve_column_reference(group_by, schema).is_none() {
            return Err(AggregateError::UnknownGroupByColumn(group_by.clone()));
        }
    }

    for aggregation in &spec.aggregations {
        let (_, source_column) = parse_aggregation_expression(&aggregation.expression)?;
        if let Some(source_column) = source_column {
            if resolve_column_reference(&source_column, schema).is_none() {
                return Err(AggregateError::UnknownAggregationColumn(source_column));
            }
        }
    }

    Ok(())
}

pub fn convert_group_by_to_polars_exprs(group_by: &[String]) -> Vec<Expr> {
    group_by.iter().map(col).collect()
}

pub fn convert_aggregations_to_polars_exprs(
    aggregations: &[Aggregation],
) -> Result<Vec<Expr>, AggregateError> {
    aggregations
        .iter()
        .map(|aggregation| {
            let (function, source_column) = parse_aggregation_expression(&aggregation.expression)?;
            let source_column = source_column
                .as_deref()
                .map(normalize_column_reference)
                .transpose()?;
            let expression = match function {
                AggregateFunction::Sum => col(required_source_column(&source_column)?).sum(),
                AggregateFunction::Count => match source_column {
                    Some(column) => col(column.as_str()).count(),
                    None => len(),
                },
                AggregateFunction::Avg => col(required_source_column(&source_column)?).mean(),
                AggregateFunction::MinAgg => col(required_source_column(&source_column)?).min(),
                AggregateFunction::MaxAgg => col(required_source_column(&source_column)?).max(),
            };

            Ok(expression.alias(&aggregation.column))
        })
        .collect()
}

fn convert_aggregations_to_polars_exprs_resolved(
    aggregations: &[Aggregation],
    schema: &SchemaRef,
) -> Result<Vec<Expr>, AggregateError> {
    aggregations
        .iter()
        .map(|aggregation| {
            let (function, source_column) = parse_aggregation_expression(&aggregation.expression)?;
            let source_column = source_column
                .as_deref()
                .map(|column| {
                    resolve_column_reference(column, schema)
                        .ok_or_else(|| AggregateError::UnknownAggregationColumn(column.to_string()))
                })
                .transpose()?;
            let expression = match function {
                AggregateFunction::Sum => col(required_source_column(&source_column)?).sum(),
                AggregateFunction::Count => match source_column {
                    Some(column) => col(column.as_str()).count(),
                    None => len(),
                },
                AggregateFunction::Avg => col(required_source_column(&source_column)?).mean(),
                AggregateFunction::MinAgg => col(required_source_column(&source_column)?).min(),
                AggregateFunction::MaxAgg => col(required_source_column(&source_column)?).max(),
            };

            Ok(expression.alias(&aggregation.column))
        })
        .collect()
}

fn required_source_column(source_column: &Option<String>) -> Result<&str, AggregateError> {
    source_column
        .as_deref()
        .ok_or_else(|| AggregateError::InvalidExpression("COUNT(*) only valid for COUNT".into()))
}

fn parse_aggregation_expression(
    expression: &Expression,
) -> Result<(AggregateFunction, Option<String>), AggregateError> {
    let source = expression.source.trim();
    if source.is_empty() {
        return Err(AggregateError::InvalidExpression(
            "empty aggregation expression".to_string(),
        ));
    }

    let open = source
        .find('(')
        .ok_or_else(|| AggregateError::InvalidExpression(source.to_string()))?;
    let close = source
        .rfind(')')
        .ok_or_else(|| AggregateError::InvalidExpression(source.to_string()))?;

    let function_name = source[..open].trim().to_ascii_uppercase();
    let function = match function_name.as_str() {
        "SUM" => AggregateFunction::Sum,
        "COUNT" => AggregateFunction::Count,
        "AVG" => AggregateFunction::Avg,
        "MIN_AGG" => AggregateFunction::MinAgg,
        "MAX_AGG" => AggregateFunction::MaxAgg,
        _ => return Err(AggregateError::InvalidExpression(source.to_string())),
    };

    if close != source.len() - 1 {
        return Err(AggregateError::InvalidAggregateContext(function_name));
    }

    let argument = source[open + 1..close].trim();
    if argument.contains('(') || argument.contains(')') {
        return Err(AggregateError::InvalidAggregateContext(function_name));
    }

    let source_column = match function {
        AggregateFunction::Count => {
            if argument == "*" {
                None
            } else if argument.is_empty() {
                return Err(AggregateError::InvalidExpression(source.to_string()));
            } else {
                Some(argument.to_string())
            }
        }
        _ => {
            if argument.is_empty() || argument == "*" {
                return Err(AggregateError::InvalidExpression(source.to_string()));
            }
            Some(argument.to_string())
        }
    };

    Ok((function, source_column))
}

pub fn identify_non_aggregated_columns(
    schema: &SchemaRef,
    spec: &AggregateOperation,
) -> Result<Vec<String>, AggregateError> {
    let grouped: HashSet<String> = spec
        .group_by
        .iter()
        .map(|column| resolve_column_reference(column, schema).unwrap_or_else(|| column.clone()))
        .collect();
    let outputs: HashSet<&str> = spec
        .aggregations
        .iter()
        .map(|a| a.column.as_str())
        .collect();
    let mut inputs: HashSet<String> = HashSet::new();
    for aggregation in &spec.aggregations {
        let (_, source_column) = parse_aggregation_expression(&aggregation.expression)?;
        if let Some(source_column) = source_column {
            if let Some(resolved) = resolve_column_reference(&source_column, schema) {
                inputs.insert(resolved);
            } else {
                inputs.insert(source_column);
            }
        }
    }

    Ok(schema
        .iter_names()
        .filter_map(|name| {
            let value = name.as_str();
            if grouped.contains(value)
                || outputs.contains(value)
                || inputs.contains(value)
                || SYSTEM_COLUMNS.contains(&value)
            {
                None
            } else {
                Some(value.to_string())
            }
        })
        .collect())
}

pub fn add_null_columns_for_non_aggregated(
    mut summary: DataFrame,
    schema: &SchemaRef,
    columns: &[String],
) -> Result<DataFrame, AggregateError> {
    for column_name in columns {
        if summary.column(column_name).is_ok() {
            continue;
        }
        let dtype = schema
            .get(column_name.as_str())
            .cloned()
            .unwrap_or(DataType::Null);
        let series = Series::full_null(column_name.as_str().into(), summary.height(), &dtype);
        summary
            .with_column(series)
            .map_err(|err| AggregateError::ExecutionError(err.to_string()))?;
    }
    Ok(summary)
}

pub fn generate_row_ids(size: usize) -> Vec<String> {
    (0..size).map(|_| Uuid::now_v7().to_string()).collect()
}

pub fn add_system_metadata(
    mut summary: DataFrame,
    context: &ExecutionContext,
) -> Result<DataFrame, AggregateError> {
    let row_count = summary.height();
    let timestamp = context.execution_time.to_rfc3339();

    summary
        .with_column(Series::new("_row_id".into(), generate_row_ids(row_count)))
        .map_err(|err| AggregateError::ExecutionError(err.to_string()))?;
    summary
        .with_column(Series::new(
            "_created_at".into(),
            vec![timestamp.clone(); row_count],
        ))
        .map_err(|err| AggregateError::ExecutionError(err.to_string()))?;
    summary
        .with_column(Series::new(
            "_updated_at".into(),
            vec![timestamp; row_count],
        ))
        .map_err(|err| AggregateError::ExecutionError(err.to_string()))?;
    summary
        .with_column(Series::new(
            "_source_dataset_id".into(),
            vec![context.source_dataset_id.to_string(); row_count],
        ))
        .map_err(|err| AggregateError::ExecutionError(err.to_string()))?;
    summary
        .with_column(Series::new(
            "_source_table".into(),
            vec![context.source_table.clone(); row_count],
        ))
        .map_err(|err| AggregateError::ExecutionError(err.to_string()))?;
    summary
        .with_column(Series::new("_deleted".into(), vec![false; row_count]))
        .map_err(|err| AggregateError::ExecutionError(err.to_string()))?;

    if summary.column("_period").is_err() {
        let null_period = Series::full_null("_period".into(), row_count, &DataType::String);
        summary
            .with_column(null_period)
            .map_err(|err| AggregateError::ExecutionError(err.to_string()))?;
    }

    Ok(summary)
}

fn align_frame_columns(
    mut frame: LazyFrame,
    target_columns: &[(String, DataType)],
) -> Result<LazyFrame, AggregateError> {
    let schema = frame
        .collect_schema()
        .map_err(|err| AggregateError::ExecutionError(err.to_string()))?;

    let expressions = target_columns
        .iter()
        .map(|(column_name, data_type)| {
            if schema.get(column_name.as_str()).is_some() {
                col(column_name.as_str())
                    .cast(data_type.clone())
                    .alias(column_name.as_str())
            } else {
                lit(Null {})
                    .cast(data_type.clone())
                    .alias(column_name.as_str())
            }
        })
        .collect::<Vec<_>>();

    Ok(frame.select(expressions))
}

fn materialize_summary_rows(
    grouped: LazyFrame,
    schema: &SchemaRef,
    spec: &AggregateOperation,
    execution_context: &ExecutionContext,
) -> Result<DataFrame, AggregateError> {
    // Summary metadata injection requires row-wise UUID generation, so this is the explicit
    // execution boundary for the aggregate summary branch.
    let grouped = grouped
        .collect()
        .map_err(|err| AggregateError::ExecutionError(err.to_string()))?;
    let non_aggregated = identify_non_aggregated_columns(schema, spec)?;
    let summary_with_nulls = add_null_columns_for_non_aggregated(grouped, schema, &non_aggregated)?;
    add_system_metadata(summary_with_nulls, execution_context)
}

/// Execute aggregate operation and append summary rows to the working dataset.
pub fn execute_aggregate(
    spec: &AggregateOperation,
    working_dataset: LazyFrame,
    selector: Option<Expr>,
    execution_context: ExecutionContext,
) -> Result<LazyFrame, AggregateError> {
    validate_aggregate_spec(spec)?;
    let schema = working_dataset
        .clone()
        .collect_schema()
        .map_err(|err| AggregateError::ExecutionError(err.to_string()))?;
    validate_aggregate_compile(spec, &schema)?;
    let resolved_group_by = spec
        .group_by
        .iter()
        .map(|column| {
            resolve_column_reference(column, &schema)
                .ok_or_else(|| AggregateError::UnknownGroupByColumn(column.clone()))
        })
        .collect::<Result<Vec<_>, _>>()?;

    let mut filtered = working_dataset.clone();
    if schema.get("_deleted").is_some() {
        filtered = filtered.filter(col("_deleted").eq(lit(false)));
    }
    if let Some(selector_expr) = selector {
        filtered = filtered.filter(selector_expr);
    }

    let group_by_exprs = convert_group_by_to_polars_exprs(&resolved_group_by);
    let aggregation_exprs =
        convert_aggregations_to_polars_exprs_resolved(&spec.aggregations, &schema)?;
    let grouped = filtered.group_by(group_by_exprs).agg(aggregation_exprs);
    let summary = materialize_summary_rows(grouped, &schema, spec, &execution_context)?;

    let mut target_columns: Vec<(String, DataType)> = schema
        .iter_names()
        .map(|name| {
            (
                name.as_str().to_string(),
                schema.get(name.as_str()).cloned().unwrap_or(DataType::Null),
            )
        })
        .collect();
    for aggregation in &spec.aggregations {
        if target_columns
            .iter()
            .any(|(name, _)| name == &aggregation.column)
        {
            continue;
        }
        let dtype = summary
            .column(&aggregation.column)
            .map(|column| column.dtype().clone())
            .unwrap_or(DataType::Null);
        target_columns.push((aggregation.column.clone(), dtype));
    }
    for system_column in SYSTEM_COLUMNS {
        if target_columns.iter().any(|(name, _)| name == system_column) {
            continue;
        }
        let dtype = match system_column {
            "_deleted" => DataType::Boolean,
            _ => DataType::String,
        };
        target_columns.push((system_column.to_string(), dtype));
    }
    let original_ordered = align_frame_columns(working_dataset, &target_columns)?;
    let summary_ordered = align_frame_columns(summary.lazy(), &target_columns)?;

    concat(&[original_ordered, summary_ordered], UnionArgs::default())
        .map_err(|err| AggregateError::ExecutionError(err.to_string()))
}
