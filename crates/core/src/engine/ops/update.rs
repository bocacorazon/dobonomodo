//! Update operation execution for selector-based row updates.

use std::collections::HashMap;

use chrono::{DateTime, Utc};
use polars::prelude::*;
use serde::{Deserialize, Serialize};
use thiserror::Error;

use crate::dsl::{compile_expression, ExpressionCompileContext, ExpressionError};

/// Runtime configuration for an update operation.
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
pub struct UpdateOperation {
    /// Optional row filter expression; supports `{{NAME}}` selector lookup.
    #[serde(default)]
    pub selector: Option<String>,
    /// Column assignments to apply.
    pub assignments: Vec<Assignment>,
}

/// A single column assignment in an update operation.
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
pub struct Assignment {
    /// Target column name.
    pub column: String,
    /// Assignment expression.
    pub expression: String,
}

/// Context supplied by the pipeline executor for update execution.
#[derive(Clone)]
pub struct UpdateExecutionContext {
    pub working_dataset: LazyFrame,
    pub selectors: HashMap<String, String>,
    pub run_timestamp: DateTime<Utc>,
}

#[derive(Debug, Error)]
pub enum UpdateError {
    #[error("Update operation requires at least one assignment")]
    EmptyAssignments,
    #[error("Invalid column name: '{0}'")]
    InvalidColumnName(String),
    #[error("Cannot assign to reserved system column '{0}' in update operation")]
    ReservedSystemColumnAssignment(String),
    #[error("Assignment expression for '{0}' cannot be empty")]
    EmptyAssignmentExpression(String),
    #[error("Unclosed selector interpolation in '{0}'")]
    UnclosedSelectorInterpolation(String),
    #[error("Selector interpolation name cannot be empty")]
    EmptySelectorInterpolationName,
    #[error("Selector '{0}' not defined in Project")]
    SelectorNotDefined(String),
    #[error("Failed to compile selector expression: {0}")]
    SelectorCompile(#[source] ExpressionError),
    #[error("Failed to compile assignment for column '{column}': invalid expression: {source}")]
    AssignmentCompile {
        column: String,
        #[source]
        source: ExpressionError,
    },
    #[error("Failed to inspect input schema before update: {0}")]
    InputSchema(#[source] PolarsError),
    #[error("Failed to validate update output schema: {0}")]
    OutputSchema(#[source] PolarsError),
}

type Result<T> = std::result::Result<T, UpdateError>;

/// Execute an update operation and return a transformed LazyFrame.
pub fn execute_update(
    context: &UpdateExecutionContext,
    operation: &UpdateOperation,
) -> Result<LazyFrame> {
    validate_operation(operation)?;

    let schema = context
        .working_dataset
        .clone()
        .collect_schema()
        .map_err(UpdateError::InputSchema)?;

    let default_selector_expr = default_selector_expr(&schema);
    let selector_expr = match operation.selector.as_deref() {
        Some(raw_selector) => match resolve_selector(raw_selector, &context.selectors)? {
            Some(resolved) => default_selector_expr
                .clone()
                .and(compile_selector(&resolved, context.run_timestamp)?),
            None => default_selector_expr.clone(),
        },
        None => default_selector_expr,
    };

    let compiled_assignments = compile_assignments(&operation.assignments, context.run_timestamp)?;

    let mut assignment_exprs = Vec::with_capacity(compiled_assignments.len());

    for (assignment, value_expr) in operation
        .assignments
        .iter()
        .zip(compiled_assignments.into_iter())
    {
        let fallback = if schema.contains(assignment.column.as_str()) {
            col(&assignment.column)
        } else {
            lit(LiteralValue::Null)
        };

        let conditional_expr = when(selector_expr.clone())
            .then(value_expr)
            .otherwise(fallback)
            .alias(&assignment.column);
        assignment_exprs.push(conditional_expr);
    }

    let updated_at_expr = if schema.contains("_updated_at") {
        when(selector_expr)
            .then(updated_at_value_expr(&schema, context.run_timestamp))
            .otherwise(col("_updated_at"))
            .alias("_updated_at")
    } else {
        when(selector_expr)
            .then(updated_at_value_expr(&schema, context.run_timestamp))
            .otherwise(lit(LiteralValue::Null))
            .alias("_updated_at")
    };
    let mut output = context.working_dataset.clone();
    output = output.with_columns(vec![updated_at_expr]);
    for expr in assignment_exprs {
        output = output.with_columns(vec![expr]);
    }

    output.clone().collect_schema().map_err(UpdateError::OutputSchema)?;

    Ok(output)
}

fn validate_operation(operation: &UpdateOperation) -> Result<()> {
    if operation.assignments.is_empty() {
        return Err(UpdateError::EmptyAssignments);
    }

    for assignment in &operation.assignments {
        if !is_valid_identifier(&assignment.column) {
            return Err(UpdateError::InvalidColumnName(assignment.column.clone()));
        }

        if is_reserved_system_column(&assignment.column) {
            return Err(UpdateError::ReservedSystemColumnAssignment(
                assignment.column.clone(),
            ));
        }

        if assignment.expression.trim().is_empty() {
            return Err(UpdateError::EmptyAssignmentExpression(
                assignment.column.clone(),
            ));
        }
    }

    Ok(())
}

fn is_valid_identifier(name: &str) -> bool {
    let mut chars = name.chars();
    let Some(first) = chars.next() else {
        return false;
    };

    if !(first == '_' || first.is_ascii_alphabetic()) {
        return false;
    }

    chars.all(|c| c == '_' || c.is_ascii_alphanumeric())
}

fn is_reserved_system_column(name: &str) -> bool {
    matches!(
        name,
        "_row_id"
            | "_source_dataset_id"
            | "_source_table"
            | "_created_by_project_id"
            | "_created_by_run_id"
            | "_created_at"
            | "_updated_at"
            | "_deleted"
            | "_labels"
            | "_period"
            | "_period_from"
            | "_period_to"
            | "_valid_from"
            | "_valid_to"
    )
}

fn resolve_selector(selector: &str, selectors: &HashMap<String, String>) -> Result<Option<String>> {
    let selector = selector.trim();
    if selector.is_empty() {
        return Ok(None);
    }

    let mut resolved = String::with_capacity(selector.len());
    let mut cursor = 0usize;

    while let Some(open_rel) = selector[cursor..].find("{{") {
        let open = cursor + open_rel;
        resolved.push_str(&selector[cursor..open]);

        let content_start = open + 2;
        let close_rel = selector[content_start..]
            .find("}}")
            .ok_or_else(|| UpdateError::UnclosedSelectorInterpolation(selector.to_string()))?;
        let close = content_start + close_rel;

        let name = selector[content_start..close].trim();
        if name.is_empty() {
            return Err(UpdateError::EmptySelectorInterpolationName);
        }

        let replacement = selectors
            .get(name)
            .ok_or_else(|| UpdateError::SelectorNotDefined(name.to_string()))?;
        resolved.push('(');
        resolved.push_str(replacement);
        resolved.push(')');

        cursor = close + 2;
    }

    resolved.push_str(&selector[cursor..]);
    let resolved = resolved.trim();
    if resolved.is_empty() {
        Ok(None)
    } else {
        Ok(Some(resolved.to_string()))
    }
}

fn compile_selector(selector_expr: &str, run_timestamp: DateTime<Utc>) -> Result<Expr> {
    compile_expression(
        selector_expr,
        &ExpressionCompileContext::for_row_level(run_timestamp),
    )
    .map_err(UpdateError::SelectorCompile)
}

fn compile_assignments(
    assignments: &[Assignment],
    run_timestamp: DateTime<Utc>,
) -> Result<Vec<Expr>> {
    let context = ExpressionCompileContext::for_row_level(run_timestamp);

    assignments
        .iter()
        .map(|assignment| {
            compile_expression(&assignment.expression, &context).map_err(|source| {
                UpdateError::AssignmentCompile {
                    column: assignment.column.clone(),
                    source,
                }
            })
        })
        .collect()
}

fn default_selector_expr(schema: &Schema) -> Expr {
    if schema.contains("_deleted") {
        col("_deleted").neq(lit(true)).or(col("_deleted").is_null())
    } else {
        lit(true)
    }
}

fn updated_at_value_expr(schema: &Schema, run_timestamp: DateTime<Utc>) -> Expr {
    match schema.get("_updated_at") {
        Some(DataType::Datetime(unit, timezone)) => {
            lit(run_timestamp_for_time_unit(run_timestamp, *unit))
                .cast(DataType::Datetime(*unit, timezone.clone()))
        }
        _ => lit(run_timestamp.timestamp_millis()),
    }
}

fn run_timestamp_for_time_unit(run_timestamp: DateTime<Utc>, unit: TimeUnit) -> i64 {
    match unit {
        TimeUnit::Milliseconds => run_timestamp.timestamp_millis(),
        TimeUnit::Microseconds => run_timestamp.timestamp_micros(),
        TimeUnit::Nanoseconds => run_timestamp
            .timestamp_nanos_opt()
            .unwrap_or_else(|| run_timestamp.timestamp_micros().saturating_mul(1_000)),
    }
}
