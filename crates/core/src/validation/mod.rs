use std::collections::{BTreeMap, BTreeSet};

use anyhow::{bail, Result};

use crate::dsl::{
    compile_selector, find_operator, normalize_column_name, resolve_selector_reference, unquote,
};

pub fn module_name() -> &'static str {
    "validation"
}

pub fn validate_named_selector_reference(
    selector: &str,
    selectors: &BTreeMap<String, String>,
) -> Result<()> {
    let selector = selector.trim();
    if selector.is_empty() {
        bail!("selector cannot be empty");
    }

    if selector.starts_with("{{") && selector.ends_with("}}") {
        resolve_selector_reference(selector, selectors)?;
    }

    Ok(())
}

pub fn validate_selector_boolean_type(
    selector: &str,
    selectors: &BTreeMap<String, String>,
) -> Result<()> {
    let resolved = resolve_selector_reference(selector, selectors)?;

    // compile_selector only supports expressions that evaluate to booleans.
    let _ = compile_selector(&resolved)?;
    Ok(())
}

pub fn validate_selector_boolean_type_with_schema(
    selector: &str,
    selectors: &BTreeMap<String, String>,
    available_columns: &BTreeSet<String>,
) -> Result<()> {
    let resolved = resolve_selector_reference(selector, selectors)?;

    // compile_selector only supports expressions that evaluate to booleans.
    let _ = compile_selector(&resolved)?;

    for column in selector_column_references(&resolved)? {
        if !available_columns.contains(&column) {
            bail!("selector references unknown column: {column}");
        }
    }

    Ok(())
}

fn selector_column_references(selector: &str) -> Result<BTreeSet<String>> {
    let selector = selector.trim();
    if selector.eq_ignore_ascii_case("true") || selector.eq_ignore_ascii_case("false") {
        return Ok(BTreeSet::new());
    }

    let Some((index, operator)) = find_operator(selector) else {
        return Ok(BTreeSet::new());
    };

    let mut columns = BTreeSet::new();
    columns.insert(normalize_column_name(&selector[..index])?);

    let right = selector[index + operator.len()..].trim();
    if !is_literal(right) {
        columns.insert(normalize_column_name(right)?);
    }

    Ok(columns)
}

fn is_literal(value: &str) -> bool {
    let value = value.trim();
    if value.is_empty() {
        return false;
    }

    if unquote(value).is_some() {
        return true;
    }

    if value.eq_ignore_ascii_case("true") || value.eq_ignore_ascii_case("false") {
        return true;
    }

    value.parse::<i64>().is_ok() || value.parse::<f64>().is_ok()
}
