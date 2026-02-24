use std::collections::BTreeMap;

use anyhow::{Context, Result};
use chrono::Utc;
use polars::prelude::{col, lit, when, LazyFrame};
use tracing::debug;

use crate::dsl::{compile_selector, resolve_selector_reference};
use crate::model::DeleteOperationParams;
use crate::validation::{validate_named_selector_reference, validate_selector_boolean_type};

pub fn execute_delete(
    frame: LazyFrame,
    params: &DeleteOperationParams,
    selectors: &BTreeMap<String, String>,
) -> Result<LazyFrame> {
    let selector_expr = build_selector_expr(params, selectors)?;
    let active_match_expr = col("_deleted").eq(lit(false)).and(selector_expr);
    let now = Utc::now().timestamp_millis();

    debug!(
        selector = ?params.selector,
        timestamp_millis = now,
        "executing delete operation"
    );

    Ok(frame
        .with_column(
            when(active_match_expr.clone())
                .then(lit(now))
                .otherwise(col("_modified_at"))
                .alias("_modified_at"),
        )
        .with_column(
            when(active_match_expr)
                .then(lit(true))
                .otherwise(col("_deleted"))
                .alias("_deleted"),
        ))
}

fn build_selector_expr(
    params: &DeleteOperationParams,
    selectors: &BTreeMap<String, String>,
) -> Result<polars::prelude::Expr> {
    let Some(selector) = params.selector.as_deref() else {
        return Ok(lit(true));
    };

    validate_named_selector_reference(selector, selectors)
        .context("delete selector reference validation failed")?;
    validate_selector_boolean_type(selector, selectors)
        .context("delete selector must evaluate to a boolean expression")?;

    let resolved = resolve_selector_reference(selector, selectors)
        .context("failed to resolve delete selector reference")?;
    compile_selector(&resolved).context("failed to compile delete selector")
}
