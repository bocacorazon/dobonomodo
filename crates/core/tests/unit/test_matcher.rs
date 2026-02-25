use dobo_core::resolver::matcher::{evaluate_expression, parse_expression, tokenize};
use std::collections::HashMap;

#[test]
fn test_nested_expression_precedence() {
    let tokens = tokenize("table == 'sales' OR (dataset == 'prod' AND period >= '2024-Q1')").unwrap();
    let expr = parse_expression(&tokens).unwrap();

    let mut context = HashMap::new();
    context.insert("table".to_string(), "inventory".to_string());
    context.insert("dataset".to_string(), "prod".to_string());
    context.insert("period".to_string(), "2024-Q2".to_string());

    assert!(evaluate_expression(&expr, &context).unwrap());
}

#[test]
fn test_invalid_syntax_reports_error() {
    let tokens = tokenize("(table == 'sales'").unwrap();
    let error = parse_expression(&tokens).unwrap_err();
    assert!(error.contains("expected ')'"));
}

#[test]
fn test_not_operator() {
    let tokens = tokenize("NOT (table == 'sales')").unwrap();
    let expr = parse_expression(&tokens).unwrap();

    let mut context = HashMap::new();
    context.insert("table".to_string(), "inventory".to_string());

    assert!(evaluate_expression(&expr, &context).unwrap());
}
