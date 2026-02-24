use std::collections::BTreeMap;

use anyhow::{Context, Result};
use polars::prelude::{col, lit, LazyFrame};

use crate::dsl::{compile_selector, resolve_selector_reference};
use crate::model::OutputOperationParams;
use crate::validation::{validate_named_selector_reference, validate_selector_boolean_type};

pub fn execute_output(
    frame: LazyFrame,
    params: &OutputOperationParams,
    selectors: &BTreeMap<String, String>,
) -> Result<LazyFrame> {
    let mut output = frame;

    if !params.include_deleted {
        output = output.filter(col("_deleted").eq(lit(false)));
    }

    if let Some(selector) = params.selector.as_deref() {
        validate_named_selector_reference(selector, selectors)
            .context("output selector reference validation failed")?;
        validate_selector_boolean_type(selector, selectors)
            .context("output selector must evaluate to a boolean expression")?;

        let resolved = resolve_selector_reference(selector, selectors)
            .context("failed to resolve output selector reference")?;
        output = output.filter(
            compile_selector(&resolved).context("failed to compile output selector expression")?,
        );
    }

    Ok(output)
}
