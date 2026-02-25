//! DSL Parser & Expression Compiler
//!
//! This module provides parsing, validation, and compilation functionality for
//! the DobONoMoDo expression DSL. It transforms expression strings into validated
//! Abstract Syntax Trees (ASTs) and compiles them into Polars `Expr` objects.
//!
//! # Architecture
//!
//! - **Parser**: Transforms expression strings into AST using pest grammar
//! - **Validator**: Type checking, column resolution, and semantic validation
//! - **Compiler**: Transforms validated AST into Polars expressions
//! - **Interpolation**: Expands {{SELECTOR}} references before parsing
//!
//! # Key APIs
//!
//! - [`parse_expression`]: parse source text into [`ExprAST`]
//! - [`interpolate_selectors`]: expand `{{NAME}}` references
//! - [`validate_expression`]: resolve columns and enforce type rules
//! - [`compile_expression`]: compile AST to Polars [`Expr`]
//! - [`compile_with_interpolation`]: full source-to-Expr pipeline
//!
//! # Example
//!
//! ```ignore
//! use dobo_core::dsl::{
//!     parse_expression, validate_expression, compile_expression, CompilationContext, ColumnType,
//! };
//!
//! let source = "transactions.amount * 1.1";
//! let mut context = CompilationContext::new();
//! context.add_column("transactions.amount", ColumnType::Float);
//!
//! let ast = parse_expression(source)?;
//! validate_expression(&ast, &context)?;
//! let polars_expr = compile_expression(source, &ast, &context)?.into_expr();
//! ```
//!
//! ```ignore
//! use dobo_core::dsl::{compile_with_interpolation, CompilationContext, ColumnType};
//!
//! let mut context = CompilationContext::new().with_aggregates(true);
//! context.add_column("transactions.amount", ColumnType::Float);
//! context.add_selector("HIGH", "transactions.amount > 1000");
//!
//! let compiled = compile_with_interpolation("{{HIGH}} AND SUM(transactions.amount) > 0", &context)?;
//! let expr = compiled.into_expr();
//! ```

use std::collections::BTreeMap;

use anyhow::{bail, Result};
use polars::prelude::{col, lit, Expr};

pub mod ast;
pub mod compiler;
pub mod context;
pub mod error;
pub mod interpolation;
pub mod parser;
pub mod types;
pub mod validation;

// Re-export main types and functions
pub use ast::*;
pub use compiler::{
    compile_expression, compile_expression_with_source, compile_with_interpolation,
    CompiledExpression,
};
pub use context::*;
pub use error::*;
pub use interpolation::interpolate_selectors;
pub use parser::{parse_expression, parse_expression_with_span, Span};
pub use types::*;
pub use validation::{infer_type, resolve_column, validate_expression};

/// Returns this module name (used by foundation compile checks).
const COMPARISON_OPERATORS: [&str; 7] = [">=", "<=", "!=", "==", "=", ">", "<"];
pub fn module_name() -> &'static str {
    "dsl"
}

pub fn resolve_selector_reference(
    selector: &str,
    selectors: &BTreeMap<String, String>,
) -> Result<String> {
    let selector = selector.trim();
    let Some(reference) = selector
        .strip_prefix("{{")
        .and_then(|value| value.strip_suffix("}}"))
    else {
        return Ok(selector.to_string());
    };

    let reference = reference.trim();
    if reference.is_empty() {
        bail!("selector reference cannot be empty");
    }

    selectors
        .get(reference)
        .cloned()
        .ok_or_else(|| anyhow::anyhow!("unknown selector reference: {reference}"))
}

pub fn compile_selector(selector: &str) -> Result<Expr> {
    let selector = selector.trim();
    if selector.is_empty() {
        bail!("selector cannot be empty");
    }

    if selector.eq_ignore_ascii_case("true") {
        return Ok(lit(true));
    }
    if selector.eq_ignore_ascii_case("false") {
        return Ok(lit(false));
    }

    let Some((index, operator)) = find_operator(selector) else {
        bail!("unsupported selector expression: {selector}");
    };

    let left_name = normalize_column_name(&selector[..index])?;
    let right_expr = parse_value_operand(&selector[index + operator.len()..])?;
    let left_expr = col(left_name.as_str());

    let compiled = match operator {
        "=" | "==" => left_expr.eq(right_expr),
        "!=" => left_expr.neq(right_expr),
        ">" => left_expr.gt(right_expr),
        "<" => left_expr.lt(right_expr),
        ">=" => left_expr.gt_eq(right_expr),
        "<=" => left_expr.lt_eq(right_expr),
        _ => unreachable!("operator should be matched above"),
    };

    Ok(compiled)
}

fn parse_value_operand(raw: &str) -> Result<Expr> {
    let value = raw.trim();
    if value.is_empty() {
        bail!("selector value cannot be empty");
    }

    if let Some(unquoted) = unquote(value) {
        return Ok(lit(unquoted.to_string()));
    }

    if value.eq_ignore_ascii_case("true") {
        return Ok(lit(true));
    }
    if value.eq_ignore_ascii_case("false") {
        return Ok(lit(false));
    }

    if let Ok(parsed) = value.parse::<i64>() {
        return Ok(lit(parsed));
    }

    if let Ok(parsed) = value.parse::<f64>() {
        return Ok(lit(parsed));
    }

    Ok(col(normalize_column_name(value)?.as_str()))
}

pub(crate) fn find_operator(selector: &str) -> Option<(usize, &'static str)> {
    let mut in_single = false;
    let mut in_double = false;
    let mut escaped = false;

    for (index, ch) in selector.char_indices() {
        if escaped {
            escaped = false;
            continue;
        }

        if ch == '\\' {
            escaped = true;
            continue;
        }

        if ch == '\'' && !in_double {
            in_single = !in_single;
            continue;
        }

        if ch == '"' && !in_single {
            in_double = !in_double;
            continue;
        }

        if in_single || in_double {
            continue;
        }

        for operator in COMPARISON_OPERATORS {
            if selector[index..].starts_with(operator) {
                return Some((index, operator));
            }
        }
    }

    None
}

pub(crate) fn normalize_column_name(raw: &str) -> Result<String> {
    let name = raw.trim();
    if name.is_empty() {
        bail!("selector column cannot be empty");
    }

    if name.contains(' ') {
        bail!("selector column cannot contain spaces: {name}");
    }

    let normalized = name.rsplit('.').next().unwrap_or(name).trim();
    if normalized.is_empty() {
        bail!("selector column cannot be empty");
    }

    Ok(normalized.to_string())
}

pub(crate) fn unquote(value: &str) -> Option<&str> {
    if value.len() < 2 {
        return None;
    }

    if value.starts_with('"') && value.ends_with('"') {
        return Some(&value[1..value.len() - 1]);
    }

    if value.starts_with('\'') && value.ends_with('\'') {
        return Some(&value[1..value.len() - 1]);
    }

    None
}
