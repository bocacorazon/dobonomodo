// Rule condition matcher - evaluates when_expression against context
// Implements expression parsing and evaluation for boolean conditions

use crate::model::ResolutionRule;
use crate::resolver::context::ResolutionContext;
use std::collections::HashMap;

#[derive(Debug, Clone, PartialEq, Eq)]
pub enum Token {
    Identifier(String),
    StringLiteral(String),
    BooleanLiteral(bool),
    Equal,
    NotEqual,
    LessThan,
    LessThanOrEqual,
    GreaterThan,
    GreaterThanOrEqual,
    And,
    Or,
    Not,
    LeftParen,
    RightParen,
}

/// Tokenize expression string into tokens
pub fn tokenize(expr: &str) -> Result<Vec<Token>, String> {
    let mut tokens = Vec::new();
    let mut chars = expr.chars().peekable();

    while let Some(&ch) = chars.peek() {
        match ch {
            ' ' | '\t' | '\n' | '\r' => {
                chars.next();
            }
            '(' => {
                tokens.push(Token::LeftParen);
                chars.next();
            }
            ')' => {
                tokens.push(Token::RightParen);
                chars.next();
            }
            '=' => {
                chars.next();
                if chars.peek() == Some(&'=') {
                    chars.next();
                    tokens.push(Token::Equal);
                } else {
                    return Err("expected '==' not '='".to_string());
                }
            }
            '!' => {
                chars.next();
                if chars.peek() == Some(&'=') {
                    chars.next();
                    tokens.push(Token::NotEqual);
                } else {
                    return Err("expected '!=' not '!'".to_string());
                }
            }
            '<' => {
                chars.next();
                if chars.peek() == Some(&'=') {
                    chars.next();
                    tokens.push(Token::LessThanOrEqual);
                } else {
                    tokens.push(Token::LessThan);
                }
            }
            '>' => {
                chars.next();
                if chars.peek() == Some(&'=') {
                    chars.next();
                    tokens.push(Token::GreaterThanOrEqual);
                } else {
                    tokens.push(Token::GreaterThan);
                }
            }
            '\'' | '"' => {
                let quote = ch;
                chars.next();
                let mut literal = String::new();
                loop {
                    match chars.next() {
                        Some(c) if c == quote => break,
                        Some(c) => literal.push(c),
                        None => return Err("unterminated string literal".to_string()),
                    }
                }
                tokens.push(Token::StringLiteral(literal));
            }
            c if c.is_alphabetic() || c == '_' => {
                let mut ident = String::new();
                while let Some(&c) = chars.peek() {
                    if c.is_alphanumeric() || c == '_' {
                        ident.push(c);
                        chars.next();
                    } else {
                        break;
                    }
                }
                // Check for keywords
                match ident.as_str() {
                    "AND" => tokens.push(Token::And),
                    "OR" => tokens.push(Token::Or),
                    "NOT" => tokens.push(Token::Not),
                    "true" => tokens.push(Token::BooleanLiteral(true)),
                    "false" => tokens.push(Token::BooleanLiteral(false)),
                    _ => tokens.push(Token::Identifier(ident)),
                }
            }
            _ => {
                return Err(format!("unexpected character: '{}'", ch));
            }
        }
    }

    Ok(tokens)
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub enum Expr {
    Comparison {
        left: ComparisonOperand,
        op: CompOp,
        right: ComparisonOperand,
    },
    Logical {
        left: Box<Expr>,
        op: LogicalOp,
        right: Box<Expr>,
    },
    Not(Box<Expr>),
    Literal(bool),
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub enum ComparisonOperand {
    Identifier(String),
    StringLiteral(String),
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub enum CompOp {
    Equal,
    NotEqual,
    LessThan,
    LessThanOrEqual,
    GreaterThan,
    GreaterThanOrEqual,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub enum LogicalOp {
    And,
    Or,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct RuleEvaluationDetail {
    pub matched: bool,
    pub evaluated_expression: Option<String>,
}

/// Parse expression tokens into AST
pub fn parse_expression(tokens: &[Token]) -> Result<Expr, String> {
    let mut pos = 0;
    let expr = parse_or(tokens, &mut pos)?;
    if pos != tokens.len() {
        return Err(format!(
            "unexpected token {:?} at position {}",
            tokens[pos], pos
        ));
    }
    Ok(expr)
}

fn parse_or(tokens: &[Token], pos: &mut usize) -> Result<Expr, String> {
    let mut left = parse_and(tokens, pos)?;

    while *pos < tokens.len() {
        if let Token::Or = &tokens[*pos] {
            *pos += 1;
            let right = parse_and(tokens, pos)?;
            left = Expr::Logical {
                left: Box::new(left),
                op: LogicalOp::Or,
                right: Box::new(right),
            };
        } else {
            break;
        }
    }

    Ok(left)
}

fn parse_and(tokens: &[Token], pos: &mut usize) -> Result<Expr, String> {
    let mut left = parse_not(tokens, pos)?;

    while *pos < tokens.len() {
        if let Token::And = &tokens[*pos] {
            *pos += 1;
            let right = parse_not(tokens, pos)?;
            left = Expr::Logical {
                left: Box::new(left),
                op: LogicalOp::And,
                right: Box::new(right),
            };
        } else {
            break;
        }
    }

    Ok(left)
}

fn parse_not(tokens: &[Token], pos: &mut usize) -> Result<Expr, String> {
    if *pos < tokens.len() {
        if let Token::Not = &tokens[*pos] {
            *pos += 1;
            let expr = parse_not(tokens, pos)?;
            return Ok(Expr::Not(Box::new(expr)));
        }
    }
    parse_primary(tokens, pos)
}

fn parse_primary(tokens: &[Token], pos: &mut usize) -> Result<Expr, String> {
    if *pos >= tokens.len() {
        return Err("unexpected end of expression".to_string());
    }

    // Handle parenthesized expressions
    if let Token::LeftParen = &tokens[*pos] {
        *pos += 1;
        let expr = parse_or(tokens, pos)?;
        if *pos >= tokens.len() || !matches!(tokens[*pos], Token::RightParen) {
            return Err("expected ')'".to_string());
        }
        *pos += 1;
        return Ok(expr);
    }

    if let Token::BooleanLiteral(value) = &tokens[*pos] {
        *pos += 1;
        return Ok(Expr::Literal(*value));
    }

    // Parse comparison
    let left = parse_comparison_operand(tokens, pos)?;

    if *pos >= tokens.len() {
        return Err("incomplete comparison expression".to_string());
    }

    let op = match &tokens[*pos] {
        Token::Equal => CompOp::Equal,
        Token::NotEqual => CompOp::NotEqual,
        Token::LessThan => CompOp::LessThan,
        Token::LessThanOrEqual => CompOp::LessThanOrEqual,
        Token::GreaterThan => CompOp::GreaterThan,
        Token::GreaterThanOrEqual => CompOp::GreaterThanOrEqual,
        _ => {
            return Err(format!(
                "expected comparison operator, got {:?}",
                tokens[*pos]
            ))
        }
    };
    *pos += 1;

    if *pos >= tokens.len() {
        return Err("incomplete comparison expression".to_string());
    }

    let right = parse_comparison_operand(tokens, pos)?;

    Ok(Expr::Comparison { left, op, right })
}

fn parse_comparison_operand(
    tokens: &[Token],
    pos: &mut usize,
) -> Result<ComparisonOperand, String> {
    let operand = match &tokens[*pos] {
        Token::Identifier(id) => ComparisonOperand::Identifier(id.clone()),
        Token::StringLiteral(s) => ComparisonOperand::StringLiteral(s.clone()),
        _ => {
            return Err(format!(
                "expected identifier or literal, got {:?}",
                tokens[*pos]
            ))
        }
    };
    *pos += 1;
    Ok(operand)
}

/// Evaluate expression against context
pub fn evaluate_expression(expr: &Expr, context: &HashMap<String, String>) -> Result<bool, String> {
    match expr {
        Expr::Comparison { left, op, right } => {
            let left_val = resolve_operand(left, context)?;
            let right_val = resolve_operand(right, context)?;

            Ok(match op {
                CompOp::Equal => left_val == right_val,
                CompOp::NotEqual => left_val != right_val,
                CompOp::LessThan => left_val < right_val,
                CompOp::LessThanOrEqual => left_val <= right_val,
                CompOp::GreaterThan => left_val > right_val,
                CompOp::GreaterThanOrEqual => left_val >= right_val,
            })
        }
        Expr::Logical { left, op, right } => {
            let left_result = evaluate_expression(left, context)?;
            match op {
                LogicalOp::And => {
                    if !left_result {
                        Ok(false)
                    } else {
                        evaluate_expression(right, context)
                    }
                }
                LogicalOp::Or => {
                    if left_result {
                        Ok(true)
                    } else {
                        evaluate_expression(right, context)
                    }
                }
            }
        }
        Expr::Not(inner) => {
            let result = evaluate_expression(inner, context)?;
            Ok(!result)
        }
        Expr::Literal(b) => Ok(*b),
    }
}

fn resolve_operand<'a>(
    operand: &'a ComparisonOperand,
    context: &'a HashMap<String, String>,
) -> Result<&'a str, String> {
    match operand {
        ComparisonOperand::StringLiteral(value) => Ok(value.as_str()),
        ComparisonOperand::Identifier(identifier) => context
            .get(identifier)
            .map(String::as_str)
            .ok_or_else(|| format!("unknown identifier '{}'", identifier)),
    }
}

/// Evaluate a single rule against context (T023)
pub fn evaluate_rule(rule: &ResolutionRule, context: &ResolutionContext) -> Result<bool, String> {
    Ok(evaluate_rule_with_detail(rule, context)?.matched)
}

/// Evaluate a rule and retain expression metadata for diagnostics.
pub fn evaluate_rule_with_detail(
    rule: &ResolutionRule,
    context: &ResolutionContext,
) -> Result<RuleEvaluationDetail, String> {
    // If no when_expression, rule always matches
    let when_expr = match &rule.when_expression {
        Some(expr) => expr,
        None => {
            return Ok(RuleEvaluationDetail {
                matched: true,
                evaluated_expression: None,
            });
        }
    };

    // Build evaluation context
    let mut eval_context = HashMap::new();
    eval_context.insert("period".to_string(), context.period.identifier.clone());
    eval_context.insert("table".to_string(), context.table_name.clone());
    eval_context.insert("dataset".to_string(), context.dataset_id.clone());

    // Parse and evaluate expression
    let tokens = tokenize(when_expr)
        .map_err(|error| format!("invalid expression '{}': {}", when_expr, error))?;
    let expr = parse_expression(&tokens)
        .map_err(|error| format!("invalid expression '{}': {}", when_expr, error))?;
    let matched = evaluate_expression(&expr, &eval_context)
        .map_err(|error| format!("invalid expression '{}': {}", when_expr, error))?;

    Ok(RuleEvaluationDetail {
        matched,
        evaluated_expression: Some(when_expr.clone()),
    })
}

/// Validate expression syntax (T029)
pub fn validate_expression(expr_str: &str) -> Result<(), String> {
    let tokens = tokenize(expr_str)
        .map_err(|error| format!("invalid expression '{}': {}", expr_str, error))?;
    parse_expression(&tokens)
        .map_err(|error| format!("invalid expression '{}': {}", expr_str, error))?;
    Ok(())
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_tokenize_simple_comparison() {
        let tokens = tokenize("period >= '2024-Q1'").unwrap();
        assert_eq!(tokens.len(), 3);
        assert!(matches!(tokens[0], Token::Identifier(_)));
        assert!(matches!(tokens[1], Token::GreaterThanOrEqual));
        assert!(matches!(tokens[2], Token::StringLiteral(_)));
    }

    #[test]
    fn test_parse_comparison() {
        let tokens = tokenize("period >= '2024-Q1'").unwrap();
        let expr = parse_expression(&tokens).unwrap();
        assert!(matches!(expr, Expr::Comparison { .. }));
    }

    #[test]
    fn test_evaluate_comparison_true() {
        let tokens = tokenize("period >= '2024-Q1'").unwrap();
        let expr = parse_expression(&tokens).unwrap();
        let mut context = HashMap::new();
        context.insert("period".to_string(), "2024-Q2".to_string());
        assert!(evaluate_expression(&expr, &context).unwrap());
    }

    #[test]
    fn test_evaluate_comparison_false() {
        let tokens = tokenize("period >= '2024-Q1'").unwrap();
        let expr = parse_expression(&tokens).unwrap();
        let mut context = HashMap::new();
        context.insert("period".to_string(), "2023-Q4".to_string());
        assert!(!evaluate_expression(&expr, &context).unwrap());
    }

    #[test]
    fn test_evaluate_and_expression() {
        let tokens = tokenize("table == 'sales' AND period >= '2024-Q1'").unwrap();
        let expr = parse_expression(&tokens).unwrap();
        let mut context = HashMap::new();
        context.insert("table".to_string(), "sales".to_string());
        context.insert("period".to_string(), "2024-Q2".to_string());
        assert!(evaluate_expression(&expr, &context).unwrap());
    }

    #[test]
    fn test_parse_rejects_trailing_tokens() {
        let tokens = tokenize("table == 'sales' dataset == 'x'").unwrap();
        let err = parse_expression(&tokens).unwrap_err();
        assert!(err.contains("unexpected token"));
    }

    #[test]
    fn test_unknown_identifier_is_error() {
        let tokens = tokenize("unknown == 'sales'").unwrap();
        let expr = parse_expression(&tokens).unwrap();
        let context = HashMap::new();
        let err = evaluate_expression(&expr, &context).unwrap_err();
        assert!(err.contains("unknown identifier"));
    }

    #[test]
    fn test_rule_evaluation_detail_preserves_expression() {
        let rule = ResolutionRule {
            name: "rule".to_string(),
            when_expression: Some("table == 'sales'".to_string()),
            data_level: "any".to_string(),
            strategy: crate::model::ResolutionStrategy::Path {
                datasource_id: "ds".to_string(),
                path: "/data/{period_id}.parquet".to_string(),
            },
        };
        let context = crate::resolver::context::ResolutionContext {
            dataset_id: "dataset".to_string(),
            table_name: "sales".to_string(),
            period: crate::model::Period {
                id: uuid::Uuid::parse_str("550e8400-e29b-41d4-a716-446655440000").unwrap(),
                identifier: "2024-Q1".to_string(),
                name: "Q1".to_string(),
                description: None,
                calendar_id: uuid::Uuid::parse_str("550e8400-e29b-41d4-a716-446655440001").unwrap(),
                year: 2024,
                sequence: 1,
                start_date: "2024-01-01".to_string(),
                end_date: "2024-03-31".to_string(),
                status: crate::model::PeriodStatus::Open,
                parent_id: None,
                created_at: None,
                updated_at: None,
            },
            period_level: "quarter".to_string(),
            resolver_source: None,
            additional_context: HashMap::new(),
        };

        let detail = evaluate_rule_with_detail(&rule, &context).unwrap();
        assert!(detail.matched);
        assert_eq!(
            detail.evaluated_expression,
            Some("table == 'sales'".to_string())
        );
    }

    #[test]
    fn test_parse_boolean_literal_expression() {
        let tokens = tokenize("true").unwrap();
        let expr = parse_expression(&tokens).unwrap();
        assert_eq!(expr, Expr::Literal(true));
    }

    #[test]
    fn test_evaluate_boolean_literal_and_logical_expression() {
        let tokens = tokenize("true AND table == 'sales'").unwrap();
        let expr = parse_expression(&tokens).unwrap();
        let mut context = HashMap::new();
        context.insert("table".to_string(), "sales".to_string());
        assert!(evaluate_expression(&expr, &context).unwrap());
    }
}
