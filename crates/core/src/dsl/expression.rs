use polars::prelude::{col, lit, Expr};

#[derive(Debug, Clone, PartialEq, Eq)]
enum ComparisonOp {
    Eq,
    Ne,
    Gt,
    Ge,
    Lt,
    Le,
}

pub fn parse_source_selector(selector: &str) -> Result<Expr, String> {
    let trimmed = selector.trim();
    if trimmed.is_empty() {
        return Err("source_selector cannot be empty".to_owned());
    }

    if let Some((lhs, rhs)) = split_logical(trimmed, " OR ") {
        return Ok(parse_source_selector(lhs)?.or(parse_source_selector(rhs)?));
    }
    if let Some((lhs, rhs)) = split_logical(trimmed, " AND ") {
        return Ok(parse_source_selector(lhs)?.and(parse_source_selector(rhs)?));
    }

    let (column, op, literal) = parse_comparison(trimmed)?;
    let rhs = parse_literal(literal);
    let lhs = col(column);

    Ok(match op {
        ComparisonOp::Eq => lhs.eq(rhs),
        ComparisonOp::Ne => lhs.neq(rhs),
        ComparisonOp::Gt => lhs.gt(rhs),
        ComparisonOp::Ge => lhs.gt_eq(rhs),
        ComparisonOp::Lt => lhs.lt(rhs),
        ComparisonOp::Le => lhs.lt_eq(rhs),
    })
}

fn parse_literal(raw: &str) -> Expr {
    if let Some(stripped) = raw.strip_prefix('\'').and_then(|v| v.strip_suffix('\'')) {
        return lit(stripped);
    }
    if let Ok(value) = raw.parse::<i64>() {
        return lit(value);
    }
    if let Ok(value) = raw.parse::<f64>() {
        return lit(value);
    }
    if raw.eq_ignore_ascii_case("true") {
        return lit(true);
    }
    if raw.eq_ignore_ascii_case("false") {
        return lit(false);
    }

    lit(raw)
}

fn split_logical<'a>(input: &'a str, needle: &str) -> Option<(&'a str, &'a str)> {
    let mut in_quotes = false;
    for (idx, ch) in input.char_indices() {
        if ch == '\'' {
            in_quotes = !in_quotes;
        }
        if !in_quotes && input[idx..].starts_with(needle) {
            let lhs = &input[..idx];
            let rhs = &input[idx + needle.len()..];
            return Some((lhs, rhs));
        }
    }
    None
}

fn parse_comparison(input: &str) -> Result<(&str, ComparisonOp, &str), String> {
    const OPS: [(&str, ComparisonOp); 6] = [
        ("!=", ComparisonOp::Ne),
        (">=", ComparisonOp::Ge),
        ("<=", ComparisonOp::Le),
        ("=", ComparisonOp::Eq),
        (">", ComparisonOp::Gt),
        ("<", ComparisonOp::Lt),
    ];

    let mut in_quotes = false;
    for (idx, ch) in input.char_indices() {
        if ch == '\'' {
            in_quotes = !in_quotes;
            continue;
        }
        if in_quotes {
            continue;
        }
        for (token, op) in OPS {
            if input[idx..].starts_with(token) {
                let lhs = input[..idx].trim();
                let rhs = input[idx + token.len()..].trim();
                if lhs.is_empty() || rhs.is_empty() {
                    return Err(format!("invalid comparison expression '{input}'"));
                }
                return Ok((lhs, op, rhs));
            }
        }
    }

    Err(format!("invalid comparison expression '{input}'"))
}
