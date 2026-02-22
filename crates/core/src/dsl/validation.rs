//! Expression validation module
//!
//! This module provides validation functionality for parsed AST expressions,
//! including column resolution, type checking, and aggregate context validation.

use crate::dsl::ast::*;
use crate::dsl::context::{ColumnType, CompilationContext};
use crate::dsl::error::ValidationError;
use crate::dsl::types::{ExprType, TypedExprAST};
use std::collections::HashSet;

/// Validate an AST expression against a compilation context
pub fn validate_expression(
    ast: &ExprAST,
    context: &CompilationContext,
) -> Result<TypedExprAST, ValidationError> {
    // Infer the type of the expression
    let return_type = infer_type(ast, context)?;
    
    // Type check the expression
    type_check(ast, context)?;
    
    // Check aggregate context
    validate_aggregate_context(ast, context)?;
    
    Ok(TypedExprAST::new(ast.clone(), return_type))
}

/// Infer the type of an expression
pub fn infer_type(ast: &ExprAST, context: &CompilationContext) -> Result<ExprType, ValidationError> {
    match ast {
        ExprAST::Literal(lit) => Ok(infer_literal_type(lit)),
        
        ExprAST::ColumnRef { table, column } => {
            let col_name = if table.is_empty() {
                column.clone()
            } else {
                format!("{}.{}", table, column)
            };
            
            let col_info = context.get_column(&col_name).ok_or_else(|| {
                ValidationError::UnresolvedColumnRef {
                    table: table.clone(),
                    column: column.clone(),
                }
            })?;
            
            Ok(column_type_to_expr_type(col_info.column_type))
        }
        
        ExprAST::BinaryOp { op, left, right } => {
            let left_type = infer_type(left, context)?;
            let right_type = infer_type(right, context)?;
            
            match op {
                BinaryOperator::Add | BinaryOperator::Subtract | 
                BinaryOperator::Multiply | BinaryOperator::Divide => Ok(ExprType::Number),
                
                BinaryOperator::Equal | BinaryOperator::NotEqual |
                BinaryOperator::LessThan | BinaryOperator::LessThanOrEqual |
                BinaryOperator::GreaterThan | BinaryOperator::GreaterThanOrEqual => Ok(ExprType::Boolean),
                
                BinaryOperator::And | BinaryOperator::Or => Ok(ExprType::Boolean),
            }
        }
        
        ExprAST::UnaryOp { op, operand } => {
            match op {
                UnaryOperator::Not => Ok(ExprType::Boolean),
                UnaryOperator::Negate => Ok(ExprType::Number),
            }
        }
        
        ExprAST::FunctionCall { name, args } => {
            infer_function_type(name, args, context)
        }
    }
}

/// Type check an expression
fn type_check(ast: &ExprAST, context: &CompilationContext) -> Result<(), ValidationError> {
    match ast {
        ExprAST::Literal(_) => Ok(()),
        
        ExprAST::ColumnRef { .. } => Ok(()),
        
        ExprAST::BinaryOp { op, left, right } => {
            let left_type = infer_type(left, context)?;
            let right_type = infer_type(right, context)?;
            
            match op {
                BinaryOperator::Add | BinaryOperator::Subtract | 
                BinaryOperator::Multiply | BinaryOperator::Divide => {
                    if !left_type.is_numeric() && left_type != ExprType::Any {
                        return Err(ValidationError::TypeMismatch {
                            expected: "Number".to_string(),
                            actual: left_type.name().to_string(),
                            context: format!("left operand of {}", op),
                        });
                    }
                    if !right_type.is_numeric() && right_type != ExprType::Any {
                        return Err(ValidationError::TypeMismatch {
                            expected: "Number".to_string(),
                            actual: right_type.name().to_string(),
                            context: format!("right operand of {}", op),
                        });
                    }
                }
                
                BinaryOperator::And | BinaryOperator::Or => {
                    if !left_type.is_boolean() && left_type != ExprType::Any {
                        return Err(ValidationError::TypeMismatch {
                            expected: "Boolean".to_string(),
                            actual: left_type.name().to_string(),
                            context: format!("left operand of {}", op),
                        });
                    }
                    if !right_type.is_boolean() && right_type != ExprType::Any {
                        return Err(ValidationError::TypeMismatch {
                            expected: "Boolean".to_string(),
                            actual: right_type.name().to_string(),
                            context: format!("right operand of {}", op),
                        });
                    }
                }
                
                _ => {} // Comparison operators accept any type
            }
            
            // Recursively check operands
            type_check(left, context)?;
            type_check(right, context)?;
            
            Ok(())
        }
        
        ExprAST::UnaryOp { op, operand } => {
            let operand_type = infer_type(operand, context)?;
            
            match op {
                UnaryOperator::Not => {
                    if !operand_type.is_boolean() && operand_type != ExprType::Any {
                        return Err(ValidationError::TypeMismatch {
                            expected: "Boolean".to_string(),
                            actual: operand_type.name().to_string(),
                            context: "operand of NOT".to_string(),
                        });
                    }
                }
                UnaryOperator::Negate => {
                    if !operand_type.is_numeric() && operand_type != ExprType::Any {
                        return Err(ValidationError::TypeMismatch {
                            expected: "Number".to_string(),
                            actual: operand_type.name().to_string(),
                            context: "operand of negation".to_string(),
                        });
                    }
                }
            }
            
            type_check(operand, context)?;
            Ok(())
        }
        
        ExprAST::FunctionCall { name, args } => {
            // Check each argument
            for arg in args {
                type_check(arg, context)?;
            }
            Ok(())
        }
    }
}

/// Validate aggregate function usage
fn validate_aggregate_context(ast: &ExprAST, context: &CompilationContext) -> Result<(), ValidationError> {
    match ast {
        ExprAST::FunctionCall { name, args } => {
            // Check if this is an aggregate function
            if is_aggregate_function(name) && !context.allow_aggregates {
                return Err(ValidationError::InvalidAggregateContext {
                    function: name.clone(),
                });
            }
            
            // Recursively check arguments
            for arg in args {
                validate_aggregate_context(arg, context)?;
            }
        }
        
        ExprAST::BinaryOp { left, right, .. } => {
            validate_aggregate_context(left, context)?;
            validate_aggregate_context(right, context)?;
        }
        
        ExprAST::UnaryOp { operand, .. } => {
            validate_aggregate_context(operand, context)?;
        }
        
        _ => {}
    }
    
    Ok(())
}

/// Check if a function is an aggregate function
fn is_aggregate_function(name: &str) -> bool {
    matches!(
        name.to_uppercase().as_str(),
        "SUM" | "COUNT" | "COUNT_ALL" | "AVG" | "MIN_AGG" | "MAX_AGG"
    )
}

/// Infer the type of a literal
fn infer_literal_type(lit: &LiteralValue) -> ExprType {
    match lit {
        LiteralValue::Number(_) => ExprType::Number,
        LiteralValue::String(_) => ExprType::String,
        LiteralValue::Boolean(_) => ExprType::Boolean,
        LiteralValue::Date(_) => ExprType::Date,
        LiteralValue::Null => ExprType::Null,
    }
}

/// Convert column type to expression type
fn column_type_to_expr_type(col_type: ColumnType) -> ExprType {
    match col_type {
        ColumnType::Integer | ColumnType::Float => ExprType::Number,
        ColumnType::String => ExprType::String,
        ColumnType::Boolean => ExprType::Boolean,
        ColumnType::Date => ExprType::Date,
    }
}

/// Infer function return type
fn infer_function_type(
    name: &str,
    args: &[ExprAST],
    context: &CompilationContext,
) -> Result<ExprType, ValidationError> {
    match name.to_uppercase().as_str() {
        // Arithmetic functions
        "ABS" | "ROUND" | "FLOOR" | "CEIL" | "MOD" | "MIN" | "MAX" => Ok(ExprType::Number),
        
        // Aggregate functions
        "SUM" | "COUNT" | "COUNT_ALL" | "AVG" | "MIN_AGG" | "MAX_AGG" => Ok(ExprType::Number),
        
        // String functions
        "CONCAT" | "UPPER" | "LOWER" | "TRIM" | "LEFT" | "RIGHT" => Ok(ExprType::String),
        "LEN" => Ok(ExprType::Number),
        "CONTAINS" => Ok(ExprType::Boolean),
        "REPLACE" => Ok(ExprType::String),
        
        // Conditional functions
        "IF" => {
            if args.len() >= 2 {
                infer_type(&args[1], context)
            } else {
                Ok(ExprType::Any)
            }
        }
        "ISNULL" => Ok(ExprType::Boolean),
        "COALESCE" => {
            if let Some(first) = args.first() {
                infer_type(first, context)
            } else {
                Ok(ExprType::Any)
            }
        }
        
        // Date functions
        "DATE" | "DATEADD" => Ok(ExprType::Date),
        "TODAY" => Ok(ExprType::Date),
        "YEAR" | "MONTH" | "DAY" | "DATEDIFF" => Ok(ExprType::Number),
        
        _ => Err(ValidationError::InvalidFunction {
            function: name.to_string(),
            reason: "Unknown function".to_string(),
        }),
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_validate_literal() {
        let ast = ExprAST::number(42.0);
        let ctx = CompilationContext::new();
        let result = validate_expression(&ast, &ctx);
        assert!(result.is_ok());
        assert_eq!(result.unwrap().return_type, ExprType::Number);
    }

    #[test]
    fn test_validate_column_ref() {
        let ast = ExprAST::column_ref("transactions", "amount");
        let mut ctx = CompilationContext::new();
        ctx.add_column("transactions.amount", ColumnType::Float);
        
        let result = validate_expression(&ast, &ctx);
        assert!(result.is_ok());
        assert_eq!(result.unwrap().return_type, ExprType::Number);
    }

    #[test]
    fn test_unresolved_column() {
        let ast = ExprAST::column_ref("transactions", "unknown");
        let ctx = CompilationContext::new();
        
        let result = validate_expression(&ast, &ctx);
        assert!(result.is_err());
        assert!(matches!(result.unwrap_err(), ValidationError::UnresolvedColumnRef { .. }));
    }

    #[test]
    fn test_type_mismatch() {
        let ast = ExprAST::binary_op(
            BinaryOperator::Add,
            ExprAST::number(1.0),
            ExprAST::string("text"),
        );
        let ctx = CompilationContext::new();
        
        let result = validate_expression(&ast, &ctx);
        assert!(result.is_err());
        assert!(matches!(result.unwrap_err(), ValidationError::TypeMismatch { .. }));
    }

    #[test]
    fn test_aggregate_without_context() {
        let ast = ExprAST::function_call("SUM", vec![ExprAST::number(1.0)]);
        let ctx = CompilationContext::new();
        
        let result = validate_expression(&ast, &ctx);
        assert!(result.is_err());
        assert!(matches!(result.unwrap_err(), ValidationError::InvalidAggregateContext { .. }));
    }

    #[test]
    fn test_aggregate_with_context() {
        let ast = ExprAST::function_call("SUM", vec![ExprAST::number(1.0)]);
        let ctx = CompilationContext::new().with_aggregates(true);
        
        let result = validate_expression(&ast, &ctx);
        assert!(result.is_ok());
    }
}
