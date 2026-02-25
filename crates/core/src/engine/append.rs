use std::collections::BTreeSet;

use polars::prelude::{col, len, lit, DataFrame, IntoLazy, LazyFrame, NamedFrom, Series};
use uuid::Uuid;

use crate::dsl::aggregation::parse_aggregation;
use crate::dsl::expression::{extract_selector_columns, parse_source_selector};
use crate::engine::aggregation::apply_aggregation_lazy;
use crate::engine::error::AppendError;
use crate::engine::io_traits::DataLoader;
use crate::engine::temporal::apply_temporal_filter_lazy;
use crate::model::{
    AppendOperation, Dataset, Expression, Project, ResolutionRule, ResolutionStrategy,
    ResolvedLocation, Resolver,
};
use crate::{MetadataStore, MetadataStoreError};

#[derive(Debug, Clone, Default)]
pub struct AppendExecutionContext {
    pub run_period: Option<String>,
    pub as_of_date: Option<String>,
    pub operation_seq: u32,
}

#[derive(Debug, Clone)]
pub struct AppendResult {
    pub frame: DataFrame,
    pub rows_appended: usize,
    pub source_rows_loaded: usize,
    pub source_rows_after_selector: usize,
}

pub fn validate_append_operation<M: MetadataStore>(
    metadata_store: &M,
    operation: &AppendOperation,
) -> Result<(), AppendError> {
    get_source_dataset(metadata_store, operation)?;
    Ok(())
}

pub fn resolve_and_load_source<M: MetadataStore, D: DataLoader>(
    metadata_store: &M,
    data_loader: &D,
    project: &Project,
    operation: &AppendOperation,
    context: &AppendExecutionContext,
) -> Result<LazyFrame, AppendError> {
    validate_append_operation(metadata_store, operation)?;

    let dataset = get_source_dataset(metadata_store, operation)?;
    let resolver = resolve_source_resolver(metadata_store, project, &dataset, operation)?;

    let location = resolve_location(&dataset, &resolver, context.run_period.as_deref())?;
    let frame = data_loader
        .load(&location, &dataset.main_table)
        .map_err(|error| AppendError::DataLoadError {
            message: error.to_string(),
        })?;

    apply_temporal_filter_lazy(
        frame,
        dataset.main_table.temporal_mode.clone(),
        context.run_period.as_deref(),
        context.as_of_date.as_deref(),
    )
}

pub fn execute_append(
    working_frame: &DataFrame,
    source_frame: &DataFrame,
    operation: &AppendOperation,
    context: &AppendExecutionContext,
) -> Result<AppendResult, AppendError> {
    execute_append_lazy(
        working_frame,
        source_frame.clone().lazy(),
        operation,
        context,
    )
}

fn execute_append_lazy(
    working_frame: &DataFrame,
    source_frame: LazyFrame,
    operation: &AppendOperation,
    context: &AppendExecutionContext,
) -> Result<AppendResult, AppendError> {
    let source_rows_loaded = count_rows_lazy(source_frame.clone())?;
    let mut transformed = apply_soft_delete_filter_lazy(source_frame)?;

    if let Some(selector) = &operation.source_selector {
        transformed = apply_source_selector_lazy(transformed, selector)?;
    }

    let source_rows_after_selector = count_rows_lazy(transformed.clone())?;

    if let Some(aggregation) = &operation.aggregation {
        validate_lazy_columns_exist(&transformed, &aggregation.group_by, "aggregation.group_by")?;
        for aggregation_item in &aggregation.aggregations {
            let parsed = parse_aggregation(&aggregation_item.expression)
                .map_err(|error| AppendError::AggregationError { message: error })?;
            if parsed.input_column != "*" {
                validate_lazy_columns_exist(
                    &transformed,
                    &[parsed.input_column.to_owned()],
                    "aggregation.expression",
                )?;
            }
        }

        transformed = apply_aggregation_lazy(transformed, aggregation)?;
    }

    let mut transformed = transformed
        .collect()
        .map_err(|error| AppendError::DataLoadError {
            message: error.to_string(),
        })?;

    transformed = align_appended_schema(&transformed, working_frame)?;

    if transformed.height() == 0 {
        return Ok(AppendResult {
            frame: working_frame.clone(),
            rows_appended: 0,
            source_rows_loaded,
            source_rows_after_selector,
        });
    }

    transformed = add_system_columns(
        &transformed,
        operation.source.dataset_id,
        context.operation_seq,
    )?;

    let mut frame = working_frame.clone();
    frame
        .vstack_mut(&transformed)
        .map_err(|error| AppendError::DataLoadError {
            message: error.to_string(),
        })?;

    Ok(AppendResult {
        rows_appended: transformed.height(),
        frame,
        source_rows_loaded,
        source_rows_after_selector,
    })
}

pub fn execute_append_operation<M: MetadataStore, D: DataLoader>(
    working_frame: &DataFrame,
    metadata_store: &M,
    data_loader: &D,
    project: &Project,
    operation: &AppendOperation,
    context: &AppendExecutionContext,
) -> Result<AppendResult, AppendError> {
    let source_frame =
        resolve_and_load_source(metadata_store, data_loader, project, operation, context)?;
    execute_append_lazy(working_frame, source_frame, operation, context)
}

pub fn apply_source_selector(
    frame: &DataFrame,
    selector: &Expression,
) -> Result<DataFrame, AppendError> {
    apply_source_selector_lazy(frame.clone().lazy(), selector)?
        .collect()
        .map_err(|error| AppendError::ExpressionParseError {
            expression: selector.source.clone(),
            error: error.to_string(),
        })
}

pub fn apply_source_selector_lazy(
    frame: LazyFrame,
    selector: &Expression,
) -> Result<LazyFrame, AppendError> {
    let filter_expr = parse_source_selector(&selector.source).map_err(|error| {
        AppendError::ExpressionParseError {
            expression: selector.source.clone(),
            error,
        }
    })?;

    let columns = extract_selector_columns(&selector.source).map_err(|error| {
        AppendError::ExpressionParseError {
            expression: selector.source.clone(),
            error,
        }
    })?;
    validate_lazy_columns_exist(&frame, &columns, "source_selector")?;

    Ok(frame.filter(filter_expr))
}

pub fn align_appended_schema(
    appended_frame: &DataFrame,
    working_frame: &DataFrame,
) -> Result<DataFrame, AppendError> {
    let appended_columns = appended_frame
        .get_column_names()
        .iter()
        .map(ToString::to_string)
        .collect::<BTreeSet<_>>();
    let working_columns = working_frame
        .get_column_names()
        .iter()
        .map(ToString::to_string)
        .collect::<Vec<_>>();
    let working_column_set = working_columns.iter().cloned().collect::<BTreeSet<_>>();

    let extra_columns = appended_columns
        .difference(&working_column_set)
        .cloned()
        .collect::<Vec<_>>();
    if !extra_columns.is_empty() {
        return Err(AppendError::ColumnMismatch { extra_columns });
    }

    let mut aligned = appended_frame.clone();
    for working_column in working_frame.get_columns() {
        let column_name = working_column.name().to_string();
        if !appended_columns.contains(&column_name) {
            let null_series = Series::full_null(
                working_column.name().clone(),
                aligned.height(),
                working_column.dtype(),
            );
            aligned
                .with_column(null_series)
                .map_err(|error| AppendError::DataLoadError {
                    message: error.to_string(),
                })?;
        }
    }

    let projected_columns = working_columns
        .iter()
        .map(String::as_str)
        .collect::<Vec<_>>();
    aligned
        .select(projected_columns)
        .map_err(|error| AppendError::DataLoadError {
            message: error.to_string(),
        })
}

pub fn add_system_columns(
    frame: &DataFrame,
    source_dataset_id: Uuid,
    operation_seq: u32,
) -> Result<DataFrame, AppendError> {
    let row_count = frame.height();
    let mut enriched = frame.clone();

    let row_ids = (0..row_count)
        .map(|_| Uuid::now_v7().to_string())
        .collect::<Vec<_>>();
    let source_ids = (0..row_count)
        .map(|_| source_dataset_id.to_string())
        .collect::<Vec<_>>();
    let operation_seqs = vec![operation_seq as i64; row_count];
    let deleted = vec![false; row_count];

    enriched
        .with_column(Series::new("_row_id".into(), row_ids))
        .map_err(|error| AppendError::DataLoadError {
            message: error.to_string(),
        })?;
    enriched
        .with_column(Series::new("_source_dataset".into(), source_ids))
        .map_err(|error| AppendError::DataLoadError {
            message: error.to_string(),
        })?;
    enriched
        .with_column(Series::new("_operation_seq".into(), operation_seqs))
        .map_err(|error| AppendError::DataLoadError {
            message: error.to_string(),
        })?;
    enriched
        .with_column(Series::new("_deleted".into(), deleted))
        .map_err(|error| AppendError::DataLoadError {
            message: error.to_string(),
        })?;

    Ok(enriched)
}

fn resolve_location(
    dataset: &Dataset,
    resolver: &Resolver,
    run_period: Option<&str>,
) -> Result<ResolvedLocation, AppendError> {
    let selected_rule = select_resolution_rule(dataset, resolver, run_period)?;

    let mut resolved = ResolvedLocation {
        datasource_id: String::new(),
        path: None,
        table: None,
        schema: None,
        period_identifier: run_period.map(str::to_owned),
        resolver_id: Some(resolver.id.clone()),
        rule_name: Some(selected_rule.name.clone()),
        catalog_response: None,
    };

    match &selected_rule.strategy {
        ResolutionStrategy::Path {
            datasource_id,
            path,
        } => {
            resolved.datasource_id = datasource_id.clone();
            resolved.path = Some(path.replace("{{table_name}}", &dataset.main_table.name));
        }
        ResolutionStrategy::Table {
            datasource_id,
            table,
            schema,
        } => {
            resolved.datasource_id = datasource_id.clone();
            resolved.table = Some(table.clone());
            resolved.schema = schema.clone();
        }
        ResolutionStrategy::Catalog {
            endpoint,
            method: _,
            auth: _,
            params: _,
            headers: _,
        } => {
            resolved.datasource_id = "catalog".to_owned();
            resolved.path = Some(endpoint.clone());
        }
    }

    Ok(resolved)
}

fn validate_lazy_columns_exist(
    frame: &LazyFrame,
    columns: &[String],
    context: &str,
) -> Result<(), AppendError> {
    let schema = frame
        .clone()
        .collect_schema()
        .map_err(|error| AppendError::DataLoadError {
            message: error.to_string(),
        })?;
    for column in columns {
        if schema.get(column.as_str()).is_none() {
            return Err(AppendError::ColumnNotFound {
                column: column.clone(),
                context: context.to_owned(),
            });
        }
    }
    Ok(())
}

fn get_source_dataset<M: MetadataStore>(
    metadata_store: &M,
    operation: &AppendOperation,
) -> Result<Dataset, AppendError> {
    metadata_store
        .get_dataset(
            &operation.source.dataset_id,
            operation.source.dataset_version,
        )
        .map_err(|error| {
            map_dataset_lookup_error(
                error,
                operation.source.dataset_id,
                operation.source.dataset_version,
            )
        })
}

fn map_dataset_lookup_error(
    error: MetadataStoreError,
    dataset_id: Uuid,
    version: Option<i32>,
) -> AppendError {
    match error {
        MetadataStoreError::DatasetNotFound { .. } => match version {
            Some(version) => AppendError::DatasetVersionNotFound {
                dataset_id,
                version,
            },
            None => AppendError::DatasetNotFound { dataset_id },
        },
        other => AppendError::MetadataAccessError {
            entity: "dataset".to_owned(),
            message: other.to_string(),
        },
    }
}

fn resolve_source_resolver<M: MetadataStore>(
    metadata_store: &M,
    project: &Project,
    dataset: &Dataset,
    operation: &AppendOperation,
) -> Result<Resolver, AppendError> {
    if let Some(resolver_id) = project.resolver_overrides.get(&operation.source.dataset_id) {
        return metadata_store
            .get_resolver(resolver_id)
            .map_err(|error| map_resolver_lookup_error(error, operation.source.dataset_id));
    }

    if let Some(resolver_id) = dataset.resolver_id.as_deref() {
        return metadata_store
            .get_resolver(resolver_id)
            .map_err(|error| map_resolver_lookup_error(error, operation.source.dataset_id));
    }

    metadata_store
        .get_default_resolver()
        .map_err(|error| map_resolver_lookup_error(error, operation.source.dataset_id))
}

fn map_resolver_lookup_error(error: MetadataStoreError, dataset_id: Uuid) -> AppendError {
    match error {
        MetadataStoreError::ResolverNotFound { .. } => AppendError::ResolverNotFound { dataset_id },
        other => AppendError::MetadataAccessError {
            entity: "resolver".to_owned(),
            message: other.to_string(),
        },
    }
}

fn apply_soft_delete_filter_lazy(frame: LazyFrame) -> Result<LazyFrame, AppendError> {
    let schema = frame
        .clone()
        .collect_schema()
        .map_err(|error| AppendError::DataLoadError {
            message: error.to_string(),
        })?;
    if schema.get("_deleted").is_none() {
        return Ok(frame);
    }

    Ok(frame.filter(col("_deleted").eq(lit(false)).or(col("_deleted").is_null())))
}

fn count_rows_lazy(frame: LazyFrame) -> Result<usize, AppendError> {
    frame
        .select([len()])
        .collect()
        .map_err(|error| AppendError::DataLoadError {
            message: error.to_string(),
        })
        .map(|count_frame| {
            count_frame
                .column("len")
                .ok()
                .and_then(|s| s.u32().ok())
                .and_then(|v| v.into_iter().next().flatten())
                .unwrap_or(0) as usize
        })
}

fn select_resolution_rule<'a>(
    dataset: &Dataset,
    resolver: &'a Resolver,
    run_period: Option<&str>,
) -> Result<&'a ResolutionRule, AppendError> {
    for rule in &resolver.rules {
        if rule_matches(rule, dataset, run_period)? {
            return Ok(rule);
        }
    }

    Err(AppendError::DataLoadError {
        message: format!("resolver '{}' has no applicable rules", resolver.id),
    })
}

fn rule_matches(
    rule: &ResolutionRule,
    dataset: &Dataset,
    run_period: Option<&str>,
) -> Result<bool, AppendError> {
    let Some(expression) = rule.when_expression.as_deref() else {
        return Ok(true);
    };

    let filter_expr =
        parse_source_selector(expression).map_err(|error| AppendError::ExpressionParseError {
            expression: expression.to_owned(),
            error,
        })?;

    let dataset_id = dataset.id.to_string();
    let context = DataFrame::new(vec![
        Series::new("run_period".into(), [run_period.unwrap_or("")]).into(),
        Series::new("dataset_name".into(), [dataset.main_table.name.as_str()]).into(),
        Series::new("dataset_id".into(), [dataset_id.as_str()]).into(),
        Series::new("dataset_version".into(), [dataset.version as i64]).into(),
        Series::new("data_level".into(), [rule.data_level.as_str()]).into(),
    ])
    .map_err(|error| AppendError::DataLoadError {
        message: error.to_string(),
    })?;

    let matched = context
        .lazy()
        .filter(filter_expr)
        .collect()
        .map_err(|error| AppendError::ExpressionParseError {
            expression: expression.to_owned(),
            error: error.to_string(),
        })?;
    Ok(matched.height() > 0)
}
