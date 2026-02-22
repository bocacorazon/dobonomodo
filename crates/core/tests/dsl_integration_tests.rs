//! Integration tests for DSL parser
//!
//! These tests verify end-to-end parsing of complete expressions from the spec.

use dobo_core::dsl::*;

// T021: Integration test for sample expressions from spec

#[test]
fn test_sample_expression_arithmetic() {
    // transactions.amount * 1.1
    let ast = parse_expression("transactions.amount * 1.1")
        .expect("Failed to parse arithmetic expression");

    match ast {
        ExprAST::BinaryOp { op, left, right } => {
            assert_eq!(op, BinaryOperator::Multiply);
            assert!(matches!(*left, ExprAST::ColumnRef { .. }));
            assert!(
                matches!(*right, ExprAST::Literal(LiteralValue::Number(n)) if (n - 1.1).abs() < 0.001)
            );
        }
        _ => panic!("Expected BinaryOp"),
    }
}

#[test]
fn test_sample_expression_comparison() {
    // transactions.date >= DATE("2024-01-01")
    let ast = parse_expression(r#"transactions.date >= DATE("2024-01-01")"#)
        .expect("Failed to parse comparison expression");

    match ast {
        ExprAST::BinaryOp { op, left, right } => {
            assert_eq!(op, BinaryOperator::GreaterThanOrEqual);
            assert!(matches!(*left, ExprAST::ColumnRef { .. }));
            assert!(matches!(*right, ExprAST::FunctionCall { .. }));
        }
        _ => panic!("Expected BinaryOp"),
    }
}

#[test]
fn test_sample_expression_logical() {
    // transactions.status = "completed" AND transactions.amount > 0
    let ast = parse_expression(r#"transactions.status = "completed" AND transactions.amount > 0"#)
        .expect("Failed to parse logical expression");

    match ast {
        ExprAST::BinaryOp { op, left, right } => {
            assert_eq!(op, BinaryOperator::And);
            assert!(matches!(
                *left,
                ExprAST::BinaryOp {
                    op: BinaryOperator::Equal,
                    ..
                }
            ));
            assert!(matches!(
                *right,
                ExprAST::BinaryOp {
                    op: BinaryOperator::GreaterThan,
                    ..
                }
            ));
        }
        _ => panic!("Expected BinaryOp"),
    }
}

#[test]
fn test_sample_expression_aggregate() {
    // SUM(transactions.amount)
    let ast =
        parse_expression("SUM(transactions.amount)").expect("Failed to parse aggregate expression");

    match ast {
        ExprAST::FunctionCall { name, args } => {
            assert_eq!(name, "SUM");
            assert_eq!(args.len(), 1);
            assert!(matches!(args[0], ExprAST::ColumnRef { .. }));
        }
        _ => panic!("Expected FunctionCall"),
    }
}

#[test]
fn test_sample_expression_string_function() {
    // CONCAT(customers.first_name, " ", customers.last_name)
    let ast = parse_expression(r#"CONCAT(customers.first_name, " ", customers.last_name)"#)
        .expect("Failed to parse string function expression");

    match ast {
        ExprAST::FunctionCall { name, args } => {
            assert_eq!(name, "CONCAT");
            assert_eq!(args.len(), 3);
        }
        _ => panic!("Expected FunctionCall"),
    }
}

#[test]
fn test_sample_expression_conditional() {
    // IF(transactions.amount > 100, "large", "small")
    let ast = parse_expression(r#"IF(transactions.amount > 100, "large", "small")"#)
        .expect("Failed to parse conditional expression");

    match ast {
        ExprAST::FunctionCall { name, args } => {
            assert_eq!(name, "IF");
            assert_eq!(args.len(), 3);
            assert!(matches!(args[0], ExprAST::BinaryOp { .. }));
        }
        _ => panic!("Expected FunctionCall"),
    }
}

#[test]
fn test_sample_expression_date_functions() {
    // DATEDIFF(transactions.delivery_date, transactions.order_date)
    let ast = parse_expression("DATEDIFF(transactions.delivery_date, transactions.order_date)")
        .expect("Failed to parse date function expression");

    match ast {
        ExprAST::FunctionCall { name, args } => {
            assert_eq!(name, "DATEDIFF");
            assert_eq!(args.len(), 2);
        }
        _ => panic!("Expected FunctionCall"),
    }
}

#[test]
fn test_sample_expression_complex() {
    // IF(SUM(transactions.amount) > 1000, SUM(transactions.amount) * 0.1, 0)
    let ast =
        parse_expression("IF(SUM(transactions.amount) > 1000, SUM(transactions.amount) * 0.1, 0)")
            .expect("Failed to parse complex expression");

    match ast {
        ExprAST::FunctionCall { name, args } => {
            assert_eq!(name, "IF");
            assert_eq!(args.len(), 3);
        }
        _ => panic!("Expected FunctionCall"),
    }
}

#[test]
fn test_sample_expression_nested_functions() {
    // ROUND(AVG(transactions.amount), 2)
    let ast = parse_expression("ROUND(AVG(transactions.amount), 2)")
        .expect("Failed to parse nested function expression");

    match ast {
        ExprAST::FunctionCall { name, args } => {
            assert_eq!(name, "ROUND");
            assert_eq!(args.len(), 2);
            assert!(matches!(args[0], ExprAST::FunctionCall { .. }));
        }
        _ => panic!("Expected FunctionCall"),
    }
}

#[test]
fn test_sample_expression_with_parentheses() {
    // (transactions.price - transactions.cost) / transactions.price * 100
    let ast =
        parse_expression("(transactions.price - transactions.cost) / transactions.price * 100")
            .expect("Failed to parse expression with parentheses");

    match ast {
        ExprAST::BinaryOp {
            op: BinaryOperator::Multiply,
            ..
        } => {
            // Verified structure
        }
        _ => panic!("Expected BinaryOp with Multiply"),
    }
}
