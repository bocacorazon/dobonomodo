//! Expression compiler from DSL AST to Polars Expr.

use crate::dsl::ast::{BinaryOperator, ExprAST, LiteralValue, UnaryOperator};
use crate::dsl::context::CompilationContext;
use crate::dsl::error::CompilationError;
use crate::dsl::interpolation::interpolate_selectors;
use crate::dsl::parser::parse_expression;
use crate::dsl::types::ExprType;
use crate::dsl::validation::{infer_type, validate_expression};
use chrono::NaiveDate;
use polars::datatypes::{DataType, TimeUnit};
use polars::lazy::dsl::functions::concat_str;
use polars::lazy::dsl::{col, len, lit, when, Expr};
use polars::prelude::{Null, StrptimeOptions};

/// Result of a successful expression compilation.
#[derive(Debug, Clone)]
pub struct CompiledExpression {
    /// Source expression used for compilation.
    pub source: String,
    /// Compiled Polars expression.
    pub expr: Expr,
    /// Inferred expression return type.
    pub return_type: ExprType,
}

impl CompiledExpression {
    /// Consume and return the Polars expression.
    pub fn into_expr(self) -> Expr {
        self.expr
    }

    /// Borrow the Polars expression.
    pub fn as_expr(&self) -> &Expr {
        &self.expr
    }

    /// Return the inferred expression type.
    pub fn return_type(&self) -> ExprType {
        self.return_type
    }
}

/// Compile an AST expression to Polars Expr.
pub fn compile_expression(
    source: &str,
    ast: &ExprAST,
    context: &CompilationContext,
) -> Result<CompiledExpression, CompilationError> {
    if source.is_empty() {
        return Err(CompilationError::InternalError {
            message: "compile_expression requires non-empty authored source".to_string(),
        });
    }

    validate_expression(ast, context)?;
    let return_type = infer_type(ast, context)?;
    let expr = compile_ast(ast, context)?;

    Ok(CompiledExpression {
        source: source.to_string(),
        expr,
        return_type,
    })
}

/// Compile an AST expression with an explicit authored source string.
pub fn compile_expression_with_source(
    source: &str,
    ast: &ExprAST,
    context: &CompilationContext,
) -> Result<CompiledExpression, CompilationError> {
    compile_expression(source, ast, context)
}

/// Full pipeline: interpolation -> parse -> validate -> compile.
pub fn compile_with_interpolation(
    source: &str,
    context: &CompilationContext,
) -> Result<CompiledExpression, CompilationError> {
    let expanded = interpolate_selectors(source, context)?;
    let ast = parse_expression(&expanded)?;
    compile_expression(source, &ast, context)
}

fn compile_ast(ast: &ExprAST, context: &CompilationContext) -> Result<Expr, CompilationError> {
    match ast {
        ExprAST::Literal(lit_value) => Ok(compile_literal(lit_value)),
        ExprAST::ColumnRef { table, column } => {
            let name = if table.is_empty() {
                column.clone()
            } else {
                format!("{table}.{column}")
            };
            Ok(col(name))
        }
        ExprAST::BinaryOp { op, left, right } => {
            let left_type = infer_type(left, context)?;
            let right_type = infer_type(right, context)?;
            let left_expr = compile_ast(left, context)?;
            let right_expr = compile_ast(right, context)?;
            Ok(compile_binary_op(
                *op, left_expr, right_expr, left_type, right_type,
            ))
        }
        ExprAST::UnaryOp { op, operand } => {
            let operand_expr = compile_ast(operand, context)?;
            Ok(match op {
                UnaryOperator::Not => operand_expr.not(),
                UnaryOperator::Negate => lit(0.0) - operand_expr,
            })
        }
        ExprAST::FunctionCall { name, args } => compile_function(name, args, context),
    }
}

fn compile_literal(value: &LiteralValue) -> Expr {
    match value {
        LiteralValue::Number(n) => lit(*n),
        LiteralValue::String(s) => lit(s.clone()),
        LiteralValue::Boolean(b) => lit(*b),
        LiteralValue::Date(date) => lit(*date),
        LiteralValue::Null => lit(Null {}),
    }
}

fn compile_binary_op(
    op: BinaryOperator,
    left: Expr,
    right: Expr,
    left_type: ExprType,
    right_type: ExprType,
) -> Expr {
    match op {
        BinaryOperator::Add => {
            if left_type == ExprType::Date && right_type == ExprType::Number {
                left + days_to_duration(right)
            } else {
                left + right
            }
        }
        BinaryOperator::Subtract => {
            if left_type == ExprType::Date && right_type == ExprType::Number {
                left - days_to_duration(right)
            } else {
                left - right
            }
        }
        BinaryOperator::Multiply => left * right,
        BinaryOperator::Divide => when(right.clone().eq(lit(0)))
            .then(lit(Null {}))
            .otherwise(left / right),
        BinaryOperator::Equal => left.eq(right),
        BinaryOperator::NotEqual => left.neq(right),
        BinaryOperator::LessThan => left.lt(right),
        BinaryOperator::LessThanOrEqual => left.lt_eq(right),
        BinaryOperator::GreaterThan => left.gt(right),
        BinaryOperator::GreaterThanOrEqual => left.gt_eq(right),
        BinaryOperator::And => left.and(right),
        BinaryOperator::Or => left.or(right),
    }
}

fn days_to_duration(days: Expr) -> Expr {
    let duration_ms = days.cast(DataType::Int64) * lit(86_400_000i64);
    duration_ms.cast(DataType::Duration(TimeUnit::Milliseconds))
}

fn compile_function(
    name: &str,
    args: &[ExprAST],
    context: &CompilationContext,
) -> Result<Expr, CompilationError> {
    let normalized = name.to_uppercase();
    let compiled_args = args
        .iter()
        .map(|arg| compile_ast(arg, context))
        .collect::<Result<Vec<_>, _>>()?;

    let expr = match normalized.as_str() {
        // Arithmetic functions
        "ABS" => {
            let arg = require_arg(&compiled_args, &normalized)?;
            when(arg.clone().lt(lit(0)))
                .then(lit(0) - arg.clone())
                .otherwise(arg)
        }
        "ROUND" => {
            let value = require_arg(&compiled_args, &normalized)?;
            let (_, decimals) = require_binary_args(&compiled_args, &normalized)?;
            let factor = lit(10.0).pow(decimals.cast(DataType::Float64));
            (value * factor.clone()).round(0) / factor
        }
        "FLOOR" => require_arg(&compiled_args, &normalized)?.floor(),
        "CEIL" => require_arg(&compiled_args, &normalized)?.ceil(),
        "MOD" => {
            let (left, right) = require_binary_args(&compiled_args, &normalized)?;
            left % right
        }
        "MIN" => {
            let (left, right) = require_binary_args(&compiled_args, &normalized)?;
            when(left.clone().lt(right.clone()))
                .then(left)
                .otherwise(right)
        }
        "MAX" => {
            let (left, right) = require_binary_args(&compiled_args, &normalized)?;
            when(left.clone().gt(right.clone()))
                .then(left)
                .otherwise(right)
        }

        // Aggregate
        "SUM" => require_arg(&compiled_args, &normalized)?.sum(),
        "COUNT" => require_arg(&compiled_args, &normalized)?.count(),
        "COUNT_ALL" => len(),
        "AVG" => require_arg(&compiled_args, &normalized)?.mean(),
        "MIN_AGG" => require_arg(&compiled_args, &normalized)?.min(),
        "MAX_AGG" => require_arg(&compiled_args, &normalized)?.max(),

        // Conditional
        "IF" => {
            if compiled_args.len() != 3 {
                return Err(CompilationError::UnsupportedFunction {
                    function: normalized,
                    reason: "IF requires 3 arguments".to_string(),
                });
            }
            when(compiled_args[0].clone())
                .then(compiled_args[1].clone())
                .otherwise(compiled_args[2].clone())
        }
        "AND" => {
            if compiled_args.len() < 2 {
                return Err(CompilationError::UnsupportedFunction {
                    function: normalized,
                    reason: "AND requires at least 2 arguments".to_string(),
                });
            }
            let mut iter = compiled_args.into_iter();
            let mut expr = iter.next().ok_or_else(|| CompilationError::InternalError {
                message: "AND received no arguments".to_string(),
            })?;
            for candidate in iter {
                expr = expr.and(candidate);
            }
            expr
        }
        "OR" => {
            if compiled_args.len() < 2 {
                return Err(CompilationError::UnsupportedFunction {
                    function: normalized,
                    reason: "OR requires at least 2 arguments".to_string(),
                });
            }
            let mut iter = compiled_args.into_iter();
            let mut expr = iter.next().ok_or_else(|| CompilationError::InternalError {
                message: "OR received no arguments".to_string(),
            })?;
            for candidate in iter {
                expr = expr.or(candidate);
            }
            expr
        }
        "NOT" => {
            if compiled_args.len() != 1 {
                return Err(CompilationError::UnsupportedFunction {
                    function: normalized,
                    reason: "NOT requires 1 argument".to_string(),
                });
            }
            compiled_args[0].clone().not()
        }
        "ISNULL" | "IS_NULL" => require_arg(&compiled_args, &normalized)?.is_null(),
        "COALESCE" => {
            if compiled_args.is_empty() {
                return Err(CompilationError::UnsupportedFunction {
                    function: normalized,
                    reason: "COALESCE requires at least one argument".to_string(),
                });
            }
            let mut iter = compiled_args.into_iter();
            let mut expr = iter.next().ok_or_else(|| CompilationError::InternalError {
                message: "COALESCE received no arguments".to_string(),
            })?;
            for candidate in iter {
                expr = expr.fill_null(candidate);
            }
            expr
        }

        // String functions
        "CONCAT" => {
            if compiled_args.is_empty() {
                return Err(CompilationError::UnsupportedFunction {
                    function: normalized,
                    reason: "CONCAT requires at least one argument".to_string(),
                });
            }
            concat_str(compiled_args, "", false)
        }
        "UPPER" => require_arg(&compiled_args, &normalized)?
            .str()
            .to_uppercase(),
        "LOWER" => require_arg(&compiled_args, &normalized)?
            .str()
            .to_lowercase(),
        "TRIM" => require_arg(&compiled_args, &normalized)?
            .str()
            .strip_chars(lit(" \t\r\n")),
        "LEFT" => {
            let (text, len) = require_binary_args(&compiled_args, &normalized)?;
            text.str().slice(lit(0), len)
        }
        "RIGHT" => {
            let (text, len) = require_binary_args(&compiled_args, &normalized)?;
            text.str().slice(lit(0) - len.clone(), len)
        }
        "LEN" => require_arg(&compiled_args, &normalized)?.str().len_chars(),
        "CONTAINS" => {
            let (text, needle) = require_binary_args(&compiled_args, &normalized)?;
            text.str().contains_literal(needle)
        }
        "REPLACE" => {
            if compiled_args.len() != 3 {
                return Err(CompilationError::UnsupportedFunction {
                    function: normalized,
                    reason: "REPLACE requires 3 arguments".to_string(),
                });
            }
            compiled_args[0].clone().str().replace_all(
                compiled_args[1].clone(),
                compiled_args[2].clone(),
                true,
            )
        }

        // Date functions
        "DATE" => {
            if args.len() != 1 {
                return Err(CompilationError::UnsupportedFunction {
                    function: normalized,
                    reason: "DATE requires 1 argument".to_string(),
                });
            }
            match &args[0] {
                ExprAST::Literal(LiteralValue::String(raw)) => {
                    let parsed = NaiveDate::parse_from_str(raw, "%Y-%m-%d").map_err(|_| {
                        CompilationError::UnsupportedFunction {
                            function: normalized,
                            reason: format!("Invalid ISO date literal: {raw}"),
                        }
                    })?;
                    lit(parsed)
                }
                _ => require_arg(&compiled_args, &normalized)?
                    .str()
                    .to_date(StrptimeOptions::default()),
            }
        }
        "TODAY" => {
            if !compiled_args.is_empty() {
                return Err(CompilationError::UnsupportedFunction {
                    function: normalized,
                    reason: "TODAY requires 0 arguments".to_string(),
                });
            }
            let today = context.today.as_ref().cloned().ok_or_else(|| {
                CompilationError::UnsupportedFunction {
                    function: normalized,
                    reason: "TODAY requires an explicit context date via with_today(...)"
                        .to_string(),
                }
            })?;
            lit(today).cast(DataType::Date)
        }
        "YEAR" => require_arg(&compiled_args, &normalized)?.dt().year(),
        "MONTH" => require_arg(&compiled_args, &normalized)?.dt().month(),
        "DAY" => require_arg(&compiled_args, &normalized)?.dt().day(),
        "DATEDIFF" => {
            let (end_date, start_date) = require_binary_args(&compiled_args, &normalized)?;
            (end_date - start_date).dt().total_days()
        }
        "DATEADD" => {
            let (date, days) = require_binary_args(&compiled_args, &normalized)?;
            let duration_ms = (days.cast(DataType::Int64)) * lit(86_400_000i64);
            date + duration_ms.cast(DataType::Duration(TimeUnit::Milliseconds))
        }

        _ => {
            return Err(CompilationError::UnsupportedFunction {
                function: normalized,
                reason: "Function mapping not implemented".to_string(),
            })
        }
    };

    Ok(expr)
}

fn require_arg(args: &[Expr], function: &str) -> Result<Expr, CompilationError> {
    args.first()
        .cloned()
        .ok_or_else(|| CompilationError::UnsupportedFunction {
            function: function.to_string(),
            reason: "Expected at least one argument".to_string(),
        })
}

fn require_binary_args(args: &[Expr], function: &str) -> Result<(Expr, Expr), CompilationError> {
    if args.len() < 2 {
        return Err(CompilationError::UnsupportedFunction {
            function: function.to_string(),
            reason: "Expected two arguments".to_string(),
        });
    }
    Ok((args[0].clone(), args[1].clone()))
}
