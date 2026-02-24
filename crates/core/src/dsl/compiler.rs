use std::collections::{BTreeMap, BTreeSet};

use thiserror::Error;

#[derive(Debug, Clone, PartialEq, Eq, Default)]
pub struct ExpressionSymbolTable {
    working_columns: BTreeSet<String>,
    join_alias_columns: BTreeMap<String, BTreeSet<String>>,
}

impl ExpressionSymbolTable {
    pub fn with_working_columns(columns: impl IntoIterator<Item = String>) -> Self {
        Self {
            working_columns: columns.into_iter().collect(),
            join_alias_columns: BTreeMap::new(),
        }
    }

    pub fn add_join_alias(
        &mut self,
        alias: impl Into<String>,
        columns: impl IntoIterator<Item = String>,
    ) {
        self.join_alias_columns
            .insert(alias.into(), columns.into_iter().collect());
    }

    pub fn add_working_column(&mut self, column: impl Into<String>) {
        self.working_columns.insert(column.into());
    }

    pub fn map_reference(&self, reference: &str) -> Result<String, CompileError> {
        if let Some((alias, column)) = reference.split_once('.') {
            let Some(columns) = self.join_alias_columns.get(alias) else {
                return Err(CompileError::UnknownAlias(alias.to_string()));
            };

            let is_quoted = column.starts_with('"') && column.ends_with('"');
            let column_name = if is_quoted {
                &column[1..column.len() - 1]
            } else {
                column
            };

            if !columns.contains(column_name) {
                return Err(CompileError::UnknownAliasedColumn {
                    alias: alias.to_string(),
                    column: column_name.to_string(),
                });
            }

            if is_quoted {
                return Ok(format!("\"{column_name}_{alias}\""));
            }
            return Ok(format!("{column_name}_{alias}"));
        }

        if self.working_columns.contains(reference) {
            return Ok(reference.to_string());
        }

        Err(CompileError::UnknownColumn(reference.to_string()))
    }
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub enum JoinLogicalOp {
    And,
    Or,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub enum JoinComparisonOp {
    Eq,
    NotEq,
    Lt,
    Lte,
    Gt,
    Gte,
}

#[derive(Debug, Clone, PartialEq)]
pub enum JoinConditionValue {
    Reference(String),
    StringLiteral(String),
    NumberLiteral(String),
    BooleanLiteral(bool),
}

#[derive(Debug, Clone, PartialEq)]
pub enum JoinConditionExpr {
    Comparison {
        left: JoinConditionValue,
        op: JoinComparisonOp,
        right: JoinConditionValue,
    },
    Logical {
        left: Box<JoinConditionExpr>,
        op: JoinLogicalOp,
        right: Box<JoinConditionExpr>,
    },
}

#[derive(Debug, Error, PartialEq, Eq)]
pub enum CompileError {
    #[error("Unknown alias '{0}'")]
    UnknownAlias(String),

    #[error("Unknown column '{alias}.{column}'")]
    UnknownAliasedColumn { alias: String, column: String },

    #[error("Unknown column '{0}'")]
    UnknownColumn(String),

    #[error("Invalid expression: {0}")]
    InvalidExpression(String),
}

#[derive(Debug, Clone)]
struct ReferenceSpan {
    start: usize,
    end: usize,
}

fn is_identifier_start(ch: char) -> bool {
    ch.is_ascii_alphabetic() || ch == '_'
}

fn is_identifier_part(ch: char) -> bool {
    ch.is_ascii_alphanumeric() || ch == '_'
}

fn is_reserved_word(identifier: &str) -> bool {
    matches!(
        identifier.to_ascii_lowercase().as_str(),
        "and" | "or" | "true" | "false" | "null"
    )
}

fn byte_offset(chars: &[(usize, char)], index: usize, expression_len: usize) -> usize {
    chars
        .get(index)
        .map(|(offset, _)| *offset)
        .unwrap_or(expression_len)
}

fn next_non_whitespace_char(expression: &str, from: usize) -> Option<char> {
    expression[from..].chars().find(|ch| !ch.is_whitespace())
}

fn extract_reference_spans(expression: &str) -> Vec<ReferenceSpan> {
    let chars = expression.chars().collect::<Vec<_>>();
    let indexed_chars = expression.char_indices().collect::<Vec<_>>();
    let mut references = Vec::new();
    let mut index = 0usize;
    let expression_len = expression.len();

    while index < chars.len() {
        let current = chars[index];

        if current == '\'' || current == '"' {
            let quote = current;
            index += 1;
            while index < chars.len() {
                if chars[index] == '\\' {
                    index += 2;
                    continue;
                }
                if chars[index] == quote {
                    index += 1;
                    break;
                }
                index += 1;
            }
            continue;
        }

        if is_identifier_start(current) {
            let identifier_start_index = index;
            let start = byte_offset(&indexed_chars, identifier_start_index, expression_len);

            index += 1;
            while index < chars.len() && is_identifier_part(chars[index]) {
                index += 1;
            }
            let identifier_end = byte_offset(&indexed_chars, index, expression_len);
            let identifier = &expression[start..identifier_end];

            let mut reference_end = identifier_end;
            let mut has_alias_separator = false;

            if index < chars.len() && chars[index] == '.' {
                let dot_index = index;
                index += 1;
                if index < chars.len() {
                    let is_quoted = chars[index] == '"';
                    if is_identifier_start(chars[index]) || is_quoted {
                        has_alias_separator = true;
                        if is_quoted {
                            let quote = chars[index];
                            index += 1;
                            while index < chars.len() {
                                if chars[index] == '\\' {
                                    index += 2;
                                    continue;
                                }
                                if chars[index] == quote {
                                    index += 1;
                                    break;
                                }
                                index += 1;
                            }
                        } else {
                            index += 1;
                            while index < chars.len() && is_identifier_part(chars[index]) {
                                index += 1;
                            }
                        }
                        reference_end = byte_offset(&indexed_chars, index, expression_len);
                    } else {
                        index = dot_index;
                    }
                } else {
                    index = dot_index;
                }
            }

            let is_function_name = !has_alias_separator
                && next_non_whitespace_char(expression, reference_end) == Some('(');
            if is_reserved_word(identifier) || is_function_name {
                continue;
            }

            references.push(ReferenceSpan {
                start,
                end: reference_end,
            });

            continue;
        }

        index += 1;
    }

    references
}

#[derive(Debug, Clone, PartialEq)]
enum JoinTokenKind {
    Identifier(String),
    StringLiteral(String),
    NumberLiteral(String),
    LParen,
    RParen,
    Dot,
    Eq,
    NotEq,
    Lt,
    Lte,
    Gt,
    Gte,
    And,
    Or,
}

#[derive(Debug, Clone, PartialEq)]
struct JoinToken {
    kind: JoinTokenKind,
}

fn tokenize_join_condition(expression: &str) -> Result<Vec<JoinToken>, CompileError> {
    let chars = expression.chars().collect::<Vec<_>>();
    let indexed_chars = expression.char_indices().collect::<Vec<_>>();
    let mut tokens = Vec::new();
    let mut index = 0usize;
    let expression_len = expression.len();

    while index < chars.len() {
        let ch = chars[index];

        if ch.is_whitespace() {
            index += 1;
            continue;
        }

        if ch == '\'' || ch == '"' {
            let quote = ch;
            index += 1;
            let mut literal = String::new();
            let mut terminated = false;
            while index < chars.len() {
                if chars[index] == '\\' {
                    if let Some(escaped) = chars.get(index + 1) {
                        literal.push(*escaped);
                        index += 2;
                        continue;
                    }
                    return Err(CompileError::InvalidExpression(
                        "unterminated escape sequence".to_string(),
                    ));
                }
                if chars[index] == quote {
                    index += 1;
                    terminated = true;
                    break;
                }
                literal.push(chars[index]);
                index += 1;
            }
            if !terminated {
                return Err(CompileError::InvalidExpression(
                    "unterminated string literal".to_string(),
                ));
            }

            tokens.push(JoinToken {
                kind: JoinTokenKind::StringLiteral(literal),
            });
            continue;
        }

        if is_identifier_start(ch) {
            let start = index;
            index += 1;
            while index < chars.len() && is_identifier_part(chars[index]) {
                index += 1;
            }

            let start_offset = byte_offset(&indexed_chars, start, expression_len);
            let end_offset = byte_offset(&indexed_chars, index, expression_len);
            let identifier = expression[start_offset..end_offset].to_string();
            let lower = identifier.to_ascii_lowercase();

            let kind = match lower.as_str() {
                "and" => JoinTokenKind::And,
                "or" => JoinTokenKind::Or,
                _ => JoinTokenKind::Identifier(identifier),
            };

            tokens.push(JoinToken { kind });
            continue;
        }

        if ch.is_ascii_digit()
            || (ch == '-'
                && chars
                    .get(index + 1)
                    .is_some_and(|next| next.is_ascii_digit()))
        {
            let start = index;
            index += 1;
            while index < chars.len() && chars[index].is_ascii_digit() {
                index += 1;
            }
            if index < chars.len() && chars[index] == '.' {
                index += 1;
                while index < chars.len() && chars[index].is_ascii_digit() {
                    index += 1;
                }
            }

            let start_offset = byte_offset(&indexed_chars, start, expression_len);
            let end_offset = byte_offset(&indexed_chars, index, expression_len);
            tokens.push(JoinToken {
                kind: JoinTokenKind::NumberLiteral(
                    expression[start_offset..end_offset].to_string(),
                ),
            });
            continue;
        }

        let kind = match ch {
            '(' => Some(JoinTokenKind::LParen),
            ')' => Some(JoinTokenKind::RParen),
            '.' => Some(JoinTokenKind::Dot),
            '=' => Some(JoinTokenKind::Eq),
            '!' => {
                if chars.get(index + 1) == Some(&'=') {
                    index += 1;
                    Some(JoinTokenKind::NotEq)
                } else {
                    return Err(CompileError::InvalidExpression(
                        "unexpected '!' token".to_string(),
                    ));
                }
            }
            '<' => {
                if chars.get(index + 1) == Some(&'=') {
                    index += 1;
                    Some(JoinTokenKind::Lte)
                } else {
                    Some(JoinTokenKind::Lt)
                }
            }
            '>' => {
                if chars.get(index + 1) == Some(&'=') {
                    index += 1;
                    Some(JoinTokenKind::Gte)
                } else {
                    Some(JoinTokenKind::Gt)
                }
            }
            _ => None,
        };

        if let Some(kind) = kind {
            tokens.push(JoinToken { kind });
            index += 1;
            continue;
        }

        return Err(CompileError::InvalidExpression(format!(
            "unexpected character '{ch}'"
        )));
    }

    Ok(tokens)
}

struct JoinConditionParser {
    tokens: Vec<JoinToken>,
    cursor: usize,
}

impl JoinConditionParser {
    fn new(tokens: Vec<JoinToken>) -> Self {
        Self { tokens, cursor: 0 }
    }

    fn parse(mut self) -> Result<JoinConditionExpr, CompileError> {
        let expression = self.parse_or()?;
        if self.cursor < self.tokens.len() {
            return Err(CompileError::InvalidExpression(
                "unexpected trailing tokens".to_string(),
            ));
        }
        Ok(expression)
    }

    fn parse_or(&mut self) -> Result<JoinConditionExpr, CompileError> {
        let mut expression = self.parse_and()?;
        while self.consume_if(|kind| matches!(kind, JoinTokenKind::Or)) {
            let right = self.parse_and()?;
            expression = JoinConditionExpr::Logical {
                left: Box::new(expression),
                op: JoinLogicalOp::Or,
                right: Box::new(right),
            };
        }
        Ok(expression)
    }

    fn parse_and(&mut self) -> Result<JoinConditionExpr, CompileError> {
        let mut expression = self.parse_primary()?;
        while self.consume_if(|kind| matches!(kind, JoinTokenKind::And)) {
            let right = self.parse_primary()?;
            expression = JoinConditionExpr::Logical {
                left: Box::new(expression),
                op: JoinLogicalOp::And,
                right: Box::new(right),
            };
        }
        Ok(expression)
    }

    fn parse_primary(&mut self) -> Result<JoinConditionExpr, CompileError> {
        if self.consume_if(|kind| matches!(kind, JoinTokenKind::LParen)) {
            let expression = self.parse_or()?;
            if !self.consume_if(|kind| matches!(kind, JoinTokenKind::RParen)) {
                return Err(CompileError::InvalidExpression(
                    "missing closing ')'".to_string(),
                ));
            }
            return Ok(expression);
        }

        self.parse_comparison()
    }

    fn parse_comparison(&mut self) -> Result<JoinConditionExpr, CompileError> {
        let left = self.parse_value()?;
        let op = self.parse_comparison_operator()?;
        let right = self.parse_value()?;

        Ok(JoinConditionExpr::Comparison { left, op, right })
    }

    fn parse_value(&mut self) -> Result<JoinConditionValue, CompileError> {
        let token = self.take_current().ok_or_else(|| {
            CompileError::InvalidExpression("unexpected end of expression".to_string())
        })?;

        match token.kind {
            JoinTokenKind::Identifier(identifier) => {
                if self.peek_is(|kind| matches!(kind, JoinTokenKind::LParen)) {
                    return Err(CompileError::InvalidExpression(format!(
                        "function '{identifier}' is not supported in runtime join conditions"
                    )));
                }

                let lower = identifier.to_ascii_lowercase();
                if lower == "true" {
                    return Ok(JoinConditionValue::BooleanLiteral(true));
                }
                if lower == "false" {
                    return Ok(JoinConditionValue::BooleanLiteral(false));
                }
                if lower == "null" {
                    return Err(CompileError::InvalidExpression(
                        "null literals are not supported in runtime join conditions".to_string(),
                    ));
                }

                if self.consume_if(|kind| matches!(kind, JoinTokenKind::Dot)) {
                    let right = self.take_identifier()?;
                    return Ok(JoinConditionValue::Reference(format!(
                        "{identifier}.{right}"
                    )));
                }

                Ok(JoinConditionValue::Reference(identifier))
            }
            JoinTokenKind::StringLiteral(value) => Ok(JoinConditionValue::StringLiteral(value)),
            JoinTokenKind::NumberLiteral(value) => Ok(JoinConditionValue::NumberLiteral(value)),
            JoinTokenKind::LParen => Err(CompileError::InvalidExpression(
                "unexpected '(' while parsing value".to_string(),
            )),
            _ => Err(CompileError::InvalidExpression(
                "invalid value in join condition".to_string(),
            )),
        }
    }

    fn parse_comparison_operator(&mut self) -> Result<JoinComparisonOp, CompileError> {
        let token = self.take_current().ok_or_else(|| {
            CompileError::InvalidExpression(
                "expected comparison operator, found end of expression".to_string(),
            )
        })?;

        match token.kind {
            JoinTokenKind::Eq => Ok(JoinComparisonOp::Eq),
            JoinTokenKind::NotEq => Ok(JoinComparisonOp::NotEq),
            JoinTokenKind::Lt => Ok(JoinComparisonOp::Lt),
            JoinTokenKind::Lte => Ok(JoinComparisonOp::Lte),
            JoinTokenKind::Gt => Ok(JoinComparisonOp::Gt),
            JoinTokenKind::Gte => Ok(JoinComparisonOp::Gte),
            _ => Err(CompileError::InvalidExpression(
                "expected comparison operator".to_string(),
            )),
        }
    }

    fn take_identifier(&mut self) -> Result<String, CompileError> {
        let token = self.take_current().ok_or_else(|| {
            CompileError::InvalidExpression(
                "expected identifier, found end of expression".to_string(),
            )
        })?;

        match token.kind {
            JoinTokenKind::Identifier(value) => Ok(value),
            _ => Err(CompileError::InvalidExpression(
                "expected identifier".to_string(),
            )),
        }
    }

    fn consume_if(&mut self, predicate: impl FnOnce(&JoinTokenKind) -> bool) -> bool {
        let Some(token) = self.tokens.get(self.cursor) else {
            return false;
        };
        if predicate(&token.kind) {
            self.cursor += 1;
            return true;
        }
        false
    }

    fn peek_is(&self, predicate: impl FnOnce(&JoinTokenKind) -> bool) -> bool {
        self.tokens
            .get(self.cursor)
            .is_some_and(|token| predicate(&token.kind))
    }

    fn take_current(&mut self) -> Option<JoinToken> {
        let token = self.tokens.get(self.cursor).cloned()?;
        self.cursor += 1;
        Some(token)
    }
}

pub fn compile_join_condition_expression(
    expression: &str,
) -> Result<JoinConditionExpr, CompileError> {
    let tokens = tokenize_join_condition(expression)?;
    if tokens.is_empty() {
        return Err(CompileError::InvalidExpression(
            "expression is empty".to_string(),
        ));
    }

    JoinConditionParser::new(tokens).parse()
}

/// Validate and rewrite alias.column references to suffixed Polars columns.
pub fn compile_assignment_expression(
    expression: &str,
    symbols: &ExpressionSymbolTable,
) -> Result<String, CompileError> {
    let mut compiled = String::with_capacity(expression.len());
    let mut cursor = 0usize;

    for reference in extract_reference_spans(expression) {
        compiled.push_str(&expression[cursor..reference.start]);
        let mapped = symbols.map_reference(&expression[reference.start..reference.end])?;
        compiled.push_str(&mapped);
        cursor = reference.end;
    }
    compiled.push_str(&expression[cursor..]);

    Ok(compiled)
}

pub fn extract_assignment_alias_column_references(expression: &str) -> Vec<(String, String)> {
    extract_reference_spans(expression)
        .into_iter()
        .filter_map(|reference| {
            expression[reference.start..reference.end]
                .split_once('.')
                .map(|(alias, column)| {
                    let column_name = if column.starts_with('"') && column.ends_with('"') {
                        &column[1..column.len() - 1]
                    } else {
                        column
                    };
                    (alias.to_string(), column_name.to_string())
                })
        })
        .collect()
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn maps_join_alias_columns() {
        let mut symbols = ExpressionSymbolTable::with_working_columns(["amount_local".to_string()]);
        symbols.add_join_alias("fx", ["rate".to_string()]);

        let compiled = compile_assignment_expression("amount_local * fx.rate", &symbols)
            .expect("expression compiles");
        assert_eq!(compiled, "amount_local * rate_fx");
    }

    #[test]
    fn validates_unaliased_assignment_columns() {
        let mut symbols = ExpressionSymbolTable::with_working_columns(["amount_local".to_string()]);
        symbols.add_join_alias("fx", ["rate".to_string()]);

        let error = compile_assignment_expression("amount_locl * fx.rate", &symbols)
            .expect_err("unknown unaliased column should fail");
        assert!(matches!(error, CompileError::UnknownColumn(column) if column == "amount_locl"));
    }

    #[test]
    fn ignores_function_names_during_assignment_compilation() {
        let mut symbols = ExpressionSymbolTable::with_working_columns(["amount_local".to_string()]);
        symbols.add_join_alias("fx", ["rate".to_string()]);

        let compiled = compile_assignment_expression(
            "IF(amount_local > 0, amount_local * fx.rate, 0)",
            &symbols,
        )
        .expect("expression compiles");

        assert_eq!(compiled, "IF(amount_local > 0, amount_local * rate_fx, 0)");
    }

    #[test]
    fn extracts_only_aliased_references_from_assignments() {
        let references = extract_assignment_alias_column_references(
            r#"CONCAT("unknown.rate", fx.rate, customers.tier, amount_local)"#,
        );

        assert_eq!(
            references,
            vec![
                ("fx".to_string(), "rate".to_string()),
                ("customers".to_string(), "tier".to_string()),
            ]
        );
    }

    #[test]
    fn parses_grouped_join_condition_expression() {
        let condition = compile_join_condition_expression(
            "currency = fx.from_currency AND (fx.to_currency = 'USD' OR fx.rate > 1.0)",
        )
        .expect("condition should parse");

        match condition {
            JoinConditionExpr::Logical { op, .. } => assert_eq!(op, JoinLogicalOp::And),
            _ => panic!("expected top-level logical expression"),
        }
    }

    #[test]
    fn rejects_unterminated_string_literal_in_join_condition() {
        let error = compile_join_condition_expression("currency = 'USD")
            .expect_err("unterminated string literals should fail");
        assert!(matches!(
            error,
            CompileError::InvalidExpression(message) if message == "unterminated string literal"
        ));
    }
}
