//! Unit and integration tests for DSL validation
//!
//! These tests verify the validator can correctly:
//! - Resolve column references against a schema
//! - Infer types for all expression nodes
//! - Check type compatibility for operations
//! - Validate aggregate function usage
//! - Interpolate selector references
//! - Handle edge cases and circular references

use dobo_core::dsl::*;

// ============================================================================
// T029: Unit test for column resolution
// ============================================================================

#[test]
fn test_column_resolution_valid_simple() {
    // Setup: Create a context with a known schema
    let mut ctx = CompilationContext::new();
    ctx.add_column("transactions.amount", ColumnType::Float);
    ctx.add_column("transactions.count", ColumnType::Integer);

    // Parse a simple column reference
    let expr = parse_expression("transactions.amount").expect("Failed to parse");

    // Validate it should succeed
    let result = validate_expression(&expr, &ctx);
    assert!(result.is_ok(), "Valid column should pass validation");
}

#[test]
fn test_column_resolution_valid_multiple_tables() {
    let mut ctx = CompilationContext::new();
    ctx.add_column("users.id", ColumnType::Integer);
    ctx.add_column("users.name", ColumnType::String);
    ctx.add_column("transactions.user_id", ColumnType::Integer);
    ctx.add_column("transactions.amount", ColumnType::Float);

    // Test each column
    for col_ref in &["users.id", "users.name", "transactions.user_id", "transactions.amount"] {
        let expr = parse_expression(col_ref).expect("Failed to parse");
        let result = validate_expression(&expr, &ctx);
        assert!(result.is_ok(), "Column {} should be valid", col_ref);
    }
}

#[test]
fn test_column_resolution_invalid_unresolved_column() {
    let mut ctx = CompilationContext::new();
    ctx.add_column("transactions.amount", ColumnType::Float);

    // Try to reference a non-existent column
    let expr = parse_expression("transactions.missing").expect("Failed to parse");
    let result = validate_expression(&expr, &ctx);

    assert!(result.is_err(), "Unresolved column should fail");
    if let Err(ValidationError::UnresolvedColumnRef { table, column }) = result {
        assert_eq!(table, "transactions");
        assert_eq!(column, "missing");
    } else {
        panic!("Expected UnresolvedColumnRef error");
    }
}

#[test]
fn test_column_resolution_invalid_wrong_table() {
    let mut ctx = CompilationContext::new();
    ctx.add_column("transactions.amount", ColumnType::Float);

    let expr = parse_expression("orders.amount").expect("Failed to parse");
    let result = validate_expression(&expr, &ctx);

    assert!(result.is_err(), "Wrong table should fail");
    match result {
        Err(ValidationError::UnresolvedColumnRef { table, column }) => {
            assert_eq!(table, "orders");
            assert_eq!(column, "amount");
        }
        _ => panic!("Expected UnresolvedColumnRef error"),
    }
}

#[test]
fn test_column_resolution_in_expression() {
    let mut ctx = CompilationContext::new();
    ctx.add_column("transactions.amount", ColumnType::Float);
    ctx.add_column("transactions.tax_rate", ColumnType::Float);

    // Validate column ref in a binary expression
    let expr = parse_expression("transactions.amount * transactions.tax_rate")
        .expect("Failed to parse");
    let result = validate_expression(&expr, &ctx);

    assert!(result.is_ok(), "Valid columns in expression should pass");
}

#[test]
fn test_column_resolution_in_function() {
    let mut ctx = CompilationContext::new();
    ctx.add_column("transactions.amount", ColumnType::Float);

    // Validate column ref inside function call
    let expr = parse_expression("SUM(transactions.amount)").expect("Failed to parse");
    let result = validate_expression(&expr, &ctx);

    // Should succeed (ignoring aggregate context for now)
    assert!(result.is_ok(), "Valid column in function should pass");
}

// ============================================================================
// T030: Unit test for type inference
// ============================================================================

#[test]
fn test_type_inference_literal_number() {
    let ctx = CompilationContext::new();
    let expr = parse_expression("42").expect("Failed to parse");
    let result = validate_expression(&expr, &ctx);

    assert!(result.is_ok());
    let typed = result.unwrap();
    assert_eq!(typed.return_type, ExprType::Number);
}

#[test]
fn test_type_inference_literal_string() {
    let ctx = CompilationContext::new();
    let expr = parse_expression(r#""hello""#).expect("Failed to parse");
    let result = validate_expression(&expr, &ctx);

    assert!(result.is_ok());
    let typed = result.unwrap();
    assert_eq!(typed.return_type, ExprType::String);
}

#[test]
fn test_type_inference_literal_boolean() {
    let ctx = CompilationContext::new();
    let expr = parse_expression("TRUE").expect("Failed to parse");
    let result = validate_expression(&expr, &ctx);

    assert!(result.is_ok());
    let typed = result.unwrap();
    assert_eq!(typed.return_type, ExprType::Boolean);
}

#[test]
fn test_type_inference_literal_null() {
    let ctx = CompilationContext::new();
    let expr = parse_expression("NULL").expect("Failed to parse");
    let result = validate_expression(&expr, &ctx);

    assert!(result.is_ok());
    let typed = result.unwrap();
    assert_eq!(typed.return_type, ExprType::Null);
}

#[test]
fn test_type_inference_column_float() {
    let mut ctx = CompilationContext::new();
    ctx.add_column("transactions.amount", ColumnType::Float);

    let expr = parse_expression("transactions.amount").expect("Failed to parse");
    let result = validate_expression(&expr, &ctx);

    assert!(result.is_ok());
    let typed = result.unwrap();
    assert_eq!(typed.return_type, ExprType::Number);
}

#[test]
fn test_type_inference_column_string() {
    let mut ctx = CompilationContext::new();
    ctx.add_column("users.name", ColumnType::String);

    let expr = parse_expression("users.name").expect("Failed to parse");
    let result = validate_expression(&expr, &ctx);

    assert!(result.is_ok());
    let typed = result.unwrap();
    assert_eq!(typed.return_type, ExprType::String);
}

#[test]
fn test_type_inference_column_boolean() {
    let mut ctx = CompilationContext::new();
    ctx.add_column("users.is_active", ColumnType::Boolean);

    let expr = parse_expression("users.is_active").expect("Failed to parse");
    let result = validate_expression(&expr, &ctx);

    assert!(result.is_ok());
    let typed = result.unwrap();
    assert_eq!(typed.return_type, ExprType::Boolean);
}

#[test]
fn test_type_inference_binary_arithmetic_add() {
    let mut ctx = CompilationContext::new();
    ctx.add_column("transactions.amount", ColumnType::Float);

    let expr = parse_expression("transactions.amount + 10").expect("Failed to parse");
    let result = validate_expression(&expr, &ctx);

    assert!(result.is_ok());
    let typed = result.unwrap();
    assert_eq!(typed.return_type, ExprType::Number);
}

#[test]
fn test_type_inference_binary_arithmetic_multiply() {
    let ctx = CompilationContext::new();
    let expr = parse_expression("5 * 2.5").expect("Failed to parse");
    let result = validate_expression(&expr, &ctx);

    assert!(result.is_ok());
    let typed = result.unwrap();
    assert_eq!(typed.return_type, ExprType::Number);
}

#[test]
fn test_type_inference_binary_comparison() {
    let mut ctx = CompilationContext::new();
    ctx.add_column("transactions.amount", ColumnType::Float);

    let expr = parse_expression("transactions.amount > 100").expect("Failed to parse");
    let result = validate_expression(&expr, &ctx);

    assert!(result.is_ok());
    let typed = result.unwrap();
    assert_eq!(typed.return_type, ExprType::Boolean);
}

#[test]
fn test_type_inference_binary_logical() {
    let ctx = CompilationContext::new();
    let expr = parse_expression("TRUE AND FALSE").expect("Failed to parse");
    let result = validate_expression(&expr, &ctx);

    assert!(result.is_ok());
    let typed = result.unwrap();
    assert_eq!(typed.return_type, ExprType::Boolean);
}

#[test]
fn test_type_inference_unary_not() {
    let ctx = CompilationContext::new();
    let expr = parse_expression("NOT TRUE").expect("Failed to parse");
    let result = validate_expression(&expr, &ctx);

    assert!(result.is_ok());
    let typed = result.unwrap();
    assert_eq!(typed.return_type, ExprType::Boolean);
}

#[test]
fn test_type_inference_unary_negate() {
    let ctx = CompilationContext::new();
    let expr = parse_expression("-42").expect("Failed to parse");
    let result = validate_expression(&expr, &ctx);

    assert!(result.is_ok());
    let typed = result.unwrap();
    assert_eq!(typed.return_type, ExprType::Number);
}

#[test]
fn test_type_inference_function_abs() {
    let ctx = CompilationContext::new();
    let expr = parse_expression("ABS(-5)").expect("Failed to parse");
    let result = validate_expression(&expr, &ctx);

    assert!(result.is_ok());
    let typed = result.unwrap();
    assert_eq!(typed.return_type, ExprType::Number);
}

#[test]
fn test_type_inference_function_upper() {
    let ctx = CompilationContext::new();
    let expr = parse_expression(r#"UPPER("hello")"#).expect("Failed to parse");
    let result = validate_expression(&expr, &ctx);

    assert!(result.is_ok());
    let typed = result.unwrap();
    assert_eq!(typed.return_type, ExprType::String);
}

#[test]
fn test_type_inference_function_concat() {
    let ctx = CompilationContext::new();
    let expr = parse_expression(r#"CONCAT("hello", " ", "world")"#).expect("Failed to parse");
    let result = validate_expression(&expr, &ctx);

    assert!(result.is_ok());
    let typed = result.unwrap();
    assert_eq!(typed.return_type, ExprType::String);
}

#[test]
fn test_type_inference_function_isnull() {
    let ctx = CompilationContext::new();
    let expr = parse_expression("ISNULL(NULL)").expect("Failed to parse");
    let result = validate_expression(&expr, &ctx);

    assert!(result.is_ok());
    let typed = result.unwrap();
    assert_eq!(typed.return_type, ExprType::Boolean);
}

#[test]
fn test_type_inference_function_sum_aggregate() {
    let mut ctx = CompilationContext::new();
    ctx.add_column("transactions.amount", ColumnType::Float);
    ctx = ctx.with_aggregates(true);

    let expr = parse_expression("SUM(transactions.amount)").expect("Failed to parse");
    let result = validate_expression(&expr, &ctx);

    assert!(result.is_ok());
    let typed = result.unwrap();
    assert_eq!(typed.return_type, ExprType::Number);
}

// ============================================================================
// T031: Unit test for type checking
// ============================================================================

#[test]
fn test_type_check_arithmetic_requires_number() {
    let mut ctx = CompilationContext::new();
    ctx.add_column("transactions.amount", ColumnType::Float);

    // Valid: number + number
    let expr = parse_expression("10 + 5").expect("Failed to parse");
    let result = validate_expression(&expr, &ctx);
    assert!(result.is_ok(), "Number + Number should be valid");
}

#[test]
fn test_type_check_arithmetic_invalid_string() {
    let mut ctx = CompilationContext::new();
    ctx.add_column("users.name", ColumnType::String);

    // Invalid: string + number
    let expr = parse_expression(r#"users.name + 5"#).expect("Failed to parse");
    let result = validate_expression(&expr, &ctx);

    assert!(
        result.is_err(),
        "String + Number should fail type check"
    );
    match result {
        Err(ValidationError::TypeMismatch { expected, actual, .. }) => {
            assert_eq!(expected, "Number");
        }
        _ => panic!("Expected TypeMismatch error"),
    }
}

#[test]
fn test_type_check_logical_requires_boolean() {
    let ctx = CompilationContext::new();

    // Valid: boolean AND boolean
    let expr = parse_expression("TRUE AND FALSE").expect("Failed to parse");
    let result = validate_expression(&expr, &ctx);
    assert!(result.is_ok(), "Boolean AND Boolean should be valid");
}

#[test]
fn test_type_check_logical_invalid_number() {
    let ctx = CompilationContext::new();

    // Invalid: number AND boolean
    let expr = parse_expression("5 AND TRUE").expect("Failed to parse");
    let result = validate_expression(&expr, &ctx);

    assert!(
        result.is_err(),
        "Number AND Boolean should fail type check"
    );
}

#[test]
fn test_type_check_comparison_type_mismatch() {
    let mut ctx = CompilationContext::new();
    ctx.add_column("users.name", ColumnType::String);

    // Comparing string with number (edge case, may be allowed depending on coercion rules)
    let expr = parse_expression(r#"users.name > 5"#).expect("Failed to parse");
    let result = validate_expression(&expr, &ctx);

    // This may pass or fail depending on implementation; document behavior
    // For now, we test that validation runs without panic
    let _ = result;
}

#[test]
fn test_type_check_multiply_requires_number() {
    let ctx = CompilationContext::new();

    // Valid: number * number
    let expr = parse_expression("10 * 5").expect("Failed to parse");
    let result = validate_expression(&expr, &ctx);
    assert!(result.is_ok(), "Number * Number should be valid");
}

#[test]
fn test_type_check_divide_requires_number() {
    let ctx = CompilationContext::new();

    // Valid: number / number
    let expr = parse_expression("10 / 5").expect("Failed to parse");
    let result = validate_expression(&expr, &ctx);
    assert!(result.is_ok(), "Number / Number should be valid");
}

#[test]
fn test_type_check_function_argument_type() {
    let ctx = CompilationContext::new();

    // ABS expects a number
    let expr = parse_expression("ABS(5)").expect("Failed to parse");
    let result = validate_expression(&expr, &ctx);
    assert!(result.is_ok(), "ABS(number) should be valid");
}

#[test]
fn test_type_check_not_requires_boolean() {
    let ctx = CompilationContext::new();

    // Valid: NOT boolean
    let expr = parse_expression("NOT TRUE").expect("Failed to parse");
    let result = validate_expression(&expr, &ctx);
    assert!(result.is_ok(), "NOT boolean should be valid");
}

#[test]
fn test_type_check_null_compatible_all_types() {
    let ctx = CompilationContext::new();

    // NULL + number should be valid (NULL is compatible with any type)
    let expr = parse_expression("NULL + 5").expect("Failed to parse");
    let result = validate_expression(&expr, &ctx);
    assert!(result.is_ok(), "NULL + Number should be valid (NULL is compatible)");
}

// ============================================================================
// T032: Unit test for aggregate context validation
// ============================================================================

#[test]
fn test_aggregate_context_sum_allowed() {
    let mut ctx = CompilationContext::new();
    ctx.add_column("transactions.amount", ColumnType::Float);
    ctx = ctx.with_aggregates(true);

    let expr = parse_expression("SUM(transactions.amount)").expect("Failed to parse");
    let result = validate_expression(&expr, &ctx);

    assert!(
        result.is_ok(),
        "SUM should be allowed when allow_aggregates=true"
    );
}

#[test]
fn test_aggregate_context_sum_not_allowed() {
    let mut ctx = CompilationContext::new();
    ctx.add_column("transactions.amount", ColumnType::Float);
    ctx = ctx.with_aggregates(false);

    let expr = parse_expression("SUM(transactions.amount)").expect("Failed to parse");
    let result = validate_expression(&expr, &ctx);

    assert!(
        result.is_err(),
        "SUM should fail when allow_aggregates=false"
    );
    match result {
        Err(ValidationError::InvalidAggregateContext { function }) => {
            assert_eq!(function, "SUM");
        }
        _ => panic!("Expected InvalidAggregateContext error"),
    }
}

#[test]
fn test_aggregate_context_count_allowed() {
    let mut ctx = CompilationContext::new();
    ctx.add_column("transactions.id", ColumnType::Integer);
    ctx = ctx.with_aggregates(true);

    let expr = parse_expression("COUNT(transactions.id)").expect("Failed to parse");
    let result = validate_expression(&expr, &ctx);

    assert!(
        result.is_ok(),
        "COUNT should be allowed when allow_aggregates=true"
    );
}

#[test]
fn test_aggregate_context_count_not_allowed() {
    let mut ctx = CompilationContext::new();
    ctx.add_column("transactions.id", ColumnType::Integer);
    ctx = ctx.with_aggregates(false);

    let expr = parse_expression("COUNT(transactions.id)").expect("Failed to parse");
    let result = validate_expression(&expr, &ctx);

    assert!(
        result.is_err(),
        "COUNT should fail when allow_aggregates=false"
    );
}

#[test]
fn test_aggregate_context_avg_allowed() {
    let mut ctx = CompilationContext::new();
    ctx.add_column("transactions.amount", ColumnType::Float);
    ctx = ctx.with_aggregates(true);

    let expr = parse_expression("AVG(transactions.amount)").expect("Failed to parse");
    let result = validate_expression(&expr, &ctx);

    assert!(
        result.is_ok(),
        "AVG should be allowed when allow_aggregates=true"
    );
}

#[test]
fn test_aggregate_context_avg_not_allowed() {
    let mut ctx = CompilationContext::new();
    ctx.add_column("transactions.amount", ColumnType::Float);
    ctx = ctx.with_aggregates(false);

    let expr = parse_expression("AVG(transactions.amount)").expect("Failed to parse");
    let result = validate_expression(&expr, &ctx);

    assert!(
        result.is_err(),
        "AVG should fail when allow_aggregates=false"
    );
}

#[test]
fn test_aggregate_context_regular_functions_allowed() {
    let ctx = CompilationContext::new().with_aggregates(false);

    // Regular functions should work regardless of allow_aggregates
    let expr = parse_expression("ABS(-5)").expect("Failed to parse");
    let result = validate_expression(&expr, &ctx);

    assert!(result.is_ok(), "Regular functions should work without aggregates");
}

#[test]
fn test_aggregate_context_multiple_aggregates() {
    let mut ctx = CompilationContext::new();
    ctx.add_column("transactions.amount", ColumnType::Float);
    ctx = ctx.with_aggregates(true);

    // Multiple aggregates in one expression
    let expr = parse_expression("SUM(transactions.amount) + COUNT(transactions.amount)")
        .expect("Failed to parse");
    let result = validate_expression(&expr, &ctx);

    assert!(result.is_ok(), "Multiple aggregates should work when allowed");
}

// ============================================================================
// T033: Unit test for selector interpolation
// ============================================================================

#[test]
fn test_selector_interpolation_simple() {
    let mut ctx = CompilationContext::new();
    ctx.add_column("transactions.amount", ColumnType::Float);
    ctx.add_selector("total_amount", "SUM(transactions.amount)");

    // Interpolate selector reference {total_amount}
    let result = interpolate_selectors("{total_amount}", &ctx);

    assert!(
        result.is_ok(),
        "Simple selector interpolation should succeed"
    );
    let interpolated = result.unwrap();
    assert!(interpolated.contains("SUM"));
}

#[test]
fn test_selector_interpolation_nested() {
    let mut ctx = CompilationContext::new();
    ctx.add_column("transactions.amount", ColumnType::Float);
    ctx.add_selector("base_amount", "transactions.amount");
    ctx.add_selector("total_amount", "SUM({base_amount})");

    // Nested selector: total_amount references base_amount
    let result = interpolate_selectors("{total_amount}", &ctx);

    assert!(result.is_ok(), "Nested selector interpolation should succeed");
    let interpolated = result.unwrap();
    assert!(interpolated.contains("transactions.amount"));
}

#[test]
fn test_selector_interpolation_circular_simple() {
    let mut ctx = CompilationContext::new();
    // Direct circular reference: a -> a
    ctx.add_selector("recursive", "{recursive}");

    let result = interpolate_selectors("{recursive}", &ctx);

    assert!(
        result.is_err(),
        "Circular selector reference should be detected"
    );
    match result {
        Err(ValidationError::CircularSelectorReference { .. }) => {
            // Success: circular reference detected
        }
        _ => panic!("Expected CircularSelectorReference error"),
    }
}

#[test]
fn test_selector_interpolation_circular_indirect() {
    let mut ctx = CompilationContext::new();
    // Indirect circular: a -> b -> a
    ctx.add_selector("selector_a", "{selector_b}");
    ctx.add_selector("selector_b", "{selector_a}");

    let result = interpolate_selectors("{selector_a}", &ctx);

    assert!(result.is_err(), "Indirect circular reference should be detected");
}

#[test]
fn test_selector_interpolation_multiple_selectors() {
    let mut ctx = CompilationContext::new();
    ctx.add_column("transactions.amount", ColumnType::Float);
    ctx.add_selector("sum_amount", "SUM(transactions.amount)");
    ctx.add_selector("avg_amount", "AVG(transactions.amount)");

    // Expression with multiple selector references
    let result = interpolate_selectors("{sum_amount} + {avg_amount}", &ctx);

    assert!(result.is_ok(), "Multiple selector references should work");
}

#[test]
fn test_selector_interpolation_unresolved() {
    let ctx = CompilationContext::new();

    // Reference undefined selector
    let result = interpolate_selectors("{undefined}", &ctx);

    assert!(result.is_err(), "Unresolved selector should fail");
    match result {
        Err(ValidationError::UnresolvedSelector { selector }) => {
            assert_eq!(selector, "undefined");
        }
        _ => panic!("Expected UnresolvedSelector error"),
    }
}

#[test]
fn test_selector_interpolation_no_selectors() {
    let ctx = CompilationContext::new();

    // Expression without any selector references
    let result = interpolate_selectors("42", &ctx);

    assert!(result.is_ok(), "Expression without selectors should pass");
    assert_eq!(result.unwrap(), "42");
}

// ============================================================================
// T034: Unit test for selector edge cases
// ============================================================================

#[test]
fn test_selector_edge_case_unresolved_column_in_selector() {
    let mut ctx = CompilationContext::new();
    ctx.add_selector("missing_col", "missing.column");
    ctx.add_column("real.column", ColumnType::Float);

    // Interpolate first (should work)
    let interpolated = interpolate_selectors("{missing_col}", &ctx)
        .expect("Interpolation should work");

    // Then validate the interpolated expression
    let expr = parse_expression(&interpolated).expect("Failed to parse");
    let result = validate_expression(&expr, &ctx);

    assert!(result.is_err(), "Unresolved column in selector should fail validation");
}

#[test]
fn test_selector_edge_case_max_depth() {
    let mut ctx = CompilationContext::new();
    ctx.add_column("transactions.amount", ColumnType::Float);

    // Create a chain of selectors: a -> b -> c -> d -> e (depth 4)
    ctx.add_selector("a", "{b}");
    ctx.add_selector("b", "{c}");
    ctx.add_selector("c", "{d}");
    ctx.add_selector("d", "{e}");
    ctx.add_selector("e", "transactions.amount");

    let result = interpolate_selectors("{a}", &ctx);

    // Should either succeed or fail gracefully with max depth error
    if result.is_err() {
        match result {
            Err(ValidationError::MaxInterpolationDepth { .. }) => {
                // Success: max depth exceeded
            }
            _ => panic!("Expected MaxInterpolationDepth or success"),
        }
    }
}

#[test]
fn test_selector_edge_case_empty_selector_name() {
    let ctx = CompilationContext::new();

    // Empty selector reference
    let result = interpolate_selectors("{}", &ctx);

    // Should fail gracefully
    assert!(result.is_err(), "Empty selector name should fail");
}

#[test]
fn test_selector_edge_case_special_characters_in_name() {
    let mut ctx = CompilationContext::new();
    ctx.add_column("transactions.amount", ColumnType::Float);
    ctx.add_selector("my_selector_123", "transactions.amount");

    let result = interpolate_selectors("{my_selector_123}", &ctx);

    assert!(
        result.is_ok(),
        "Alphanumeric selector names with underscores should work"
    );
}

#[test]
fn test_selector_edge_case_whitespace_handling() {
    let mut ctx = CompilationContext::new();
    ctx.add_column("transactions.amount", ColumnType::Float);
    ctx.add_selector("total", "transactions.amount");

    // Selector with extra whitespace
    let result = interpolate_selectors("{ total }", &ctx);

    // Should handle whitespace gracefully (or error clearly)
    let _ = result;
}

// ============================================================================
// T035: Integration test for end-to-end validation
// ============================================================================

#[test]
fn test_end_to_end_simple_column_reference() {
    // Full pipeline: parse -> validate
    let mut ctx = CompilationContext::new();
    ctx.add_column("transactions.amount", ColumnType::Float);

    // Parse
    let expr = parse_expression("transactions.amount").expect("Parse failed");

    // Validate
    let result = validate_expression(&expr, &ctx);

    assert!(result.is_ok(), "Simple column reference pipeline should work");
    let typed = result.unwrap();
    assert_eq!(typed.return_type, ExprType::Number);
}

#[test]
fn test_end_to_end_arithmetic_expression() {
    let mut ctx = CompilationContext::new();
    ctx.add_column("transactions.amount", ColumnType::Float);
    ctx.add_column("transactions.tax_rate", ColumnType::Float);

    // Parse: transactions.amount * (1 + transactions.tax_rate)
    let expr = parse_expression("transactions.amount * (1 + transactions.tax_rate)")
        .expect("Parse failed");

    // Validate
    let result = validate_expression(&expr, &ctx);

    assert!(result.is_ok(), "Arithmetic expression pipeline should work");
    let typed = result.unwrap();
    assert_eq!(typed.return_type, ExprType::Number);
}

#[test]
fn test_end_to_end_comparison_expression() {
    let mut ctx = CompilationContext::new();
    ctx.add_column("transactions.amount", ColumnType::Float);

    // Parse: transactions.amount > 1000
    let expr = parse_expression("transactions.amount > 1000").expect("Parse failed");

    // Validate
    let result = validate_expression(&expr, &ctx);

    assert!(result.is_ok(), "Comparison expression pipeline should work");
    let typed = result.unwrap();
    assert_eq!(typed.return_type, ExprType::Boolean);
}

#[test]
fn test_end_to_end_aggregate_with_context() {
    let mut ctx = CompilationContext::new();
    ctx.add_column("transactions.amount", ColumnType::Float);
    ctx = ctx.with_aggregates(true);

    // Parse: SUM(transactions.amount)
    let expr = parse_expression("SUM(transactions.amount)").expect("Parse failed");

    // Validate
    let result = validate_expression(&expr, &ctx);

    assert!(result.is_ok(), "Aggregate pipeline should work with context");
    let typed = result.unwrap();
    assert_eq!(typed.return_type, ExprType::Number);
}

#[test]
fn test_end_to_end_aggregate_without_context() {
    let mut ctx = CompilationContext::new();
    ctx.add_column("transactions.amount", ColumnType::Float);
    ctx = ctx.with_aggregates(false);

    // Parse: SUM(transactions.amount)
    let expr = parse_expression("SUM(transactions.amount)").expect("Parse failed");

    // Validate
    let result = validate_expression(&expr, &ctx);

    assert!(
        result.is_err(),
        "Aggregate pipeline should fail without aggregate context"
    );
}

#[test]
fn test_end_to_end_with_selector_interpolation() {
    let mut ctx = CompilationContext::new();
    ctx.add_column("transactions.amount", ColumnType::Float);
    ctx.add_selector("total_amount", "SUM(transactions.amount)");
    ctx = ctx.with_aggregates(true);

    // Interpolate: {total_amount} -> SUM(transactions.amount)
    let interpolated = interpolate_selectors("{total_amount}", &ctx)
        .expect("Interpolation failed");

    // Parse
    let expr = parse_expression(&interpolated).expect("Parse failed");

    // Validate
    let result = validate_expression(&expr, &ctx);

    assert!(
        result.is_ok(),
        "Full pipeline with selector interpolation should work"
    );
}

#[test]
fn test_end_to_end_complex_expression() {
    let mut ctx = CompilationContext::new();
    ctx.add_column("transactions.amount", ColumnType::Float);
    ctx.add_column("transactions.quantity", ColumnType::Integer);
    ctx = ctx.with_aggregates(true);

    // Parse: SUM(transactions.amount) / COUNT(transactions.quantity) > 100
    let expr = parse_expression(
        "SUM(transactions.amount) / COUNT(transactions.quantity) > 100",
    )
    .expect("Parse failed");

    // Validate
    let result = validate_expression(&expr, &ctx);

    assert!(result.is_ok(), "Complex expression pipeline should work");
    let typed = result.unwrap();
    assert_eq!(typed.return_type, ExprType::Boolean);
}

#[test]
fn test_end_to_end_validation_error_accumulation() {
    let mut ctx = CompilationContext::new();
    ctx.add_column("transactions.amount", ColumnType::Float);

    // Parse: invalid_table.missing + 5
    let expr = parse_expression("invalid_table.missing + 5").expect("Parse failed");

    // Validate
    let result = validate_expression(&expr, &ctx);

    assert!(result.is_err(), "Missing column should cause validation error");
    match result {
        Err(ValidationError::UnresolvedColumnRef { .. }) => {
            // Success: error properly identified
        }
        _ => panic!("Expected UnresolvedColumnRef error"),
    }
}

// ============================================================================
// Helper tests to ensure test infrastructure
// ============================================================================

#[test]
fn test_compilation_context_creation() {
    let ctx = CompilationContext::new();
    assert!(ctx.schema.is_empty());
    assert!(ctx.selectors.is_empty());
    assert!(!ctx.allow_aggregates);
}

#[test]
fn test_compilation_context_schema() {
    let mut ctx = CompilationContext::new();
    ctx.add_column("users.id", ColumnType::Integer);
    ctx.add_column("users.name", ColumnType::String);

    assert_eq!(ctx.schema.len(), 2);
    assert!(ctx.get_column("users.id").is_some());
    assert!(ctx.get_column("nonexistent").is_none());
}
