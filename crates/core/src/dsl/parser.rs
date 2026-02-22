//! Expression parser implementation using pest

use crate::dsl::ast::*;
use crate::dsl::error::ParseError;
use pest::iterators::Pair;
use pest::pratt_parser::{Assoc, Op, PrattParser};
use pest::Parser;
use pest_derive::Parser;

#[derive(Parser)]
#[grammar = "dsl/grammar.pest"]
struct ExprParser;

/// Span information for parsed expressions
#[derive(Debug, Clone, PartialEq)]
pub struct Span {
    pub start: usize,
    pub end: usize,
}

lazy_static::lazy_static! {
    static ref PRATT_PARSER: PrattParser<Rule> = {
        use Assoc::*;
        use Rule::*;

        PrattParser::new()
            // Logical OR (lowest precedence)
            .op(Op::infix(or_op, Left))
            // Logical AND
            .op(Op::infix(and_op, Left))
            // Comparison operators
            .op(Op::infix(eq_op, Left) | Op::infix(ne_op, Left))
            .op(Op::infix(lt_op, Left) | Op::infix(le_op, Left) | Op::infix(gt_op, Left) | Op::infix(ge_op, Left))
            // Additive operators
            .op(Op::infix(add_op, Left) | Op::infix(sub_op, Left))
            // Multiplicative operators (highest precedence for infix)
            .op(Op::infix(mul_op, Left) | Op::infix(div_op, Left))
    };
}

/// Parse an expression string into an AST
pub fn parse_expression(input: &str) -> Result<ExprAST, ParseError> {
    let pairs = ExprParser::parse(Rule::expression, input).map_err(|e| {
        let (line, column) = match e.line_col {
            pest::error::LineColLocation::Pos((line, col)) => (line, col),
            pest::error::LineColLocation::Span((line, col), _) => (line, col),
        };
        ParseError::SyntaxError {
            line,
            column,
            message: format!("{}", e.variant),
        }
    })?;

    let expr_pair = pairs
        .into_iter()
        .next()
        .ok_or_else(|| ParseError::InternalError {
            message: "No expression parsed".to_string(),
        })?
        .into_inner()
        .next()
        .ok_or_else(|| ParseError::InternalError {
            message: "Empty expression".to_string(),
        })?;

    parse_expr(expr_pair)
}

/// Parse an expression string into an AST with span information
pub fn parse_expression_with_span(input: &str) -> Result<(ExprAST, Span), ParseError> {
    let pairs = ExprParser::parse(Rule::expression, input).map_err(|e| {
        let (line, column) = match e.line_col {
            pest::error::LineColLocation::Pos((line, col)) => (line, col),
            pest::error::LineColLocation::Span((line, col), _) => (line, col),
        };
        ParseError::SyntaxError {
            line,
            column,
            message: format!("{}", e.variant),
        }
    })?;

    let expr_pair = pairs
        .into_iter()
        .next()
        .ok_or_else(|| ParseError::InternalError {
            message: "No expression parsed".to_string(),
        })?
        .into_inner()
        .next()
        .ok_or_else(|| ParseError::InternalError {
            message: "Empty expression".to_string(),
        })?;

    let span = Span {
        start: expr_pair.as_span().start(),
        end: expr_pair.as_span().end(),
    };

    let ast = parse_expr(expr_pair)?;
    Ok((ast, span))
}

/// Parse an expression using Pratt parser for precedence
fn parse_expr(pair: Pair<Rule>) -> Result<ExprAST, ParseError> {
    PRATT_PARSER
        .map_primary(|primary| parse_term(primary))
        .map_infix(|lhs, op, rhs| {
            let lhs = lhs?;
            let rhs = rhs?;
            let binary_op = match op.as_rule() {
                Rule::add_op => BinaryOperator::Add,
                Rule::sub_op => BinaryOperator::Subtract,
                Rule::mul_op => BinaryOperator::Multiply,
                Rule::div_op => BinaryOperator::Divide,
                Rule::eq_op => BinaryOperator::Equal,
                Rule::ne_op => BinaryOperator::NotEqual,
                Rule::lt_op => BinaryOperator::LessThan,
                Rule::le_op => BinaryOperator::LessThanOrEqual,
                Rule::gt_op => BinaryOperator::GreaterThan,
                Rule::ge_op => BinaryOperator::GreaterThanOrEqual,
                Rule::and_op => BinaryOperator::And,
                Rule::or_op => BinaryOperator::Or,
                _ => {
                    return Err(ParseError::InternalError {
                        message: format!("Unknown infix operator: {:?}", op.as_rule()),
                    })
                }
            };
            Ok(ExprAST::binary_op(binary_op, lhs, rhs))
        })
        .parse(pair.into_inner())
}

/// Parse a term (handles unary operators)
fn parse_term(pair: Pair<Rule>) -> Result<ExprAST, ParseError> {
    let mut inner = pair.into_inner();
    let mut unary_ops = Vec::new();

    // Collect all unary operators
    loop {
        let next = inner.next().ok_or_else(|| ParseError::InternalError {
            message: "Empty term".to_string(),
        })?;

        match next.as_rule() {
            Rule::unary_op => {
                let op_pair =
                    next.into_inner()
                        .next()
                        .ok_or_else(|| ParseError::InternalError {
                            message: "Empty unary operator".to_string(),
                        })?;
                let unary_op = match op_pair.as_rule() {
                    Rule::not_op => UnaryOperator::Not,
                    Rule::negate_op => UnaryOperator::Negate,
                    _ => {
                        return Err(ParseError::InternalError {
                            message: format!("Unknown unary operator: {:?}", op_pair.as_rule()),
                        })
                    }
                };
                unary_ops.push(unary_op);
            }
            Rule::primary => {
                // Parse the primary expression
                let mut result = parse_primary(next)?;

                // Apply unary operators in reverse order (right-to-left)
                for op in unary_ops.into_iter().rev() {
                    result = ExprAST::unary_op(op, result);
                }

                return Ok(result);
            }
            _ => {
                return Err(ParseError::InternalError {
                    message: format!("Unexpected term component: {:?}", next.as_rule()),
                })
            }
        }
    }
}

/// Parse primary expressions (literals, column refs, function calls, parentheses)
fn parse_primary(pair: Pair<Rule>) -> Result<ExprAST, ParseError> {
    let rule = pair.as_rule();
    match rule {
        Rule::literal => parse_literal(pair),
        Rule::column_ref => parse_column_ref(pair),
        Rule::bare_identifier => {
            // Treat bare identifier as column reference with empty table
            let column = pair.as_str().to_string();
            Ok(ExprAST::column_ref("", column))
        }
        Rule::function_call => parse_function_call(pair),
        Rule::expr => {
            // Recursively parse nested expression (from parentheses)
            parse_expr(pair)
        }
        Rule::primary => {
            // Unwrap the primary rule and parse its content
            let inner = pair
                .into_inner()
                .next()
                .ok_or_else(|| ParseError::InternalError {
                    message: "Empty primary".to_string(),
                })?;
            parse_primary(inner)
        }
        _ => Err(ParseError::InternalError {
            message: format!("Unexpected primary rule: {:?}", pair.as_rule()),
        }),
    }
}

/// Parse literal values
fn parse_literal(pair: Pair<Rule>) -> Result<ExprAST, ParseError> {
    let inner = pair
        .into_inner()
        .next()
        .ok_or_else(|| ParseError::InternalError {
            message: "Empty literal".to_string(),
        })?;

    match inner.as_rule() {
        Rule::number_literal => {
            let value = inner
                .as_str()
                .parse::<f64>()
                .map_err(|_| ParseError::InvalidNumber {
                    value: inner.as_str().to_string(),
                    line: inner.line_col().0,
                    column: inner.line_col().1,
                })?;
            Ok(ExprAST::number(value))
        }
        Rule::string_literal => {
            let s = inner.as_str();
            // Remove quotes
            let value = s[1..s.len() - 1].to_string();
            Ok(ExprAST::string(value))
        }
        Rule::boolean_literal => {
            let value = inner.as_str().eq_ignore_ascii_case("TRUE");
            Ok(ExprAST::boolean(value))
        }
        Rule::null_literal => Ok(ExprAST::null()),
        _ => Err(ParseError::InternalError {
            message: format!("Unknown literal type: {:?}", inner.as_rule()),
        }),
    }
}

/// Parse column reference (table.column)
fn parse_column_ref(pair: Pair<Rule>) -> Result<ExprAST, ParseError> {
    let s = pair.as_str();
    let parts: Vec<&str> = s.split('.').collect();
    if parts.len() != 2 {
        return Err(ParseError::SyntaxError {
            line: pair.line_col().0,
            column: pair.line_col().1,
            message: format!("Invalid column reference: {}", s),
        });
    }
    Ok(ExprAST::column_ref(parts[0], parts[1]))
}

/// Parse function call
fn parse_function_call(pair: Pair<Rule>) -> Result<ExprAST, ParseError> {
    let mut inner = pair.into_inner();
    let name = inner
        .next()
        .ok_or_else(|| ParseError::InternalError {
            message: "Missing function name".to_string(),
        })?
        .as_str()
        .to_uppercase();

    let args = if let Some(arg_list) = inner.next() {
        arg_list
            .into_inner()
            .map(|arg| parse_expr(arg))
            .collect::<Result<Vec<_>, _>>()?
    } else {
        Vec::new()
    };

    Ok(ExprAST::function_call(name, args))
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_parse_number() {
        let ast = parse_expression("42").unwrap();
        assert!(matches!(ast, ExprAST::Literal(LiteralValue::Number(n)) if n == 42.0));
    }

    #[test]
    fn test_parse_string() {
        let ast = parse_expression(r#""hello""#).unwrap();
        assert!(matches!(ast, ExprAST::Literal(LiteralValue::String(s)) if s == "hello"));
    }

    #[test]
    fn test_parse_boolean() {
        let ast = parse_expression("TRUE").unwrap();
        assert!(matches!(ast, ExprAST::Literal(LiteralValue::Boolean(true))));
    }

    #[test]
    fn test_parse_column() {
        let ast = parse_expression("transactions.amount").unwrap();
        match ast {
            ExprAST::ColumnRef { table, column } => {
                assert_eq!(table, "transactions");
                assert_eq!(column, "amount");
            }
            _ => panic!("Expected ColumnRef"),
        }
    }

    #[test]
    fn test_parse_binary_op() {
        let ast = parse_expression("1 + 2").unwrap();
        match ast {
            ExprAST::BinaryOp { op, .. } => {
                assert_eq!(op, BinaryOperator::Add);
            }
            _ => panic!("Expected BinaryOp"),
        }
    }

    #[test]
    fn test_parse_function() {
        let ast = parse_expression("SUM(x)").unwrap();
        match ast {
            ExprAST::FunctionCall { name, args } => {
                assert_eq!(name, "SUM");
                assert_eq!(args.len(), 1);
            }
            _ => panic!("Expected FunctionCall"),
        }
    }

    #[test]
    fn test_precedence() {
        let ast = parse_expression("1 + 2 * 3").unwrap();
        match ast {
            ExprAST::BinaryOp {
                op: BinaryOperator::Add,
                right,
                ..
            } => {
                assert!(matches!(
                    *right,
                    ExprAST::BinaryOp {
                        op: BinaryOperator::Multiply,
                        ..
                    }
                ));
            }
            _ => panic!("Expected Add with Multiply on right"),
        }
    }
}
