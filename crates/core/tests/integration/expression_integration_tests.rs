use dobo_core::dsl::compiler::{compile_assignment_expression, ExpressionSymbolTable};

#[test]
fn assignment_expression_supports_multiple_join_references() {
    let mut symbols = ExpressionSymbolTable::with_working_columns([
        "amount_local".to_string(),
        "discount".to_string(),
    ]);
    symbols.add_join_alias("fx", ["rate".to_string()]);
    symbols.add_join_alias("customers", ["tier".to_string()]);
    symbols.add_join_alias("products", ["category".to_string()]);

    let compiled = compile_assignment_expression(
        "amount_local * fx.rate + IF(customers.tier = 'gold', discount, 0) + IF(products.category = 'services', 1, 0)",
        &symbols,
    )
    .expect("expression should compile");

    assert!(compiled.contains("rate_fx"));
    assert!(compiled.contains("tier_customers"));
    assert!(compiled.contains("category_products"));
}
