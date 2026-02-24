use polars::prelude::{col, lit, DataFrame, IntoLazy, LazyFrame};

use crate::engine::error::AppendError;
use crate::model::TemporalMode;

pub fn apply_temporal_filter(
    frame: &DataFrame,
    temporal_mode: Option<TemporalMode>,
    run_period: Option<&str>,
    as_of_date: Option<&str>,
) -> Result<DataFrame, AppendError> {
    apply_temporal_filter_lazy(frame.clone().lazy(), temporal_mode, run_period, as_of_date)?
        .collect()
        .map_err(|error| AppendError::DataLoadError {
            message: error.to_string(),
        })
}

pub fn apply_temporal_filter_lazy(
    frame: LazyFrame,
    temporal_mode: Option<TemporalMode>,
    run_period: Option<&str>,
    as_of_date: Option<&str>,
) -> Result<LazyFrame, AppendError> {
    match temporal_mode {
        Some(TemporalMode::Period) => {
            let period = run_period.ok_or_else(|| AppendError::ExpressionParseError {
                expression: "_period = run_period".to_owned(),
                error: "run_period is required for period datasets".to_owned(),
            })?;
            Ok(frame.filter(col("_period").eq(lit(period))))
        }
        Some(TemporalMode::Bitemporal) => {
            let as_of = as_of_date.ok_or_else(|| AppendError::ExpressionParseError {
                expression: "_period_from/_period_to asOf".to_owned(),
                error: "as_of_date is required for bitemporal datasets".to_owned(),
            })?;
            let schema =
                frame
                    .clone()
                    .collect_schema()
                    .map_err(|error| AppendError::DataLoadError {
                        message: error.to_string(),
                    })?;
            let has_canonical =
                schema.get("_period_from").is_some() && schema.get("_period_to").is_some();
            let has_legacy = schema.get("valid_from").is_some() && schema.get("valid_to").is_some();
            if has_canonical {
                return Ok(frame.filter(
                    col("_period_from").lt_eq(lit(as_of)).and(
                        col("_period_to")
                            .gt(lit(as_of))
                            .or(col("_period_to").is_null()),
                    ),
                ));
            }
            if has_legacy {
                return Ok(frame.filter(
                    col("valid_from")
                        .lt_eq(lit(as_of))
                        .and(col("valid_to").gt(lit(as_of)).or(col("valid_to").is_null())),
                ));
            }
            Err(AppendError::ExpressionParseError {
                expression: "_period_from/_period_to asOf".to_owned(),
                error: "bitemporal datasets require either _period_from/_period_to or valid_from/valid_to columns".to_owned(),
            })
        }
        Some(TemporalMode::Snapshot) | None => Ok(frame),
    }
}
