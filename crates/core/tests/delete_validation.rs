use std::collections::{BTreeMap, BTreeSet};

use anyhow::Result;
use dobo_core::validation::{
    validate_named_selector_reference, validate_selector_boolean_type,
    validate_selector_boolean_type_with_schema,
};

#[test]
fn selector_boolean_type_validation_accepts_boolean_expressions() -> Result<()> {
    validate_selector_boolean_type("amount = 0", &BTreeMap::new())
}

#[test]
fn selector_boolean_type_validation_rejects_invalid_expressions() {
    let result = validate_selector_boolean_type("amount + 1", &BTreeMap::new());
    assert!(result.is_err());
}

#[test]
fn named_selector_validation_rejects_unknown_reference() {
    let result = validate_named_selector_reference("{{missing_selector}}", &BTreeMap::new());
    assert!(result.is_err());
}

#[test]
fn named_selector_validation_accepts_known_reference() -> Result<()> {
    let mut selectors = BTreeMap::new();
    selectors.insert("zero_amount".to_string(), "amount = 0".to_string());
    validate_named_selector_reference("{{zero_amount}}", &selectors)
}

#[test]
fn selector_validation_with_schema_rejects_unknown_column() {
    let columns = BTreeSet::from(["amount".to_string()]);
    let result =
        validate_selector_boolean_type_with_schema("missing_col = 1", &BTreeMap::new(), &columns);
    assert!(result.is_err());
}
