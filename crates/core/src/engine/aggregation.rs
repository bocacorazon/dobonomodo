use polars::prelude::{col, len, DataFrame, DataType, Expr, IntoLazy, LazyFrame};

use crate::dsl::aggregation::{parse_aggregation, AggregateFunction};
use crate::engine::error::AppendError;
use crate::model::AppendAggregation;

pub fn build_agg_expressions(config: &AppendAggregation) -> Result<Vec<Expr>, AppendError> {
    config
        .aggregations
        .iter()
        .map(|agg| {
            let parsed = parse_aggregation(&agg.expression).map_err(|error| {
                AppendError::AggregationError {
                    message: format!("{} ({error})", agg.expression),
                }
            })?;
            let expr = match parsed.function {
                AggregateFunction::Sum => col(&parsed.input_column).sum(),
                AggregateFunction::Count if parsed.input_column == "*" => {
                    len().cast(DataType::Int64)
                }
                AggregateFunction::Count => col(&parsed.input_column).count().cast(DataType::Int64),
                AggregateFunction::Avg => col(&parsed.input_column).mean(),
                AggregateFunction::MinAgg => col(&parsed.input_column).min(),
                AggregateFunction::MaxAgg => col(&parsed.input_column).max(),
            };
            Ok(expr.alias(&agg.column))
        })
        .collect()
}

pub fn apply_aggregation(
    frame: &DataFrame,
    config: &AppendAggregation,
) -> Result<DataFrame, AppendError> {
    apply_aggregation_lazy(frame.clone().lazy(), config)?
        .collect()
        .map_err(|error| AppendError::AggregationError {
            message: error.to_string(),
        })
}

pub fn apply_aggregation_lazy(
    frame: LazyFrame,
    config: &AppendAggregation,
) -> Result<LazyFrame, AppendError> {
    let group_by_exprs = config.group_by.iter().map(col).collect::<Vec<_>>();
    let agg_exprs = build_agg_expressions(config)?;

    Ok(frame.group_by(group_by_exprs).agg(agg_exprs))
}
