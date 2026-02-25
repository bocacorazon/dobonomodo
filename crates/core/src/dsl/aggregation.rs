#[derive(Debug, Clone, PartialEq, Eq)]
pub enum AggregateFunction {
    Sum,
    Count,
    Avg,
    MinAgg,
    MaxAgg,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct ParsedAggregation {
    pub function: AggregateFunction,
    pub input_column: String,
}

pub fn parse_aggregation(expression: &str) -> Result<ParsedAggregation, String> {
    let trimmed = expression.trim();
    let open_idx = trimmed
        .find('(')
        .ok_or_else(|| format!("invalid aggregation expression '{trimmed}'"))?;
    let close_idx = trimmed
        .rfind(')')
        .ok_or_else(|| format!("invalid aggregation expression '{trimmed}'"))?;
    if close_idx <= open_idx + 1 || close_idx != trimmed.len() - 1 {
        return Err(format!("invalid aggregation expression '{trimmed}'"));
    }

    let function = match trimmed[..open_idx].trim().to_ascii_uppercase().as_str() {
        "SUM" => AggregateFunction::Sum,
        "COUNT" => AggregateFunction::Count,
        "AVG" => AggregateFunction::Avg,
        "MIN_AGG" => AggregateFunction::MinAgg,
        "MAX_AGG" => AggregateFunction::MaxAgg,
        other => return Err(format!("unsupported aggregate function '{other}'")),
    };
    let input_column = trimmed[open_idx + 1..close_idx].trim().to_owned();
    if input_column.is_empty() {
        return Err(format!("invalid aggregation expression '{trimmed}'"));
    }
    if input_column == "*" && function != AggregateFunction::Count {
        return Err(format!(
            "wildcard input is only supported for COUNT(*) in '{trimmed}'"
        ));
    }

    Ok(ParsedAggregation {
        function,
        input_column,
    })
}
