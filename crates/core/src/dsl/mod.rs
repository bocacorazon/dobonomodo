use chrono::{DateTime, Utc};
use polars::prelude::*;
use thiserror::Error;

pub fn module_name() -> &'static str {
    "dsl"
}

#[derive(Debug, Clone)]
pub struct ExpressionCompileContext {
    pub run_timestamp: DateTime<Utc>,
    pub allow_aggregates: bool,
}

impl ExpressionCompileContext {
    pub fn for_row_level(run_timestamp: DateTime<Utc>) -> Self {
        Self {
            run_timestamp,
            allow_aggregates: false,
        }
    }
}

#[derive(Debug, Error)]
pub enum ExpressionError {
    #[error("Expression is empty")]
    EmptyExpression,
    #[error("{0}")]
    Message(String),
}

type Result<T> = std::result::Result<T, ExpressionError>;

pub fn compile_expression(source: &str, context: &ExpressionCompileContext) -> Result<Expr> {
    let source = source.trim();
    if source.is_empty() {
        return Err(ExpressionError::EmptyExpression);
    }

    let tokens = tokenize_expression(source)?;
    let mut parser = ExpressionParser::new(tokens, context);
    parser.parse_expression()
}

#[derive(Debug, Clone, PartialEq)]
enum Token {
    Identifier(String),
    Number(String),
    String(String),
    True,
    False,
    Null,
    LParen,
    RParen,
    Comma,
    Plus,
    Minus,
    Star,
    Slash,
    Percent,
    Eq,
    NotEq,
    Gt,
    GtEq,
    Lt,
    LtEq,
    And,
    Or,
    Not,
}

fn tokenize_expression(source: &str) -> Result<Vec<Token>> {
    let mut tokens = Vec::new();
    let chars: Vec<(usize, char)> = source.char_indices().collect();
    let mut index = 0usize;

    while index < chars.len() {
        let (byte_idx, ch) = chars[index];
        if ch.is_whitespace() {
            index += 1;
            continue;
        }

        let next_char = chars.get(index + 1).map(|(_, c)| *c);
        match ch {
            '(' => {
                tokens.push(Token::LParen);
                index += 1;
            }
            ')' => {
                tokens.push(Token::RParen);
                index += 1;
            }
            ',' => {
                tokens.push(Token::Comma);
                index += 1;
            }
            '+' => {
                tokens.push(Token::Plus);
                index += 1;
            }
            '-' => {
                tokens.push(Token::Minus);
                index += 1;
            }
            '*' => {
                tokens.push(Token::Star);
                index += 1;
            }
            '/' => {
                tokens.push(Token::Slash);
                index += 1;
            }
            '%' => {
                tokens.push(Token::Percent);
                index += 1;
            }
            '=' => {
                if next_char == Some('=') {
                    index += 2;
                } else {
                    index += 1;
                }
                tokens.push(Token::Eq);
            }
            '!' => {
                if next_char == Some('=') {
                    tokens.push(Token::NotEq);
                    index += 2;
                } else {
                    return Err(ExpressionError::Message(format!(
                        "Unexpected token '!' at byte position {}",
                        byte_idx
                    )));
                }
            }
            '>' => {
                if next_char == Some('=') {
                    tokens.push(Token::GtEq);
                    index += 2;
                } else {
                    tokens.push(Token::Gt);
                    index += 1;
                }
            }
            '<' => {
                if next_char == Some('=') {
                    tokens.push(Token::LtEq);
                    index += 2;
                } else if next_char == Some('>') {
                    tokens.push(Token::NotEq);
                    index += 2;
                } else {
                    tokens.push(Token::Lt);
                    index += 1;
                }
            }
            '"' => {
                let mut parsed = String::new();
                index += 1;
                let mut escaped = false;
                let mut closed = false;

                while index < chars.len() {
                    let (_, current) = chars[index];
                    index += 1;

                    if escaped {
                        let unescaped = match current {
                            '"' => '"',
                            '\\' => '\\',
                            'n' => '\n',
                            'r' => '\r',
                            't' => '\t',
                            other => other,
                        };
                        parsed.push(unescaped);
                        escaped = false;
                        continue;
                    }

                    if current == '\\' {
                        escaped = true;
                        continue;
                    }

                    if current == '"' {
                        closed = true;
                        break;
                    }

                    parsed.push(current);
                }

                if !closed {
                    return Err(ExpressionError::Message(
                        "Unterminated string literal".to_string(),
                    ));
                }

                tokens.push(Token::String(parsed));
            }
            c if c.is_ascii_digit() || c == '.' => {
                let start = byte_idx;
                let mut has_dot = c == '.';
                index += 1;

                while index < chars.len() {
                    let (_, current) = chars[index];
                    if current.is_ascii_digit() {
                        index += 1;
                        continue;
                    }
                    if current == '.' && !has_dot {
                        has_dot = true;
                        index += 1;
                        continue;
                    }
                    break;
                }

                let end = chars.get(index).map_or(source.len(), |(pos, _)| *pos);
                tokens.push(Token::Number(source[start..end].to_string()));
            }
            c if c == '_' || c.is_alphabetic() => {
                let start = byte_idx;
                index += 1;

                while index < chars.len() {
                    let (_, current) = chars[index];
                    if current == '_' || current == '.' || current.is_alphanumeric() {
                        index += 1;
                    } else {
                        break;
                    }
                }

                let end = chars.get(index).map_or(source.len(), |(pos, _)| *pos);
                let raw = &source[start..end];
                let token = if raw.eq_ignore_ascii_case("TRUE") {
                    Token::True
                } else if raw.eq_ignore_ascii_case("FALSE") {
                    Token::False
                } else if raw.eq_ignore_ascii_case("NULL") {
                    Token::Null
                } else if raw.eq_ignore_ascii_case("AND") {
                    Token::And
                } else if raw.eq_ignore_ascii_case("OR") {
                    Token::Or
                } else if raw.eq_ignore_ascii_case("NOT") {
                    Token::Not
                } else {
                    Token::Identifier(raw.to_string())
                };
                tokens.push(token);
            }
            _ => {
                return Err(ExpressionError::Message(format!(
                    "Unsupported token '{}' at byte position {}",
                    ch, byte_idx
                )));
            }
        }
    }

    Ok(tokens)
}

struct ExpressionParser<'a> {
    tokens: Vec<Token>,
    cursor: usize,
    context: &'a ExpressionCompileContext,
}

impl<'a> ExpressionParser<'a> {
    fn new(tokens: Vec<Token>, context: &'a ExpressionCompileContext) -> Self {
        Self {
            tokens,
            cursor: 0,
            context,
        }
    }

    fn parse_expression(&mut self) -> Result<Expr> {
        let expr = self.parse_or_expr()?;
        if self.cursor != self.tokens.len() {
            return Err(ExpressionError::Message(
                "Unexpected token at end of expression".to_string(),
            ));
        }
        Ok(expr)
    }

    fn parse_or_expr(&mut self) -> Result<Expr> {
        let mut left = self.parse_and_expr()?;
        while self.consume_if(|token| matches!(token, Token::Or)) {
            let right = self.parse_and_expr()?;
            left = left.or(right);
        }
        Ok(left)
    }

    fn parse_and_expr(&mut self) -> Result<Expr> {
        let mut left = self.parse_not_expr()?;
        while self.consume_if(|token| matches!(token, Token::And)) {
            let right = self.parse_not_expr()?;
            left = left.and(right);
        }
        Ok(left)
    }

    fn parse_not_expr(&mut self) -> Result<Expr> {
        if self.consume_if(|token| matches!(token, Token::Not)) {
            return Ok(self.parse_not_expr()?.not());
        }
        self.parse_comparison_expr()
    }

    fn parse_comparison_expr(&mut self) -> Result<Expr> {
        let left = self.parse_add_sub_expr()?;
        let Some(op) = self
            .peek()
            .filter(|token| {
                matches!(
                    token,
                    Token::Eq | Token::NotEq | Token::Gt | Token::GtEq | Token::Lt | Token::LtEq
                )
            })
            .cloned()
        else {
            return Ok(left);
        };

        self.cursor += 1;
        let right = self.parse_add_sub_expr()?;
        match op {
            Token::Eq => Ok(left.eq(right)),
            Token::NotEq => Ok(left.neq(right)),
            Token::Gt => Ok(left.gt(right)),
            Token::GtEq => Ok(left.gt_eq(right)),
            Token::Lt => Ok(left.lt(right)),
            Token::LtEq => Ok(left.lt_eq(right)),
            _ => Err(ExpressionError::Message(
                "Unsupported comparison operator".to_string(),
            )),
        }
    }

    fn parse_add_sub_expr(&mut self) -> Result<Expr> {
        let mut left = self.parse_mul_div_expr()?;
        loop {
            if self.consume_if(|token| matches!(token, Token::Plus)) {
                left = left + self.parse_mul_div_expr()?;
                continue;
            }
            if self.consume_if(|token| matches!(token, Token::Minus)) {
                left = left - self.parse_mul_div_expr()?;
                continue;
            }
            break;
        }
        Ok(left)
    }

    fn parse_mul_div_expr(&mut self) -> Result<Expr> {
        let mut left = self.parse_unary_expr()?;
        loop {
            if self.consume_if(|token| matches!(token, Token::Star)) {
                left = left * self.parse_unary_expr()?;
                continue;
            }
            if self.consume_if(|token| matches!(token, Token::Slash)) {
                left = left / self.parse_unary_expr()?;
                continue;
            }
            if self.consume_if(|token| matches!(token, Token::Percent)) {
                left = left % self.parse_unary_expr()?;
                continue;
            }
            break;
        }
        Ok(left)
    }

    fn parse_unary_expr(&mut self) -> Result<Expr> {
        if self.consume_if(|token| matches!(token, Token::Minus)) {
            return Ok(lit(0) - self.parse_unary_expr()?);
        }
        self.parse_primary_expr()
    }

    fn parse_primary_expr(&mut self) -> Result<Expr> {
        match self.next().cloned() {
            Some(Token::LParen) => {
                let expr = self.parse_or_expr()?;
                self.expect(|token| matches!(token, Token::RParen), "Expected ')'")?;
                Ok(expr)
            }
            Some(Token::True) => Ok(lit(true)),
            Some(Token::False) => Ok(lit(false)),
            Some(Token::Null) => Ok(lit(LiteralValue::Null)),
            Some(Token::String(value)) => Ok(lit(value)),
            Some(Token::Number(value)) => {
                if let Ok(v) = value.parse::<i64>() {
                    Ok(lit(v))
                } else {
                    let v = value.parse::<f64>().map_err(|_| {
                        ExpressionError::Message(format!("Invalid number literal '{value}'"))
                    })?;
                    Ok(lit(v))
                }
            }
            Some(Token::Identifier(name)) => {
                if self.consume_if(|token| matches!(token, Token::LParen)) {
                    let args = self.parse_call_args()?;
                    self.compile_function(&name, args)
                } else {
                    Ok(col(&name))
                }
            }
            _ => Err(ExpressionError::Message("Expected expression".to_string())),
        }
    }

    fn parse_call_args(&mut self) -> Result<Vec<Expr>> {
        let mut args = Vec::new();
        if self.consume_if(|token| matches!(token, Token::RParen)) {
            return Ok(args);
        }

        loop {
            args.push(self.parse_or_expr()?);
            if self.consume_if(|token| matches!(token, Token::Comma)) {
                continue;
            }
            self.expect(
                |token| matches!(token, Token::RParen),
                "Expected ')' after function arguments",
            )?;
            break;
        }
        Ok(args)
    }

    fn compile_function(&self, name: &str, args: Vec<Expr>) -> Result<Expr> {
        let normalized = name.to_ascii_uppercase();
        match normalized.as_str() {
            "IF" => {
                if args.len() != 3 {
                    return Err(ExpressionError::Message(
                        "IF expects 3 arguments".to_string(),
                    ));
                }
                let mut args = args.into_iter();
                let condition = args
                    .next()
                    .ok_or_else(|| ExpressionError::Message("IF missing condition".to_string()))?;
                let if_true = args.next().ok_or_else(|| {
                    ExpressionError::Message("IF missing true branch".to_string())
                })?;
                let if_false = args.next().ok_or_else(|| {
                    ExpressionError::Message("IF missing false branch".to_string())
                })?;
                Ok(when(condition).then(if_true).otherwise(if_false))
            }
            "AND" => {
                if args.is_empty() {
                    return Err(ExpressionError::Message(
                        "AND expects at least 1 argument".to_string(),
                    ));
                }
                let mut args = args.into_iter();
                let first = args.next().ok_or_else(|| {
                    ExpressionError::Message("AND expects at least 1 argument".to_string())
                })?;
                Ok(args.fold(first, |acc, expr| acc.and(expr)))
            }
            "OR" => {
                if args.is_empty() {
                    return Err(ExpressionError::Message(
                        "OR expects at least 1 argument".to_string(),
                    ));
                }
                let mut args = args.into_iter();
                let first = args.next().ok_or_else(|| {
                    ExpressionError::Message("OR expects at least 1 argument".to_string())
                })?;
                Ok(args.fold(first, |acc, expr| acc.or(expr)))
            }
            "NOT" => {
                if args.len() != 1 {
                    return Err(ExpressionError::Message(
                        "NOT expects 1 argument".to_string(),
                    ));
                }
                let expr = args.into_iter().next().ok_or_else(|| {
                    ExpressionError::Message("NOT expects 1 argument".to_string())
                })?;
                Ok(expr.not())
            }
            "ISNULL" => {
                if args.len() != 1 {
                    return Err(ExpressionError::Message(
                        "ISNULL expects 1 argument".to_string(),
                    ));
                }
                let expr = args.into_iter().next().ok_or_else(|| {
                    ExpressionError::Message("ISNULL expects 1 argument".to_string())
                })?;
                Ok(expr.is_null())
            }
            "COALESCE" => {
                if args.is_empty() {
                    return Err(ExpressionError::Message(
                        "COALESCE expects at least 1 argument".to_string(),
                    ));
                }
                let mut args = args.into_iter();
                let first = args.next().ok_or_else(|| {
                    ExpressionError::Message("COALESCE expects at least 1 argument".to_string())
                })?;
                Ok(args.fold(first, |acc, expr| {
                    when(acc.clone().is_not_null()).then(acc).otherwise(expr)
                }))
            }
            "ABS" => {
                if args.len() != 1 {
                    return Err(ExpressionError::Message(
                        "ABS expects 1 argument".to_string(),
                    ));
                }
                let expr = args.into_iter().next().ok_or_else(|| {
                    ExpressionError::Message("ABS expects 1 argument".to_string())
                })?;
                Ok(when(expr.clone().lt(lit(0)))
                    .then(lit(0) - expr.clone())
                    .otherwise(expr))
            }
            "ROUND" => {
                if !(1..=2).contains(&args.len()) {
                    return Err(ExpressionError::Message(
                        "ROUND expects 1 or 2 arguments".to_string(),
                    ));
                }
                let mut args = args.into_iter();
                let value = args.next().ok_or_else(|| {
                    ExpressionError::Message("ROUND missing number argument".to_string())
                })?;
                let decimals = match args.next() {
                    Some(expr) => literal_i32(&expr).ok_or_else(|| {
                        ExpressionError::Message(
                            "ROUND decimals must be a numeric literal".to_string(),
                        )
                    })?,
                    None => 0,
                };

                let factor = 10_f64.powi(decimals.abs());
                if decimals >= 0 {
                    Ok(((value * lit(factor)).round(0)) / lit(factor))
                } else {
                    Ok(((value / lit(factor)).round(0)) * lit(factor))
                }
            }
            "FLOOR" => {
                if args.len() != 1 {
                    return Err(ExpressionError::Message(
                        "FLOOR expects 1 argument".to_string(),
                    ));
                }
                let expr = args.into_iter().next().ok_or_else(|| {
                    ExpressionError::Message("FLOOR expects 1 argument".to_string())
                })?;
                Ok(expr.floor())
            }
            "CEIL" => {
                if args.len() != 1 {
                    return Err(ExpressionError::Message(
                        "CEIL expects 1 argument".to_string(),
                    ));
                }
                let expr = args.into_iter().next().ok_or_else(|| {
                    ExpressionError::Message("CEIL expects 1 argument".to_string())
                })?;
                Ok(expr.ceil())
            }
            "MOD" => {
                if args.len() != 2 {
                    return Err(ExpressionError::Message(
                        "MOD expects 2 arguments".to_string(),
                    ));
                }
                let mut args = args.into_iter();
                let left = args.next().ok_or_else(|| {
                    ExpressionError::Message("MOD missing first argument".to_string())
                })?;
                let right = args.next().ok_or_else(|| {
                    ExpressionError::Message("MOD missing second argument".to_string())
                })?;
                Ok(left % right)
            }
            "MIN" => {
                if args.len() != 2 {
                    return Err(ExpressionError::Message(
                        "MIN expects 2 arguments".to_string(),
                    ));
                }
                let mut args = args.into_iter();
                let left = args.next().ok_or_else(|| {
                    ExpressionError::Message("MIN missing first argument".to_string())
                })?;
                let right = args.next().ok_or_else(|| {
                    ExpressionError::Message("MIN missing second argument".to_string())
                })?;
                Ok(when(left.clone().lt(right.clone()))
                    .then(left)
                    .otherwise(right))
            }
            "MAX" => {
                if args.len() != 2 {
                    return Err(ExpressionError::Message(
                        "MAX expects 2 arguments".to_string(),
                    ));
                }
                let mut args = args.into_iter();
                let left = args.next().ok_or_else(|| {
                    ExpressionError::Message("MAX missing first argument".to_string())
                })?;
                let right = args.next().ok_or_else(|| {
                    ExpressionError::Message("MAX missing second argument".to_string())
                })?;
                Ok(when(left.clone().gt(right.clone()))
                    .then(left)
                    .otherwise(right))
            }
            "CONCAT" => {
                if args.is_empty() {
                    return Err(ExpressionError::Message(
                        "CONCAT expects at least 1 argument".to_string(),
                    ));
                }
                Ok(concat_str(args, "", false))
            }
            "UPPER" => one_arg(name, args).map(|expr| expr.str().to_uppercase()),
            "LOWER" => one_arg(name, args).map(|expr| expr.str().to_lowercase()),
            "TRIM" => {
                one_arg(name, args).map(|expr| expr.str().strip_chars(lit(LiteralValue::Null)))
            }
            "LEFT" => {
                let (value, length) = two_args(name, args)?;
                Ok(value.str().head(length))
            }
            "RIGHT" => {
                let (value, length) = two_args(name, args)?;
                Ok(value.str().tail(length))
            }
            "LEN" => one_arg(name, args).map(|expr| expr.str().len_chars()),
            "CONTAINS" => {
                let (value, pattern) = two_args(name, args)?;
                Ok(value.str().contains_literal(pattern))
            }
            "REPLACE" => {
                let (value, old_value, new_value) = three_args(name, args)?;
                Ok(value.str().replace_all(old_value, new_value, true))
            }
            "DATE" => one_arg(name, args).map(|expr| {
                expr.cast(DataType::String)
                    .str()
                    .to_date(StrptimeOptions::default())
            }),
            "YEAR" => one_arg(name, args).map(|expr| expr.cast(DataType::Date).dt().year()),
            "MONTH" => one_arg(name, args).map(|expr| expr.cast(DataType::Date).dt().month()),
            "DAY" => one_arg(name, args).map(|expr| expr.cast(DataType::Date).dt().day()),
            "DATEDIFF" => {
                let (end_date, start_date) = two_args(name, args)?;
                Ok(end_date.cast(DataType::Date).cast(DataType::Int32)
                    - start_date.cast(DataType::Date).cast(DataType::Int32))
            }
            "DATEADD" => {
                let (date_value, n_days) = two_args(name, args)?;
                Ok((date_value.cast(DataType::Date).cast(DataType::Int32)
                    + n_days.cast(DataType::Int32))
                .cast(DataType::Date))
            }
            "TODAY" => {
                if !args.is_empty() {
                    return Err(ExpressionError::Message(
                        "TODAY expects 0 arguments".to_string(),
                    ));
                }
                Ok(lit(self.context.run_timestamp.timestamp_millis())
                    .cast(DataType::Datetime(TimeUnit::Milliseconds, None))
                    .cast(DataType::Date))
            }
            "SUM" | "COUNT" | "COUNT_ALL" | "AVG" | "MIN_AGG" | "MAX_AGG" => {
                if !self.context.allow_aggregates {
                    return Err(ExpressionError::Message(format!(
                        "Aggregate function '{normalized}' is not allowed in this operation",
                    )));
                }
                match normalized.as_str() {
                    "SUM" => one_arg(name, args).map(|expr| expr.sum()),
                    "COUNT" => one_arg(name, args).map(|expr| expr.count()),
                    "COUNT_ALL" => {
                        if !args.is_empty() {
                            return Err(ExpressionError::Message(
                                "COUNT_ALL expects 0 arguments".to_string(),
                            ));
                        }
                        Ok(len().cast(DataType::Int64))
                    }
                    "AVG" => one_arg(name, args).map(|expr| expr.mean()),
                    "MIN_AGG" => one_arg(name, args).map(|expr| expr.min()),
                    "MAX_AGG" => one_arg(name, args).map(|expr| expr.max()),
                    _ => Err(ExpressionError::Message(format!(
                        "Unsupported function '{name}'",
                    ))),
                }
            }
            _ => Err(ExpressionError::Message(format!(
                "Unsupported function '{name}'",
            ))),
        }
    }

    fn peek(&self) -> Option<&Token> {
        self.tokens.get(self.cursor)
    }

    fn next(&mut self) -> Option<&Token> {
        let token = self.tokens.get(self.cursor);
        if token.is_some() {
            self.cursor += 1;
        }
        token
    }

    fn consume_if(&mut self, predicate: impl Fn(&Token) -> bool) -> bool {
        let Some(token) = self.peek() else {
            return false;
        };
        if !predicate(token) {
            return false;
        }
        self.cursor += 1;
        true
    }

    fn expect(&mut self, predicate: impl Fn(&Token) -> bool, message: &str) -> Result<()> {
        if self.consume_if(predicate) {
            Ok(())
        } else {
            Err(ExpressionError::Message(message.to_string()))
        }
    }
}

fn literal_i32(expr: &Expr) -> Option<i32> {
    match expr {
        Expr::Literal(LiteralValue::Int32(v)) => Some(*v),
        Expr::Literal(LiteralValue::Int64(v)) => i32::try_from(*v).ok(),
        Expr::Literal(LiteralValue::UInt32(v)) => i32::try_from(*v).ok(),
        Expr::Literal(LiteralValue::UInt64(v)) => i32::try_from(*v).ok(),
        Expr::Literal(LiteralValue::Int(v)) => i32::try_from(*v).ok(),
        Expr::Literal(LiteralValue::Float32(v)) if v.fract() == 0.0 => Some(*v as i32),
        Expr::Literal(LiteralValue::Float64(v)) if v.fract() == 0.0 => Some(*v as i32),
        _ => None,
    }
}

fn one_arg(name: &str, args: Vec<Expr>) -> Result<Expr> {
    if args.len() != 1 {
        return Err(ExpressionError::Message(format!(
            "{name} expects 1 argument",
        )));
    }
    args.into_iter()
        .next()
        .ok_or_else(|| ExpressionError::Message(format!("{name} expects 1 argument")))
}

fn two_args(name: &str, args: Vec<Expr>) -> Result<(Expr, Expr)> {
    if args.len() != 2 {
        return Err(ExpressionError::Message(format!(
            "{name} expects 2 arguments",
        )));
    }

    let mut args = args.into_iter();
    let first = args
        .next()
        .ok_or_else(|| ExpressionError::Message(format!("{name} expects 2 arguments")))?;
    let second = args
        .next()
        .ok_or_else(|| ExpressionError::Message(format!("{name} expects 2 arguments")))?;
    Ok((first, second))
}

fn three_args(name: &str, args: Vec<Expr>) -> Result<(Expr, Expr, Expr)> {
    if args.len() != 3 {
        return Err(ExpressionError::Message(format!(
            "{name} expects 3 arguments",
        )));
    }

    let mut args = args.into_iter();
    let first = args
        .next()
        .ok_or_else(|| ExpressionError::Message(format!("{name} expects 3 arguments")))?;
    let second = args
        .next()
        .ok_or_else(|| ExpressionError::Message(format!("{name} expects 3 arguments")))?;
    let third = args
        .next()
        .ok_or_else(|| ExpressionError::Message(format!("{name} expects 3 arguments")))?;
    Ok((first, second, third))
}
