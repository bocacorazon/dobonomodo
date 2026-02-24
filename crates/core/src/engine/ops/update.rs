//! Update operation execution for selector-based row updates.

use std::collections::HashMap;

use chrono::{DateTime, Utc};
use polars::prelude::*;
use serde::{Deserialize, Serialize};
use thiserror::Error;

use crate::dsl::{compile_expression, parse_expression, ColumnType, CompilationContext, CompilationError};

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
    SelectorCompile(#[source] CompilationError),
    #[error("Failed to compile assignment for column '{column}': invalid expression (Schema / not found): {source}")]
    SelectorCompile(#[source] CompilationError),
    #[error("Failed to compile assignment for column '{column}': invalid expression (Schema / not found): {source}")]
    AssignmentCompile {
        column: String,
        #[source]
        source: CompilationError,
        source: CompilationError,
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

    let helper_aliases: Vec<String> = schema
        .iter()
        .map(|(name, _)| format!("input.{}", name.as_str()))
        .collect();

    let helper_exprs: Vec<Expr> = schema
        .iter()
        .map(|(name, _)| {
            let base = name.as_str();
            col(base).alias(format!("input.{base}"))
        })
        .collect();

    let mut working_dataset = context.working_dataset.clone();
    if !helper_exprs.is_empty() {
        working_dataset = working_dataset.with_columns(helper_exprs);
    }

    let helper_aliases: Vec<String> = schema
        .iter()
        .map(|(name, _)| format!("input.{}", name.as_str()))
        .collect();

    let helper_exprs: Vec<Expr> = schema
        .iter()
        .map(|(name, _)| {
            let base = name.as_str();
            col(base).alias(format!("input.{base}"))
        })
        .collect();

    let mut working_dataset = context.working_dataset.clone();
    if !helper_exprs.is_empty() {
        working_dataset = working_dataset.with_columns(helper_exprs);
    }

    let default_selector_expr = default_selector_expr(&schema);
    let dsl_context = build_compilation_context(&schema, context.run_timestamp, &context.selectors);
    let resolved_selector = match operation.selector.as_deref() {
        Some(raw_selector) => resolve_selector(raw_selector, &context.selectors)?,
        None => None,
    };

    let selector_expr = match resolved_selector.as_deref() {
        Some(resolved) => default_selector_expr
            .clone()
            .and(compile_selector(resolved, &dsl_context)?),
        None => default_selector_expr.clone(),
    let dsl_context = build_compilation_context(&schema, context.run_timestamp, &context.selectors);
    let resolved_selector = match operation.selector.as_deref() {
        Some(raw_selector) => resolve_selector(raw_selector, &context.selectors)?,
        None => None,
    };

    let selector_expr = match resolved_selector.as_deref() {
        Some(resolved) => default_selector_expr
            .clone()
            .and(compile_selector(resolved, &dsl_context)?),
        None => default_selector_expr.clone(),
    };

    let applies_to_all_rows = resolved_selector.is_none() && !schema.contains("_deleted");

    let compiled_assignments = compile_assignments(&operation.assignments, &dsl_context)?;
    let applies_to_all_rows = resolved_selector.is_none() && !schema.contains("_deleted");

    let compiled_assignments = compile_assignments(&operation.assignments, &dsl_context)?;

    let mut assignment_exprs = Vec::with_capacity(compiled_assignments.len());

    for (assignment, value_expr) in operation
        .assignments
        .iter()
        .zip(compiled_assignments.into_iter())
    {
        let assignment_expr = if applies_to_all_rows {
            value_expr.alias(&assignment.column)
        } else {
            let fallback = if schema.contains(assignment.column.as_str()) {
                col(&assignment.column)
            } else {
                lit(LiteralValue::Null)
            };
            when(selector_expr.clone())
                .then(value_expr)
                .otherwise(fallback)
                .alias(&assignment.column)
        };
        assignment_exprs.push(assignment_expr);
        let assignment_expr = if applies_to_all_rows {
            value_expr.alias(&assignment.column)
        } else {
            let fallback = if schema.contains(assignment.column.as_str()) {
                col(&assignment.column)
            } else {
                lit(LiteralValue::Null)
            };
            when(selector_expr.clone())
                .then(value_expr)
                .otherwise(fallback)
                .alias(&assignment.column)
        };
        assignment_exprs.push(assignment_expr);
    }

    let updated_at_expr = if applies_to_all_rows {
        updated_at_value_expr(&schema, context.run_timestamp).alias("_updated_at")
    } else if schema.contains("_updated_at") {
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
    let mut output = working_dataset;
    let mut output = working_dataset;
    output = output.with_columns(vec![updated_at_expr]);
    for expr in assignment_exprs {
        output = output.with_columns(vec![expr]);
        if !helper_aliases.is_empty() {
            let current_schema = output.clone().collect_schema().map_err(UpdateError::OutputSchema)?;
            let refresh_exprs: Vec<Expr> = current_schema
                .iter()
                .filter_map(|(name, _)| {
                    let base = name.as_str();
                    if helper_aliases.iter().any(|alias| alias == base) {
                        None
                    } else {
                        Some(col(base).alias(format!("input.{base}")))
                    }
                })
                .collect();
            output = output.with_columns(refresh_exprs);
        }
        if !helper_aliases.is_empty() {
            let current_schema = output.clone().collect_schema().map_err(UpdateError::OutputSchema)?;
            let refresh_exprs: Vec<Expr> = current_schema
                .iter()
                .filter_map(|(name, _)| {
                    let base = name.as_str();
                    if helper_aliases.iter().any(|alias| alias == base) {
                        None
                    } else {
                        Some(col(base).alias(format!("input.{base}")))
                    }
                })
                .collect();
            output = output.with_columns(refresh_exprs);
        }
    }

    output.clone().collect_schema().map_err(UpdateError::OutputSchema)?;

    if !helper_aliases.is_empty() {
        let output_schema = output.clone().collect_schema().map_err(UpdateError::OutputSchema)?;
        let projected_columns: Vec<Expr> = output_schema
            .iter()
            .filter_map(|(name, _)| {
                let column_name = name.as_str();
                if helper_aliases.iter().any(|alias| alias == column_name) {
                    None
                } else {
                    Some(col(column_name))
                }
            })
            .collect();
        output = output.select(projected_columns);
    }

    if !helper_aliases.is_empty() {
        let output_schema = output.clone().collect_schema().map_err(UpdateError::OutputSchema)?;
        let projected_columns: Vec<Expr> = output_schema
            .iter()
            .filter_map(|(name, _)| {
                let column_name = name.as_str();
                if helper_aliases.iter().any(|alias| alias == column_name) {
                    None
                } else {
                    Some(col(column_name))
                }
            })
            .collect();
        output = output.select(projected_columns);
    }

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

fn compile_selector(selector_expr: &str, context: &CompilationContext) -> Result<Expr> {
    let normalized = normalize_update_expression(selector_expr);
    let ast = parse_expression(&normalized)
        .map_err(CompilationError::ParseFailure)
        .map_err(UpdateError::SelectorCompile)?;
    compile_expression(&normalized, &ast, context)
        .map(|compiled| compiled.into_expr())
        .map_err(UpdateError::SelectorCompile)
fn compile_selector(selector_expr: &str, context: &CompilationContext) -> Result<Expr> {
    let normalized = normalize_update_expression(selector_expr);
    let ast = parse_expression(&normalized)
        .map_err(CompilationError::ParseFailure)
        .map_err(UpdateError::SelectorCompile)?;
    compile_expression(&normalized, &ast, context)
        .map(|compiled| compiled.into_expr())
        .map_err(UpdateError::SelectorCompile)
}

fn compile_assignments(assignments: &[Assignment], context: &CompilationContext) -> Result<Vec<Expr>> {
fn compile_assignments(assignments: &[Assignment], context: &CompilationContext) -> Result<Vec<Expr>> {
    assignments
        .iter()
        .map(|assignment| {
            let normalized = normalize_update_expression(&assignment.expression);
            let ast = parse_expression(&normalized)
                .map_err(CompilationError::ParseFailure)
                .map_err(|source| UpdateError::AssignmentCompile {
                    column: assignment.column.clone(),
                    source,
                })?;
            compile_expression(&normalized, &ast, context)
                .map(|compiled| compiled.into_expr())
                .map_err(|source| UpdateError::AssignmentCompile {
            let normalized = normalize_update_expression(&assignment.expression);
            let ast = parse_expression(&normalized)
                .map_err(CompilationError::ParseFailure)
                .map_err(|source| UpdateError::AssignmentCompile {
                    column: assignment.column.clone(),
                    source,
                })?;
            compile_expression(&normalized, &ast, context)
                .map(|compiled| compiled.into_expr())
                .map_err(|source| UpdateError::AssignmentCompile {
                    column: assignment.column.clone(),
                    source,
                })
                })
        })
        .collect()
}

fn build_compilation_context(
    schema: &Schema,
    run_timestamp: DateTime<Utc>,
    selectors: &HashMap<String, String>,
) -> CompilationContext {
    let mut context = CompilationContext::new().with_today(run_timestamp.date_naive());

    for (name, data_type) in schema.iter() {
        let column_name = name.as_str();
        let column_type = map_data_type_to_column_type(data_type);
        context.add_column(column_name, column_type);
        context.add_column(format!("input.{column_name}"), column_type);
    }

    for (name, expr) in selectors {
        context.add_selector(name.clone(), expr.clone());
    }

    context
}

fn map_data_type_to_column_type(data_type: &DataType) -> ColumnType {
    match data_type {
        DataType::Boolean => ColumnType::Boolean,
        DataType::String => ColumnType::String,
        DataType::Date | DataType::Datetime(..) => ColumnType::Date,
        DataType::Int8
        | DataType::Int16
        | DataType::Int32
        | DataType::Int64
        | DataType::UInt8
        | DataType::UInt16
        | DataType::UInt32
        | DataType::UInt64
        | DataType::Float32
        | DataType::Float64 => ColumnType::Float,
        _ => ColumnType::String,
    }
}

fn normalize_update_expression(source: &str) -> String {
    let mut result = String::with_capacity(source.len() + 16);
    let chars: Vec<char> = source.chars().collect();
    let mut index = 0usize;
    let mut in_string = false;

    while index < chars.len() {
        let ch = chars[index];

        if ch == '"' {
            in_string = !in_string;
            result.push(ch);
            index += 1;
            continue;
        }

        if in_string {
            result.push(ch);
            index += 1;
            continue;
        }

        if ch == '_' || ch.is_ascii_alphabetic() {
            let start = index;
            index += 1;
            while index < chars.len()
                && (chars[index] == '_' || chars[index] == '.' || chars[index].is_ascii_alphanumeric())
            {
                index += 1;
            }

            let token: String = chars[start..index].iter().collect();
            let upper = token.to_ascii_uppercase();
            let mut lookahead = index;
            while lookahead < chars.len() && chars[lookahead].is_whitespace() {
                lookahead += 1;
            }
            let is_function = lookahead < chars.len() && chars[lookahead] == '(';

            if token.contains('.')
                || matches!(upper.as_str(), "AND" | "OR" | "NOT" | "TRUE" | "FALSE" | "NULL")
                || is_function
            {
                result.push_str(&token);
            } else {
                result.push_str("input.");
                result.push_str(&token);
            }
            continue;
        }

        result.push(ch);
        index += 1;
    }

    result
}

fn build_compilation_context(
    schema: &Schema,
    run_timestamp: DateTime<Utc>,
    selectors: &HashMap<String, String>,
) -> CompilationContext {
    let mut context = CompilationContext::new().with_today(run_timestamp.date_naive());

    for (name, data_type) in schema.iter() {
        let column_name = name.as_str();
        let column_type = map_data_type_to_column_type(data_type);
        context.add_column(column_name, column_type);
        context.add_column(format!("input.{column_name}"), column_type);
    }

    for (name, expr) in selectors {
        context.add_selector(name.clone(), expr.clone());
    }

    context
}

fn map_data_type_to_column_type(data_type: &DataType) -> ColumnType {
    match data_type {
        DataType::Boolean => ColumnType::Boolean,
        DataType::String => ColumnType::String,
        DataType::Date | DataType::Datetime(..) => ColumnType::Date,
        DataType::Int8
        | DataType::Int16
        | DataType::Int32
        | DataType::Int64
        | DataType::UInt8
        | DataType::UInt16
        | DataType::UInt32
        | DataType::UInt64
        | DataType::Float32
        | DataType::Float64 => ColumnType::Float,
        _ => ColumnType::String,
    }
}

fn normalize_update_expression(source: &str) -> String {
    let mut result = String::with_capacity(source.len() + 16);
    let chars: Vec<char> = source.chars().collect();
    let mut index = 0usize;
    let mut in_string = false;

    while index < chars.len() {
        let ch = chars[index];

        if ch == '"' {
            in_string = !in_string;
            result.push(ch);
            index += 1;
            continue;
        }

        if in_string {
            result.push(ch);
            index += 1;
            continue;
        }

        if ch == '_' || ch.is_ascii_alphabetic() {
            let start = index;
            index += 1;
            while index < chars.len()
                && (chars[index] == '_' || chars[index] == '.' || chars[index].is_ascii_alphanumeric())
            {
                index += 1;
            }

            let token: String = chars[start..index].iter().collect();
            let upper = token.to_ascii_uppercase();
            let mut lookahead = index;
            while lookahead < chars.len() && chars[lookahead].is_whitespace() {
                lookahead += 1;
            }
            let is_function = lookahead < chars.len() && chars[lookahead] == '(';

            if token.contains('.')
                || matches!(upper.as_str(), "AND" | "OR" | "NOT" | "TRUE" | "FALSE" | "NULL")
                || is_function
            {
                result.push_str(&token);
            } else {
                result.push_str("input.");
                result.push_str(&token);
            }
            continue;
        }

        result.push(ch);
        index += 1;
    }

    result
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
