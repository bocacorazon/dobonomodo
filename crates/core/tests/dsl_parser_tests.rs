//! Unit tests for DSL parser
//!
//! These tests verify the parser can correctly transform expression strings
//! into AST nodes for all supported syntax elements.

use dobo_core::dsl::*;

// T012: Unit test for literal parsing
#[test]
fn test_parse_literal_number() {
    // Integer
    let ast = parse_expression("42").expect("Failed to parse number");
    assert!(matches!(ast, ExprAST::Literal(LiteralValue::Number(n)) if n == 42.0));

    // Float
    let ast = parse_expression("3.5").expect("Failed to parse float");
    assert!(matches!(ast, ExprAST::Literal(LiteralValue::Number(n)) if (n - 3.5).abs() < 0.001));

    // Negative number
    let ast = parse_expression("-10.5").expect("Failed to parse negative number");
    match ast {
        ExprAST::UnaryOp { op, operand } => {
            assert_eq!(op, UnaryOperator::Negate);
            assert!(
                matches!(*operand, ExprAST::Literal(LiteralValue::Number(n)) if (n - 10.5).abs() < 0.001)
            );
        }
        _ => panic!("Expected UnaryOp for negative number"),
    }
}

#[test]
fn test_parse_literal_string() {
    let ast = parse_expression(r#""hello world""#).expect("Failed to parse string");
    assert!(matches!(ast, ExprAST::Literal(LiteralValue::String(s)) if s == "hello world"));

    // String with quotes
    let ast = parse_expression(r#""it's ok""#).expect("Failed to parse string with quote");
    assert!(matches!(ast, ExprAST::Literal(LiteralValue::String(s)) if s == "it's ok"));
}

#[test]
fn test_parse_literal_boolean() {
    let ast = parse_expression("TRUE").expect("Failed to parse TRUE");
    assert!(matches!(ast, ExprAST::Literal(LiteralValue::Boolean(true))));

    let ast = parse_expression("FALSE").expect("Failed to parse FALSE");
    assert!(matches!(
        ast,
        ExprAST::Literal(LiteralValue::Boolean(false))
    ));

    // Case insensitive
    let ast = parse_expression("true").expect("Failed to parse true");
    assert!(matches!(ast, ExprAST::Literal(LiteralValue::Boolean(true))));
}

#[test]
fn test_parse_literal_date() {
    let ast = parse_expression(r#"DATE("2024-01-15")"#).expect("Failed to parse date");
    assert!(matches!(
        ast,
        ExprAST::Literal(LiteralValue::Date(d)) if d == chrono::NaiveDate::from_ymd_opt(2024, 1, 15).expect("valid date")
    ));
}

#[test]
fn test_parse_literal_null() {
    let ast = parse_expression("NULL").expect("Failed to parse NULL");
    assert!(matches!(ast, ExprAST::Literal(LiteralValue::Null)));
}

// T013: Unit test for column reference parsing
#[test]
fn test_parse_column_reference() {
    let ast = parse_expression("transactions.amount").expect("Failed to parse column ref");
    match ast {
        ExprAST::ColumnRef { table, column } => {
            assert_eq!(table, "transactions");
            assert_eq!(column, "amount");
        }
        _ => panic!("Expected ColumnRef"),
    }
}

#[test]
fn test_parse_column_reference_with_underscore() {
    let ast = parse_expression("customer_data.first_name").expect("Failed to parse column ref");
    match ast {
        ExprAST::ColumnRef { table, column } => {
            assert_eq!(table, "customer_data");
            assert_eq!(column, "first_name");
        }
        _ => panic!("Expected ColumnRef"),
    }
}

// T014: Unit test for binary operator parsing
#[test]
fn test_parse_arithmetic_operators() {
    // Addition
    let ast = parse_expression("1 + 2").expect("Failed to parse addition");
    match ast {
        ExprAST::BinaryOp { op, .. } => assert_eq!(op, BinaryOperator::Add),
        _ => panic!("Expected BinaryOp"),
    }

    // Subtraction
    let ast = parse_expression("5 - 3").expect("Failed to parse subtraction");
    match ast {
        ExprAST::BinaryOp { op, .. } => assert_eq!(op, BinaryOperator::Subtract),
        _ => panic!("Expected BinaryOp"),
    }

    // Multiplication
    let ast = parse_expression("4 * 2").expect("Failed to parse multiplication");
    match ast {
        ExprAST::BinaryOp { op, .. } => assert_eq!(op, BinaryOperator::Multiply),
        _ => panic!("Expected BinaryOp"),
    }

    // Division
    let ast = parse_expression("10 / 2").expect("Failed to parse division");
    match ast {
        ExprAST::BinaryOp { op, .. } => assert_eq!(op, BinaryOperator::Divide),
        _ => panic!("Expected BinaryOp"),
    }
}

#[test]
fn test_parse_comparison_operators() {
    // Equal
    let ast = parse_expression("transactions.x = 5").expect("Failed to parse equals");
    match ast {
        ExprAST::BinaryOp { op, .. } => assert_eq!(op, BinaryOperator::Equal),
        _ => panic!("Expected BinaryOp"),
    }

    // Not equal
    let ast = parse_expression("transactions.x <> 5").expect("Failed to parse not equals");
    match ast {
        ExprAST::BinaryOp { op, .. } => assert_eq!(op, BinaryOperator::NotEqual),
        _ => panic!("Expected BinaryOp"),
    }

    // Less than
    let ast = parse_expression("transactions.x < 5").expect("Failed to parse less than");
    match ast {
        ExprAST::BinaryOp { op, .. } => assert_eq!(op, BinaryOperator::LessThan),
        _ => panic!("Expected BinaryOp"),
    }

    // Greater than
    let ast = parse_expression("transactions.x > 5").expect("Failed to parse greater than");
    match ast {
        ExprAST::BinaryOp { op, .. } => assert_eq!(op, BinaryOperator::GreaterThan),
        _ => panic!("Expected BinaryOp"),
    }
}

#[test]
fn test_parse_logical_operators() {
    // AND
    let ast = parse_expression("TRUE AND FALSE").expect("Failed to parse AND");
    match ast {
        ExprAST::BinaryOp { op, .. } => assert_eq!(op, BinaryOperator::And),
        _ => panic!("Expected BinaryOp"),
    }

    // OR
    let ast = parse_expression("TRUE OR FALSE").expect("Failed to parse OR");
    match ast {
        ExprAST::BinaryOp { op, .. } => assert_eq!(op, BinaryOperator::Or),
        _ => panic!("Expected BinaryOp"),
    }
}

// T015: Unit test for unary operator parsing
#[test]
fn test_parse_unary_not() {
    let ast = parse_expression("NOT TRUE").expect("Failed to parse NOT");
    match ast {
        ExprAST::UnaryOp { op, operand } => {
            assert_eq!(op, UnaryOperator::Not);
            assert!(matches!(
                *operand,
                ExprAST::Literal(LiteralValue::Boolean(true))
            ));
        }
        _ => panic!("Expected UnaryOp"),
    }
}

#[test]
fn test_parse_unary_negate() {
    let ast = parse_expression("-42").expect("Failed to parse negation");
    match ast {
        ExprAST::UnaryOp { op, operand } => {
            assert_eq!(op, UnaryOperator::Negate);
            assert!(matches!(*operand, ExprAST::Literal(LiteralValue::Number(n)) if n == 42.0));
        }
        _ => panic!("Expected UnaryOp"),
    }
}

// T016: Unit test for function call parsing
#[test]
fn test_parse_function_zero_args() {
    let ast = parse_expression("TODAY()").expect("Failed to parse function with no args");
    match ast {
        ExprAST::FunctionCall { name, args } => {
            assert_eq!(name, "TODAY");
            assert_eq!(args.len(), 0);
        }
        _ => panic!("Expected FunctionCall"),
    }
}

#[test]
fn test_parse_function_one_arg() {
    let ast = parse_expression("ABS(-5)").expect("Failed to parse function with one arg");
    match ast {
        ExprAST::FunctionCall { name, args } => {
            assert_eq!(name, "ABS");
            assert_eq!(args.len(), 1);
        }
        _ => panic!("Expected FunctionCall"),
    }
}

#[test]
fn test_parse_function_multiple_args() {
    let ast = parse_expression(r#"CONCAT("hello", " ", "world")"#)
        .expect("Failed to parse function with multiple args");
    match ast {
        ExprAST::FunctionCall { name, args } => {
            assert_eq!(name, "CONCAT");
            assert_eq!(args.len(), 3);
        }
        _ => panic!("Expected FunctionCall"),
    }
}

#[test]
fn test_parse_function_nested() {
    let ast = parse_expression("ABS(MIN(transactions.a, transactions.b))")
        .expect("Failed to parse nested function");
    match ast {
        ExprAST::FunctionCall { name, args } => {
            assert_eq!(name, "ABS");
            assert_eq!(args.len(), 1);
            match &args[0] {
                ExprAST::FunctionCall { name, args } => {
                    assert_eq!(name, "MIN");
                    assert_eq!(args.len(), 2);
                }
                _ => panic!("Expected nested FunctionCall"),
            }
        }
        _ => panic!("Expected FunctionCall"),
    }
}

#[test]
fn test_parse_boolean_functions() {
    for source in [
        "AND(TRUE, FALSE)",
        "OR(TRUE, FALSE)",
        "NOT(TRUE)",
        "AND(transactions.flag, OR(TRUE, FALSE))",
    ] {
        let ast = parse_expression(source).expect("Failed to parse boolean function");
        assert!(matches!(ast, ExprAST::FunctionCall { .. }));
    }
}

// T017: Unit test for operator precedence
#[test]
fn test_operator_precedence_arithmetic() {
    // 1 + 2 * 3 should parse as 1 + (2 * 3)
    let ast = parse_expression("1 + 2 * 3").expect("Failed to parse precedence");
    match ast {
        ExprAST::BinaryOp { op, left, right } => {
            assert_eq!(op, BinaryOperator::Add);
            assert!(matches!(*left, ExprAST::Literal(LiteralValue::Number(n)) if n == 1.0));
            match *right {
                ExprAST::BinaryOp { op, .. } => {
                    assert_eq!(op, BinaryOperator::Multiply);
                }
                _ => panic!("Expected multiply on right"),
            }
        }
        _ => panic!("Expected BinaryOp"),
    }
}

#[test]
fn test_operator_precedence_comparison() {
    // 1 + 2 < 5 should parse as (1 + 2) < 5
    let ast = parse_expression("1 + 2 < 5").expect("Failed to parse precedence");
    match ast {
        ExprAST::BinaryOp { op, left, .. } => {
            assert_eq!(op, BinaryOperator::LessThan);
            match *left {
                ExprAST::BinaryOp { op, .. } => {
                    assert_eq!(op, BinaryOperator::Add);
                }
                _ => panic!("Expected add on left"),
            }
        }
        _ => panic!("Expected BinaryOp"),
    }
}

#[test]
fn test_operator_precedence_logical() {
    // TRUE OR FALSE AND TRUE should parse as TRUE OR (FALSE AND TRUE)
    let ast = parse_expression("TRUE OR FALSE AND TRUE").expect("Failed to parse precedence");
    match ast {
        ExprAST::BinaryOp { op, right, .. } => {
            assert_eq!(op, BinaryOperator::Or);
            match *right {
                ExprAST::BinaryOp { op, .. } => {
                    assert_eq!(op, BinaryOperator::And);
                }
                _ => panic!("Expected AND on right"),
            }
        }
        _ => panic!("Expected BinaryOp"),
    }
}

// T018: Unit test for parentheses grouping
#[test]
fn test_parentheses_override_precedence() {
    // (1 + 2) * 3 should parse differently from 1 + 2 * 3
    let ast = parse_expression("(1 + 2) * 3").expect("Failed to parse with parentheses");
    match ast {
        ExprAST::BinaryOp { op, left, .. } => {
            assert_eq!(op, BinaryOperator::Multiply);
            match *left {
                ExprAST::BinaryOp { op, .. } => {
                    assert_eq!(op, BinaryOperator::Add);
                }
                _ => panic!("Expected add on left"),
            }
        }
        _ => panic!("Expected BinaryOp"),
    }
}

#[test]
fn test_nested_parentheses() {
    let ast = parse_expression("((1 + 2) * 3)").expect("Failed to parse nested parentheses");
    match ast {
        ExprAST::BinaryOp { op, .. } => {
            assert_eq!(op, BinaryOperator::Multiply);
        }
        _ => panic!("Expected BinaryOp"),
    }
}

// T019: Unit test for parse error cases
#[test]
fn test_parse_error_unclosed_string() {
    let result = parse_expression(r#""unclosed string"#);
    assert!(result.is_err());
    // Should contain position information
}

#[test]
fn test_parse_error_unclosed_paren() {
    let result = parse_expression("(1 + 2");
    assert!(result.is_err());
}

#[test]
fn test_parse_error_invalid_token() {
    let err = parse_expression("1 + @").expect_err("invalid token should fail");
    match err {
        ParseError::UnexpectedToken {
            token,
            line,
            column,
        } => {
            assert_eq!(line, 1);
            assert!(column >= 5);
            assert!(token.contains('@'));
            assert!(token.contains("expected:"));
        }
        other => panic!("Expected UnexpectedToken, got {other:?}"),
    }
}

#[test]
fn test_parse_error_incomplete_expression() {
    let err = parse_expression("1 +").expect_err("incomplete expression should fail");
    match err {
        ParseError::UnexpectedToken {
            token,
            line,
            column,
        } => {
            assert_eq!(line, 1);
            assert!(column >= 3);
            assert!(token.contains("<eof>"));
            assert!(token.contains("expected:"));
        }
        other => panic!("Expected UnexpectedToken, got {other:?}"),
    }
}

#[test]
fn test_parse_error_bare_identifier_not_allowed() {
    let result = parse_expression("just_identifier");
    assert!(result.is_err());
}

// T020: Unit test for position tracking
#[test]
fn test_position_tracking() {
    // Parse with span to get position information
    let result = parse_expression_with_span("1 + @");
    assert!(result.is_err());

    if let Err(e) = result {
        let err_str = e.to_string();
        // Should contain line and column information
        assert!(err_str.contains("line") || err_str.contains("column"));
    }
}
