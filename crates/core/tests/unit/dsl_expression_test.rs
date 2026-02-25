use dobo_core::dsl::expression::parse_source_selector;

#[test]
fn parse_source_selector_supports_string_equality() {
    let expr = parse_source_selector("budget_type = 'original'");
    assert!(expr.is_ok());
}

#[test]
fn parse_source_selector_supports_numeric_comparison() {
    let expr = parse_source_selector("amount > 10000");
    assert!(expr.is_ok());
}

#[test]
fn parse_source_selector_supports_and_expressions() {
    let expr = parse_source_selector("status = 'approved' AND amount > 5000");
    assert!(expr.is_ok());
}

#[test]
fn parse_source_selector_rejects_invalid_expression() {
    let expr = parse_source_selector("invalid expression");
    assert!(expr.is_err());
}
