use dobo_core::dsl::compiler::{
    compile_assignment_expression, CompileError, ExpressionSymbolTable,
};

#[test]
fn supports_join_aliases_in_symbol_table() {
    let mut symbols = ExpressionSymbolTable::with_working_columns([
        "amount_local".to_string(),
        "amount_reporting".to_string(),
    ]);
    symbols.add_join_alias("fx", ["rate".to_string()]);
    symbols.add_join_alias("customers", ["tier".to_string()]);

    let compiled = compile_assignment_expression(
        "amount_local * fx.rate + IF(customers.tier = 'gold', 10, 0)",
        &symbols,
    )
    .expect("expression should compile");

    assert!(compiled.contains("rate_fx"));
    assert!(compiled.contains("tier_customers"));
}

#[test]
fn maps_alias_columns_to_suffixed_polars_columns() {
    let mut symbols = ExpressionSymbolTable::with_working_columns(["amount_local".to_string()]);
    symbols.add_join_alias("products", ["category".to_string()]);

    let compiled = compile_assignment_expression("products.category", &symbols)
        .expect("reference should compile");
    assert_eq!(compiled, "category_products");
}

#[test]
fn errors_for_unknown_alias_or_column() {
    let mut symbols = ExpressionSymbolTable::with_working_columns(["amount_local".to_string()]);
    symbols.add_join_alias("fx", ["rate".to_string()]);

    let unknown_alias = compile_assignment_expression("unknown.rate", &symbols)
        .expect_err("unknown alias should fail");
    assert!(matches!(unknown_alias, CompileError::UnknownAlias(alias) if alias == "unknown"));

    let unknown_column = compile_assignment_expression("fx.missing", &symbols)
        .expect_err("unknown column should fail");
    assert!(matches!(
        unknown_column,
        CompileError::UnknownAliasedColumn { alias, column }
        if alias == "fx" && column == "missing"
    ));

    let unknown_unaliased = compile_assignment_expression("amount_locl * fx.rate", &symbols)
        .expect_err("unknown unaliased column should fail");
    assert!(matches!(
        unknown_unaliased,
        CompileError::UnknownColumn(column) if column == "amount_locl"
    ));
}
