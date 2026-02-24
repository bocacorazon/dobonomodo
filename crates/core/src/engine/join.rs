//! Runtime join resolution and execution
//!
//! This module handles:
//! - Dataset version resolution (pinned vs latest active)
//! - Resolver precedence (Project override -> Dataset resolver_id -> system default)
//! - Period-aware filtering based on temporal_mode
//! - Left join execution with column suffixing

use std::collections::{BTreeMap, HashMap, HashSet};
use std::time::Instant;

use polars::prelude::*;
use thiserror::Error;
use uuid::Uuid;

use crate::dsl::compiler::{
    compile_assignment_expression, compile_join_condition_expression,
    extract_assignment_alias_column_references, CompileError, ExpressionSymbolTable,
    JoinComparisonOp, JoinConditionExpr, JoinConditionValue, JoinLogicalOp,
};
use crate::engine::io_traits::DataLoader;
use crate::engine::period_filter::apply_period_filter;
use crate::model::metadata_store::DatasetLookupError;
use crate::model::{
    Dataset, DatasetStatus, JoinDatasetSnapshot, OperationInstance, OperationKind, Period,
    ResolvedLocation, ResolverSnapshot, Run, RuntimeJoin, TemporalMode, UpdateArguments,
};
use crate::MetadataStore;

/// Source chosen during resolver precedence evaluation.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum ResolverSource {
    ProjectOverride,
    DatasetResolver,
    SystemDefault,
}

/// Metadata captured when a runtime join dataset is resolved.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct ResolvedJoinDataset {
    pub alias: String,
    pub dataset_id: Uuid,
    pub dataset_version: i32,
    pub resolver_id: String,
    pub resolver_source: ResolverSource,
    pub temporal_mode: Option<TemporalMode>,
    pub location: ResolvedLocation,
    pub join_columns: Vec<String>,
}

/// Errors that can occur during join resolution and execution.
#[derive(Debug, Error)]
pub enum JoinError {
    #[error("Dataset {0} not found in MetadataStore")]
    DatasetNotFound(Uuid),

    #[error("Dataset {0} is disabled and cannot be used in joins")]
    DatasetDisabled(Uuid),

    #[error("Unknown column '{0}' in join condition")]
    UnknownColumn(String),

    #[error("Metadata lookup failed for dataset {dataset_id} version {version:?}: {detail}")]
    MetadataLookupFailed {
        dataset_id: Uuid,
        version: Option<i32>,
        detail: String,
    },

    #[error("Resolver failed for join alias '{alias}' dataset {dataset_id}: {detail}")]
    ResolverFailed {
        dataset_id: Uuid,
        alias: String,
        detail: String,
    },

    #[error("DataLoader failed for join alias '{alias}' dataset {dataset_id}: {detail}")]
    LoadFailed {
        dataset_id: Uuid,
        alias: String,
        detail: String,
    },

    #[error("Period filter failed for join alias '{alias}' dataset {dataset_id}: {detail}")]
    PeriodFilterFailed {
        dataset_id: Uuid,
        alias: String,
        detail: String,
    },

    #[error("Dataset {dataset_id} version {version} not found")]
    VersionNotFound { dataset_id: Uuid, version: i32 },

    #[error("Join alias '{0}' is already used in this operation")]
    AliasNotUnique(String),

    #[error("Join alias '{alias}' conflicts with working dataset table name '{table}'")]
    AliasConflictsWithWorkingTable { alias: String, table: String },

    #[error("Join column '{0}' conflicts with existing column")]
    AliasColumnConflict(String),

    #[error("Unknown join alias '{0}' in assignment expression")]
    UnknownJoinAlias(String),

    #[error("Unknown join column '{alias}.{column}' in assignment expression")]
    UnknownJoinColumn { alias: String, column: String },

    #[error("Invalid join condition '{0}'")]
    InvalidJoinCondition(String),
}

fn resolver_source_name(source: ResolverSource) -> &'static str {
    match source {
        ResolverSource::ProjectOverride => "project_override",
        ResolverSource::DatasetResolver => "dataset_resolver",
        ResolverSource::SystemDefault => "system_default",
    }
}

/// Resolve the dataset version to use for a join.
pub fn resolve_dataset_version<M>(
    dataset_id: &Uuid,
    pinned_version: Option<i32>,
    metadata_store: &M,
) -> Result<(Dataset, i32), JoinError>
where
    M: MetadataStore,
{
    if let Some(version) = pinned_version {
        let dataset = metadata_store
            .get_dataset(dataset_id, Some(version))
            .map_err(|error| match error {
                DatasetLookupError::DatasetNotFound { .. } => {
                    JoinError::DatasetNotFound(*dataset_id)
                }
                DatasetLookupError::VersionNotFound { version, .. } => JoinError::VersionNotFound {
                    dataset_id: *dataset_id,
                    version,
                },
                DatasetLookupError::Other(detail) => JoinError::MetadataLookupFailed {
                    dataset_id: *dataset_id,
                    version: Some(version),
                    detail: detail.to_string(),
                },
            })?;

        if dataset.status != DatasetStatus::Active {
            return Err(JoinError::DatasetDisabled(*dataset_id));
        }
        return Ok((dataset, version));
    }

    let dataset = metadata_store
        .get_dataset(dataset_id, None)
        .map_err(|error| match error {
            DatasetLookupError::DatasetNotFound { .. } => JoinError::DatasetNotFound(*dataset_id),
            DatasetLookupError::VersionNotFound { version, .. } => {
                JoinError::MetadataLookupFailed {
                    dataset_id: *dataset_id,
                    version: Some(version),
                    detail:
                        "unexpected version lookup failure while resolving latest dataset version"
                            .to_string(),
                }
            }
            DatasetLookupError::Other(detail) => JoinError::MetadataLookupFailed {
                dataset_id: *dataset_id,
                version: None,
                detail: detail.to_string(),
            },
        })?;

    if dataset.status != DatasetStatus::Active {
        return Err(JoinError::DatasetDisabled(*dataset_id));
    }
    let version = dataset.version;
    Ok((dataset, version))
}

/// Determine resolver based on precedence rules.
pub fn resolve_resolver_id(
    dataset_id: &Uuid,
    dataset_resolver_id: Option<&str>,
    project_overrides: &BTreeMap<Uuid, String>,
    system_default: &str,
) -> String {
    resolve_resolver_with_source(
        dataset_id,
        dataset_resolver_id,
        project_overrides,
        system_default,
    )
    .0
}

/// Determine resolver and capture where it came from.
pub fn resolve_resolver_with_source(
    dataset_id: &Uuid,
    dataset_resolver_id: Option<&str>,
    project_overrides: &BTreeMap<Uuid, String>,
    system_default: &str,
) -> (String, ResolverSource) {
    if let Some(override_resolver) = project_overrides.get(dataset_id) {
        return (override_resolver.clone(), ResolverSource::ProjectOverride);
    }

    if let Some(resolver_id) = dataset_resolver_id {
        return (resolver_id.to_string(), ResolverSource::DatasetResolver);
    }

    (system_default.to_string(), ResolverSource::SystemDefault)
}

/// Validate operation-scoped join aliases.
pub fn validate_join_aliases(
    joins: &[RuntimeJoin],
    working_table_name: &str,
) -> Result<(), JoinError> {
    let mut seen = HashSet::new();
    for join in joins {
        if join.alias == working_table_name {
            return Err(JoinError::AliasConflictsWithWorkingTable {
                alias: join.alias.clone(),
                table: working_table_name.to_string(),
            });
        }
        if !seen.insert(join.alias.clone()) {
            return Err(JoinError::AliasNotUnique(join.alias.clone()));
        }
    }

    Ok(())
}

/// Resolve, load, and period-filter one RuntimeJoin dataset.
#[allow(clippy::too_many_arguments)]
pub fn resolve_and_load_join<F, R, L>(
    join: &RuntimeJoin,
    project_overrides: &BTreeMap<Uuid, String>,
    system_default_resolver: &str,
    period: &Period,
    metadata_store: &F,
    resolve_location: R,
    loader: &L,
    join_datasets: &mut Vec<JoinDatasetSnapshot>,
) -> Result<(LazyFrame, ResolvedJoinDataset), JoinError>
where
    F: MetadataStore,
    R: Fn(&Dataset, &str, &Period) -> Result<ResolvedLocation, String>,
    L: DataLoader,
{
    let (dataset, resolved_version) =
        resolve_dataset_version(&join.dataset_id, join.dataset_version, metadata_store)?;

    let (resolver_id, resolver_source) = resolve_resolver_with_source(
        &dataset.id,
        dataset.resolver_id.as_deref(),
        project_overrides,
        system_default_resolver,
    );

    let location = resolve_location(&dataset, &resolver_id, period).map_err(|detail| {
        JoinError::ResolverFailed {
            dataset_id: dataset.id,
            alias: join.alias.clone(),
            detail,
        }
    })?;

    let load_started = Instant::now();
    let mut join_lf = loader
        .load(&location, &dataset.main_table)
        .map_err(|error| JoinError::LoadFailed {
            dataset_id: dataset.id,
            alias: join.alias.clone(),
            detail: error.to_string(),
        })?;
    let load_elapsed = load_started.elapsed();
    let temporal_mode = dataset
        .main_table
        .temporal_mode
        .clone()
        .unwrap_or(TemporalMode::Period);
    let filter_started = Instant::now();
    join_lf = apply_period_filter(join_lf, &temporal_mode, period).map_err(|error| {
        JoinError::PeriodFilterFailed {
            dataset_id: dataset.id,
            alias: join.alias.clone(),
            detail: error.to_string(),
        }
    })?;
    let filter_elapsed = filter_started.elapsed();

    join_datasets.push(JoinDatasetSnapshot {
        alias: join.alias.clone(),
        dataset_id: dataset.id,
        dataset_version: resolved_version,
        resolver_source: resolver_source_name(resolver_source).to_string(),
    });

    let resolved = ResolvedJoinDataset {
        alias: join.alias.clone(),
        dataset_id: dataset.id,
        dataset_version: resolved_version,
        resolver_id,
        resolver_source,
        temporal_mode: Some(temporal_mode),
        location,
        join_columns: dataset
            .main_table
            .columns
            .iter()
            .map(|column| column.name.clone())
            .collect(),
    };

    eprintln!(
        "join resolve: dataset={} version={} resolver={} source={:?} temporal_mode={:?} period={} load_ms={} filter_ms={}",
        resolved.dataset_id,
        resolved.dataset_version,
        resolved.resolver_id,
        resolved.resolver_source,
        resolved.temporal_mode,
        period.identifier,
        load_elapsed.as_millis(),
        filter_elapsed.as_millis()
    );

    Ok((join_lf, resolved))
}

fn collect_join_reference_usage(
    expression: &JoinConditionExpr,
    join_alias: &str,
    has_current_join_reference: &mut bool,
    has_other_reference: &mut bool,
) {
    fn apply_reference(
        reference: &str,
        join_alias: &str,
        has_current_join_reference: &mut bool,
        has_other_reference: &mut bool,
    ) {
        if reference
            .split_once('.')
            .is_some_and(|(alias, _)| alias == join_alias)
        {
            *has_current_join_reference = true;
        } else {
            *has_other_reference = true;
        }
    }

    match expression {
        JoinConditionExpr::Comparison { left, right, .. } => {
            if let JoinConditionValue::Reference(reference) = left {
                apply_reference(
                    reference,
                    join_alias,
                    has_current_join_reference,
                    has_other_reference,
                );
            }
            if let JoinConditionValue::Reference(reference) = right {
                apply_reference(
                    reference,
                    join_alias,
                    has_current_join_reference,
                    has_other_reference,
                );
            }
        }
        JoinConditionExpr::Logical { left, right, .. } => {
            collect_join_reference_usage(
                left,
                join_alias,
                has_current_join_reference,
                has_other_reference,
            );
            collect_join_reference_usage(
                right,
                join_alias,
                has_current_join_reference,
                has_other_reference,
            );
        }
    }
}

fn split_and_conditions<'a>(
    expression: &'a JoinConditionExpr,
    conditions: &mut Vec<&'a JoinConditionExpr>,
) {
    if let JoinConditionExpr::Logical {
        left,
        op: JoinLogicalOp::And,
        right,
    } = expression
    {
        split_and_conditions(left, conditions);
        split_and_conditions(right, conditions);
    } else {
        conditions.push(expression);
    }
}

fn map_condition_reference(
    reference: &str,
    join_alias: &str,
    working_table_name: &str,
    available_columns: &HashSet<&str>,
    prior_join_columns: &HashSet<&str>,
    join_columns: &HashSet<&str>,
) -> Result<String, JoinError> {
    if let Some((alias, column)) = reference.split_once('.') {
        if alias == join_alias {
            if !join_columns.contains(column) {
                return Err(JoinError::UnknownColumn(column.to_string()));
            }
            return Ok(format!("{column}_{join_alias}"));
        }

        if alias == working_table_name {
            if prior_join_columns.contains(column) {
                return Err(JoinError::InvalidJoinCondition(reference.to_string()));
            }
            if !available_columns.contains(column) {
                return Err(JoinError::UnknownColumn(column.to_string()));
            }
            return Ok(column.to_string());
        }

        return Err(JoinError::InvalidJoinCondition(reference.to_string()));
    }

    if prior_join_columns.contains(reference) {
        Err(JoinError::InvalidJoinCondition(reference.to_string()))
    } else if available_columns.contains(reference) {
        Ok(reference.to_string())
    } else {
        Err(JoinError::UnknownColumn(reference.to_string()))
    }
}

fn compile_condition_value(
    value: &JoinConditionValue,
    join_alias: &str,
    working_table_name: &str,
    available_columns: &HashSet<&str>,
    prior_join_columns: &HashSet<&str>,
    join_columns: &HashSet<&str>,
) -> Result<Expr, JoinError> {
    match value {
        JoinConditionValue::Reference(reference) => {
            let mapped = map_condition_reference(
                reference,
                join_alias,
                working_table_name,
                available_columns,
                prior_join_columns,
                join_columns,
            )?;
            Ok(col(mapped.as_str()))
        }
        JoinConditionValue::StringLiteral(value) => Ok(lit(value.clone())),
        JoinConditionValue::BooleanLiteral(value) => Ok(lit(*value)),
        JoinConditionValue::NumberLiteral(value) => {
            if let Ok(integer) = value.parse::<i64>() {
                return Ok(lit(integer));
            }
            if let Ok(float) = value.parse::<f64>() {
                return Ok(lit(float));
            }
            Err(JoinError::InvalidJoinCondition(format!(
                "invalid numeric literal '{value}'"
            )))
        }
    }
}

fn compile_join_condition(
    expression: &JoinConditionExpr,
    join_alias: &str,
    working_table_name: &str,
    available_columns: &HashSet<&str>,
    prior_join_columns: &HashSet<&str>,
    join_columns: &HashSet<&str>,
) -> Result<Expr, JoinError> {
    match expression {
        JoinConditionExpr::Comparison { left, op, right } => {
            let left = compile_condition_value(
                left,
                join_alias,
                working_table_name,
                available_columns,
                prior_join_columns,
                join_columns,
            )?;
            let right = compile_condition_value(
                right,
                join_alias,
                working_table_name,
                available_columns,
                prior_join_columns,
                join_columns,
            )?;

            let compiled = match op {
                JoinComparisonOp::Eq => left.eq(right),
                JoinComparisonOp::NotEq => left.neq(right),
                JoinComparisonOp::Lt => left.lt(right),
                JoinComparisonOp::Lte => left.lt_eq(right),
                JoinComparisonOp::Gt => left.gt(right),
                JoinComparisonOp::Gte => left.gt_eq(right),
            };
            Ok(compiled)
        }
        JoinConditionExpr::Logical { left, op, right } => {
            let left = compile_join_condition(
                left,
                join_alias,
                working_table_name,
                available_columns,
                prior_join_columns,
                join_columns,
            )?;
            let right = compile_join_condition(
                right,
                join_alias,
                working_table_name,
                available_columns,
                prior_join_columns,
                join_columns,
            )?;

            let compiled = match op {
                JoinLogicalOp::And => left.and(right),
                JoinLogicalOp::Or => left.or(right),
            };
            Ok(compiled)
        }
    }
}

fn map_non_join_key_reference(
    reference: &str,
    join_alias: &str,
    working_table_name: &str,
    available_columns: &HashSet<&str>,
    prior_join_columns: &HashSet<&str>,
) -> Result<String, JoinError> {
    if let Some((alias, column)) = reference.split_once('.') {
        if alias == join_alias {
            return Err(JoinError::InvalidJoinCondition(reference.to_string()));
        }
        if alias == working_table_name {
            if prior_join_columns.contains(column) {
                return Err(JoinError::InvalidJoinCondition(reference.to_string()));
            }
            if !available_columns.contains(column) {
                return Err(JoinError::UnknownColumn(column.to_string()));
            }
            return Ok(column.to_string());
        }

        return Err(JoinError::InvalidJoinCondition(reference.to_string()));
    }

    if prior_join_columns.contains(reference) {
        Err(JoinError::InvalidJoinCondition(reference.to_string()))
    } else if available_columns.contains(reference) {
        Ok(reference.to_string())
    } else {
        Err(JoinError::UnknownColumn(reference.to_string()))
    }
}

fn map_join_key_reference(
    reference: &str,
    join_alias: &str,
    join_columns: &HashSet<&str>,
) -> Result<String, JoinError> {
    let Some((alias, column)) = reference.split_once('.') else {
        return Err(JoinError::InvalidJoinCondition(reference.to_string()));
    };
    if alias != join_alias {
        return Err(JoinError::InvalidJoinCondition(reference.to_string()));
    }
    if !join_columns.contains(column) {
        return Err(JoinError::UnknownColumn(column.to_string()));
    }
    Ok(column.to_string())
}

fn build_join_key_pair(
    condition: &JoinConditionExpr,
    join_alias: &str,
    working_table_name: &str,
    available_columns: &HashSet<&str>,
    prior_join_columns: &HashSet<&str>,
    join_columns: &HashSet<&str>,
) -> Result<(String, String), JoinError> {
    let JoinConditionExpr::Comparison {
        left,
        op: JoinComparisonOp::Eq,
        right,
    } = condition
    else {
        return Err(JoinError::InvalidJoinCondition(format!("{condition:?}")));
    };

    let (
        JoinConditionValue::Reference(left_reference),
        JoinConditionValue::Reference(right_reference),
    ) = (left, right)
    else {
        return Err(JoinError::InvalidJoinCondition(format!("{condition:?}")));
    };

    let left_is_join = left_reference
        .split_once('.')
        .is_some_and(|(alias, _)| alias == join_alias);
    let right_is_join = right_reference
        .split_once('.')
        .is_some_and(|(alias, _)| alias == join_alias);

    match (left_is_join, right_is_join) {
        (true, false) => Ok((
            map_non_join_key_reference(
                right_reference,
                join_alias,
                working_table_name,
                available_columns,
                prior_join_columns,
            )?,
            map_join_key_reference(left_reference, join_alias, join_columns)?,
        )),
        (false, true) => Ok((
            map_non_join_key_reference(
                left_reference,
                join_alias,
                working_table_name,
                available_columns,
                prior_join_columns,
            )?,
            map_join_key_reference(right_reference, join_alias, join_columns)?,
        )),
        _ => Err(JoinError::InvalidJoinCondition(format!("{condition:?}"))),
    }
}

/// Apply RuntimeJoins sequentially to a working LazyFrame.
pub fn apply_runtime_joins<F>(
    mut working_lf: LazyFrame,
    joins: &[RuntimeJoin],
    working_table_name: &str,
    working_columns: &[String],
    mut load_join: F,
) -> Result<LazyFrame, JoinError>
where
    F: FnMut(&RuntimeJoin) -> Result<(LazyFrame, Vec<String>), JoinError>,
{
    validate_join_aliases(joins, working_table_name)?;

    let base_available_columns = working_columns
        .iter()
        .map(String::as_str)
        .collect::<HashSet<&str>>();
    let mut prior_join_columns = HashSet::new();

    for join in joins {
        let (join_lf, join_columns) = load_join(join)?;
        let parsed_condition =
            compile_join_condition_expression(&join.on.source).map_err(|error| {
                JoinError::InvalidJoinCondition(format!(
                    "failed to compile '{}': {error}",
                    join.on.source
                ))
            })?;
        let mut conjuncts = Vec::new();
        split_and_conditions(&parsed_condition, &mut conjuncts);
        let mut join_predicates = Vec::new();
        let mut join_side_filters = Vec::new();
        for condition in conjuncts {
            let mut has_current_join_reference = false;
            let mut has_other_reference = false;
            collect_join_reference_usage(
                condition,
                join.alias.as_str(),
                &mut has_current_join_reference,
                &mut has_other_reference,
            );
            match (has_current_join_reference, has_other_reference) {
                (true, true) => join_predicates.push(condition),
                (true, false) => join_side_filters.push(condition),
                (false, true) | (false, false) => {
                    return Err(JoinError::InvalidJoinCondition(format!(
                        "unsupported runtime join predicate in '{}': each AND term must reference the join alias '{}'",
                        join.on.source, join.alias
                    )));
                }
            }
        }

        let renamed_columns = join_columns
            .iter()
            .map(|column| format!("{column}_{}", join.alias))
            .collect::<Vec<_>>();

        for renamed in &renamed_columns {
            if base_available_columns.contains(renamed.as_str())
                || prior_join_columns.contains(renamed.as_str())
            {
                return Err(JoinError::AliasColumnConflict(renamed.clone()));
            }
        }

        let filtered_join_lf = join_lf.rename(
            join_columns.iter().map(|column| column.as_str()),
            renamed_columns.iter().map(|column| column.as_str()),
            true,
        );

        let prior_join_set = prior_join_columns
            .iter()
            .map(String::as_str)
            .collect::<HashSet<&str>>();
        let join_set = join_columns
            .iter()
            .map(String::as_str)
            .collect::<HashSet<&str>>();
        let join_filter_expression = join_side_filters
            .into_iter()
            .map(|condition| {
                compile_join_condition(
                    condition,
                    join.alias.as_str(),
                    working_table_name,
                    &base_available_columns,
                    &prior_join_set,
                    &join_set,
                )
            })
            .collect::<Result<Vec<_>, _>>()?
            .into_iter()
            .reduce(|left, right| left.and(right));

        let filtered_join_lf = if let Some(filter_expression) = join_filter_expression {
            filtered_join_lf.filter(filter_expression)
        } else {
            filtered_join_lf
        };

        if join_predicates.is_empty() {
            return Err(JoinError::InvalidJoinCondition(format!(
                "unsupported runtime join predicate in '{}': at least one equality predicate between working and join columns is required",
                join.on.source
            )));
        }

        let key_pairs = join_predicates
            .iter()
            .map(|condition| {
                build_join_key_pair(
                    condition,
                    join.alias.as_str(),
                    working_table_name,
                    &base_available_columns,
                    &prior_join_set,
                    &join_set,
                )
            })
            .collect::<Result<Vec<_>, _>>()
            .map_err(|error| {
                if matches!(error, JoinError::UnknownColumn(_)) {
                    return error;
                }
                JoinError::InvalidJoinCondition(format!(
                    "unsupported runtime join predicate in '{}': only AND-connected equality predicates are supported",
                    join.on.source
                ))
            })?;

        let join_started = Instant::now();
        let left_expressions = key_pairs
            .iter()
            .map(|(left_key, _)| col(left_key.as_str()))
            .collect::<Vec<_>>();
        let renamed_lookup = join_columns
            .iter()
            .zip(renamed_columns.iter())
            .map(|(original, renamed)| (original.as_str(), renamed.as_str()))
            .collect::<HashMap<_, _>>();
        let right_expressions = key_pairs
            .iter()
            .map(|(_, right_key)| {
                renamed_lookup
                    .get(right_key.as_str())
                    .map_or_else(|| col(right_key.as_str()), |renamed| col(*renamed))
            })
            .collect::<Vec<_>>();
        working_lf = working_lf.join(
            filtered_join_lf,
            left_expressions,
            right_expressions,
            JoinArgs::new(JoinType::Left),
        );
        let join_elapsed = join_started.elapsed();
        eprintln!(
            "join execute: alias={} join_ms={}",
            join.alias,
            join_elapsed.as_millis()
        );

        prior_join_columns.extend(renamed_columns.into_iter());
    }

    Ok(working_lf)
}

#[derive(Debug, Clone, PartialEq, Eq)]
struct AssignmentDefinition {
    column: String,
    expression: String,
}

#[derive(Debug, Clone, PartialEq, Eq)]
struct CompiledAssignment {
    column: String,
    expression: String,
}

fn extract_assignment_definitions(
    assignments: &[serde_json::Value],
) -> Result<Vec<AssignmentDefinition>, JoinError> {
    assignments
        .iter()
        .enumerate()
        .map(|(index, assignment)| {
            let Some(column) = assignment.get("column").and_then(serde_json::Value::as_str) else {
                return Err(JoinError::InvalidJoinCondition(format!(
                    "invalid assignment at index {index}: missing 'column'"
                )));
            };
            if column.trim().is_empty() {
                return Err(JoinError::InvalidJoinCondition(format!(
                    "invalid assignment at index {index}: 'column' must be non-empty"
                )));
            }

            let Some(expression_value) = assignment.get("expression") else {
                return Err(JoinError::InvalidJoinCondition(format!(
                    "invalid assignment at index {index}: missing 'expression'"
                )));
            };

            let expression = if let Some(expression) = expression_value.as_str() {
                expression.to_string()
            } else {
                let Some(source) = expression_value
                    .get("source")
                    .and_then(serde_json::Value::as_str)
                else {
                    return Err(JoinError::InvalidJoinCondition(format!(
                        "invalid assignment at index {index}: 'expression' must be a string or object with 'source'"
                    )));
                };
                source.to_string()
            };

            if expression.trim().is_empty() {
                return Err(JoinError::InvalidJoinCondition(format!(
                    "invalid assignment at index {index}: 'expression' must be non-empty"
                )));
            }

            Ok(AssignmentDefinition {
                column: column.to_string(),
                expression,
            })
        })
        .collect()
}

fn extract_assignment_columns(assignments: &[serde_json::Value]) -> Result<Vec<String>, JoinError> {
    extract_assignment_definitions(assignments).map(|definitions| {
        definitions
            .into_iter()
            .map(|definition| definition.column)
            .collect()
    })
}

fn map_assignment_compile_error(error: CompileError) -> JoinError {
    match error {
        CompileError::UnknownAlias(alias) => JoinError::UnknownJoinAlias(alias),
        CompileError::UnknownAliasedColumn { alias, column } => {
            JoinError::UnknownJoinColumn { alias, column }
        }
        CompileError::UnknownColumn(column) => JoinError::UnknownColumn(column),
        CompileError::InvalidExpression(detail) => {
            JoinError::InvalidJoinCondition(format!("invalid assignment expression: {detail}"))
        }
    }
}

fn validate_and_compile_update_assignments(
    assignments: &[serde_json::Value],
    working_columns: &[String],
    resolved_joins: &[ResolvedJoinDataset],
) -> Result<Vec<CompiledAssignment>, JoinError> {
    let definitions = extract_assignment_definitions(assignments)?;
    if definitions.is_empty() {
        return Ok(Vec::new());
    }

    let mut join_alias_columns = HashMap::new();
    let mut symbols = ExpressionSymbolTable::with_working_columns(working_columns.iter().cloned());

    for resolved in resolved_joins {
        join_alias_columns.insert(resolved.alias.clone(), resolved.join_columns.clone());
        symbols.add_join_alias(
            resolved.alias.as_str(),
            resolved.join_columns.iter().cloned(),
        );
    }

    let mut compiled_assignments = Vec::with_capacity(definitions.len());
    for definition in definitions {
        validate_assignment_alias_references(
            std::slice::from_ref(&definition.expression),
            &join_alias_columns,
        )?;
        let expression = compile_assignment_expression(&definition.expression, &symbols)
            .map_err(map_assignment_compile_error)?;
        symbols.add_working_column(definition.column.clone());
        compiled_assignments.push(CompiledAssignment {
            column: definition.column,
            expression,
        });
    }

    Ok(compiled_assignments)
}

#[derive(Debug, Clone, PartialEq)]
enum AssignmentTokenKind {
    Identifier(String),
    NumberLiteral(String),
    StringLiteral(String),
    BooleanLiteral(bool),
    LParen,
    RParen,
    Comma,
    Plus,
    Minus,
    Star,
    Slash,
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
struct AssignmentToken {
    kind: AssignmentTokenKind,
}

fn tokenize_assignment_expression(expression: &str) -> Result<Vec<AssignmentToken>, JoinError> {
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

        if ch == '\'' {
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
                    return Err(JoinError::InvalidJoinCondition(
                        "invalid assignment expression: unterminated escape sequence".to_string(),
                    ));
                }
                if chars[index] == '\'' {
                    index += 1;
                    terminated = true;
                    break;
                }
                literal.push(chars[index]);
                index += 1;
            }

            if !terminated {
                return Err(JoinError::InvalidJoinCondition(
                    "invalid assignment expression: unterminated string literal".to_string(),
                ));
            }

            tokens.push(AssignmentToken {
                kind: AssignmentTokenKind::StringLiteral(literal),
            });
            continue;
        }

        if ch == '"' {
            index += 1;
            let mut identifier = String::new();
            let mut terminated = false;
            while index < chars.len() {
                if chars[index] == '\\' {
                    if let Some(escaped) = chars.get(index + 1) {
                        identifier.push(*escaped);
                        index += 2;
                        continue;
                    }
                    return Err(JoinError::InvalidJoinCondition(
                        "invalid assignment expression: unterminated escape sequence".to_string(),
                    ));
                }
                if chars[index] == '"' {
                    index += 1;
                    terminated = true;
                    break;
                }
                identifier.push(chars[index]);
                index += 1;
            }

            if !terminated {
                return Err(JoinError::InvalidJoinCondition(
                    "invalid assignment expression: unterminated quoted identifier".to_string(),
                ));
            }

            tokens.push(AssignmentToken {
                kind: AssignmentTokenKind::Identifier(identifier),
            });
            continue;
        }

        if ch.is_ascii_alphabetic() || ch == '_' {
            let start = index;
            index += 1;
            while index < chars.len()
                && (chars[index].is_ascii_alphanumeric() || chars[index] == '_')
            {
                index += 1;
            }
            let start_offset = byte_offset(&indexed_chars, start, expression_len);
            let end_offset = byte_offset(&indexed_chars, index, expression_len);
            let identifier = expression[start_offset..end_offset].to_string();
            let lower = identifier.to_ascii_lowercase();

            let kind = match lower.as_str() {
                "and" => AssignmentTokenKind::And,
                "or" => AssignmentTokenKind::Or,
                "true" => AssignmentTokenKind::BooleanLiteral(true),
                "false" => AssignmentTokenKind::BooleanLiteral(false),
                _ => AssignmentTokenKind::Identifier(identifier),
            };
            tokens.push(AssignmentToken { kind });
            continue;
        }

        if ch.is_ascii_digit()
            || (ch == '.'
                && chars
                    .get(index + 1)
                    .is_some_and(|candidate| candidate.is_ascii_digit()))
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
            tokens.push(AssignmentToken {
                kind: AssignmentTokenKind::NumberLiteral(
                    expression[start_offset..end_offset].to_string(),
                ),
            });
            continue;
        }

        let kind = match ch {
            '(' => Some(AssignmentTokenKind::LParen),
            ')' => Some(AssignmentTokenKind::RParen),
            ',' => Some(AssignmentTokenKind::Comma),
            '+' => Some(AssignmentTokenKind::Plus),
            '-' => Some(AssignmentTokenKind::Minus),
            '*' => Some(AssignmentTokenKind::Star),
            '/' => Some(AssignmentTokenKind::Slash),
            '=' => {
                if chars.get(index + 1) == Some(&'=') {
                    index += 1;
                }
                Some(AssignmentTokenKind::Eq)
            }
            '!' => {
                if chars.get(index + 1) == Some(&'=') {
                    index += 1;
                    Some(AssignmentTokenKind::NotEq)
                } else {
                    None
                }
            }
            '<' => {
                if chars.get(index + 1) == Some(&'=') {
                    index += 1;
                    Some(AssignmentTokenKind::Lte)
                } else {
                    Some(AssignmentTokenKind::Lt)
                }
            }
            '>' => {
                if chars.get(index + 1) == Some(&'=') {
                    index += 1;
                    Some(AssignmentTokenKind::Gte)
                } else {
                    Some(AssignmentTokenKind::Gt)
                }
            }
            _ => None,
        };

        let Some(kind) = kind else {
            return Err(JoinError::InvalidJoinCondition(format!(
                "invalid assignment expression: unexpected character '{ch}'"
            )));
        };
        tokens.push(AssignmentToken { kind });
        index += 1;
    }

    Ok(tokens)
}

fn byte_offset(chars: &[(usize, char)], index: usize, expression_len: usize) -> usize {
    chars
        .get(index)
        .map(|(offset, _)| *offset)
        .unwrap_or(expression_len)
}

struct AssignmentParser {
    tokens: Vec<AssignmentToken>,
    cursor: usize,
}

impl AssignmentParser {
    fn new(tokens: Vec<AssignmentToken>) -> Self {
        Self { tokens, cursor: 0 }
    }

    fn parse(mut self) -> Result<Expr, JoinError> {
        let expression = self.parse_or_expression()?;
        if self.cursor != self.tokens.len() {
            return Err(JoinError::InvalidJoinCondition(
                "invalid assignment expression: unexpected trailing tokens".to_string(),
            ));
        }
        Ok(expression)
    }

    fn parse_or_expression(&mut self) -> Result<Expr, JoinError> {
        let mut expression = self.parse_and_expression()?;
        while self.consume_if(|kind| matches!(kind, AssignmentTokenKind::Or)) {
            let right = self.parse_and_expression()?;
            expression = expression.or(right);
        }
        Ok(expression)
    }

    fn parse_and_expression(&mut self) -> Result<Expr, JoinError> {
        let mut expression = self.parse_comparison_expression()?;
        while self.consume_if(|kind| matches!(kind, AssignmentTokenKind::And)) {
            let right = self.parse_comparison_expression()?;
            expression = expression.and(right);
        }
        Ok(expression)
    }

    fn parse_comparison_expression(&mut self) -> Result<Expr, JoinError> {
        let left = self.parse_additive_expression()?;
        let Some(operator) = self.peek().cloned() else {
            return Ok(left);
        };

        match operator {
            AssignmentTokenKind::Eq
            | AssignmentTokenKind::NotEq
            | AssignmentTokenKind::Lt
            | AssignmentTokenKind::Lte
            | AssignmentTokenKind::Gt
            | AssignmentTokenKind::Gte => {
                self.cursor += 1;
                let right = self.parse_additive_expression()?;
                let expression = match operator {
                    AssignmentTokenKind::Eq => left.eq(right),
                    AssignmentTokenKind::NotEq => left.neq(right),
                    AssignmentTokenKind::Lt => left.lt(right),
                    AssignmentTokenKind::Lte => left.lt_eq(right),
                    AssignmentTokenKind::Gt => left.gt(right),
                    AssignmentTokenKind::Gte => left.gt_eq(right),
                    _ => unreachable!(),
                };
                Ok(expression)
            }
            _ => Ok(left),
        }
    }

    fn parse_additive_expression(&mut self) -> Result<Expr, JoinError> {
        let mut expression = self.parse_multiplicative_expression()?;

        loop {
            if self.consume_if(|kind| matches!(kind, AssignmentTokenKind::Plus)) {
                let right = self.parse_multiplicative_expression()?;
                expression = expression + right;
                continue;
            }
            if self.consume_if(|kind| matches!(kind, AssignmentTokenKind::Minus)) {
                let right = self.parse_multiplicative_expression()?;
                expression = expression - right;
                continue;
            }
            break;
        }

        Ok(expression)
    }

    fn parse_multiplicative_expression(&mut self) -> Result<Expr, JoinError> {
        let mut expression = self.parse_unary_expression()?;

        loop {
            if self.consume_if(|kind| matches!(kind, AssignmentTokenKind::Star)) {
                let right = self.parse_unary_expression()?;
                expression = expression * right;
                continue;
            }
            if self.consume_if(|kind| matches!(kind, AssignmentTokenKind::Slash)) {
                let right = self.parse_unary_expression()?;
                expression = expression / right;
                continue;
            }
            break;
        }

        Ok(expression)
    }

    fn parse_unary_expression(&mut self) -> Result<Expr, JoinError> {
        if self.consume_if(|kind| matches!(kind, AssignmentTokenKind::Minus)) {
            use std::ops::Neg;
            return Ok(self.parse_unary_expression()?.neg());
        }
        self.parse_primary_expression()
    }

    fn parse_primary_expression(&mut self) -> Result<Expr, JoinError> {
        if self.consume_if(|kind| matches!(kind, AssignmentTokenKind::LParen)) {
            let expression = self.parse_or_expression()?;
            if !self.consume_if(|kind| matches!(kind, AssignmentTokenKind::RParen)) {
                return Err(JoinError::InvalidJoinCondition(
                    "invalid assignment expression: missing closing ')'".to_string(),
                ));
            }
            return Ok(expression);
        }

        let token = self.take_current().ok_or_else(|| {
            JoinError::InvalidJoinCondition(
                "invalid assignment expression: unexpected end of expression".to_string(),
            )
        })?;

        match token.kind {
            AssignmentTokenKind::Identifier(identifier) => {
                if self.consume_if(|kind| matches!(kind, AssignmentTokenKind::LParen)) {
                    return self.parse_function_call(identifier);
                }
                Ok(col(identifier.as_str()))
            }
            AssignmentTokenKind::NumberLiteral(value) => {
                if let Ok(integer) = value.parse::<i64>() {
                    return Ok(lit(integer));
                }
                value.parse::<f64>().map(lit).map_err(|_| {
                    JoinError::InvalidJoinCondition(format!(
                        "invalid assignment expression: invalid numeric literal '{value}'"
                    ))
                })
            }
            AssignmentTokenKind::StringLiteral(value) => Ok(lit(value)),
            AssignmentTokenKind::BooleanLiteral(value) => Ok(lit(value)),
            _ => Err(JoinError::InvalidJoinCondition(
                "invalid assignment expression: invalid value".to_string(),
            )),
        }
    }

    fn parse_function_call(&mut self, name: String) -> Result<Expr, JoinError> {
        let mut args = Vec::new();
        if !self.consume_if(|kind| matches!(kind, AssignmentTokenKind::RParen)) {
            loop {
                args.push(self.parse_or_expression()?);
                if self.consume_if(|kind| matches!(kind, AssignmentTokenKind::Comma)) {
                    continue;
                }
                if self.consume_if(|kind| matches!(kind, AssignmentTokenKind::RParen)) {
                    break;
                }
                return Err(JoinError::InvalidJoinCondition(
                    "invalid assignment expression: expected ',' or ')' in function call"
                        .to_string(),
                ));
            }
        }

        if name.eq_ignore_ascii_case("if") {
            if args.len() != 3 {
                return Err(JoinError::InvalidJoinCondition(
                    "invalid assignment expression: IF expects exactly 3 arguments".to_string(),
                ));
            }
            let condition = args.remove(0);
            let when_true = args.remove(0);
            let when_false = args.remove(0);
            return Ok(when(condition).then(when_true).otherwise(when_false));
        }

        Err(JoinError::InvalidJoinCondition(format!(
            "invalid assignment expression: unsupported function '{name}'"
        )))
    }

    fn consume_if(&mut self, predicate: impl FnOnce(&AssignmentTokenKind) -> bool) -> bool {
        let Some(token) = self.tokens.get(self.cursor) else {
            return false;
        };
        if predicate(&token.kind) {
            self.cursor += 1;
            return true;
        }
        false
    }

    fn peek(&self) -> Option<&AssignmentTokenKind> {
        self.tokens.get(self.cursor).map(|token| &token.kind)
    }

    fn take_current(&mut self) -> Option<AssignmentToken> {
        let token = self.tokens.get(self.cursor).cloned()?;
        self.cursor += 1;
        Some(token)
    }
}

fn compile_assignment_lazy_expr(expression: &str) -> Result<Expr, JoinError> {
    let tokens = tokenize_assignment_expression(expression)?;
    AssignmentParser::new(tokens).parse()
}

fn apply_update_assignments(
    mut frame: LazyFrame,
    assignments: &[CompiledAssignment],
) -> Result<LazyFrame, JoinError> {
    for assignment in assignments {
        let expression = compile_assignment_lazy_expr(&assignment.expression)?;
        frame = frame.with_columns([expression.alias(assignment.column.as_str())]);
    }
    Ok(frame)
}

fn project_operation_columns(
    frame: LazyFrame,
    working_columns: &[String],
    assignments: &[CompiledAssignment],
) -> LazyFrame {
    let mut projected_columns = Vec::with_capacity(working_columns.len() + assignments.len());
    let mut seen = HashSet::new();

    for column in working_columns {
        if seen.insert(column.clone()) {
            projected_columns.push(column.clone());
        }
    }
    for assignment in assignments {
        if seen.insert(assignment.column.clone()) {
            projected_columns.push(assignment.column.clone());
        }
    }

    frame.select(
        projected_columns
            .iter()
            .map(|column| col(column.as_str()))
            .collect::<Vec<_>>(),
    )
}

#[allow(clippy::too_many_arguments)]
pub fn apply_update_runtime_joins<M, R, L>(
    run: &mut Run,
    arguments: &UpdateArguments,
    working_lf: LazyFrame,
    working_table_name: &str,
    working_columns: &[String],
    project_overrides: &BTreeMap<Uuid, String>,
    system_default_resolver: &str,
    period: &Period,
    metadata_store: &M,
    resolve_location: R,
    loader: &L,
) -> Result<LazyFrame, JoinError>
where
    M: MetadataStore,
    R: Fn(&Dataset, &str, &Period) -> Result<ResolvedLocation, String>,
    L: DataLoader,
{
    let mut join_datasets = Vec::new();
    let mut resolved_joins = Vec::new();

    let joined = apply_runtime_joins(
        working_lf,
        &arguments.joins,
        working_table_name,
        working_columns,
        |join| {
            let (join_lf, resolved) = resolve_and_load_join(
                join,
                project_overrides,
                system_default_resolver,
                period,
                metadata_store,
                &resolve_location,
                loader,
                &mut join_datasets,
            )?;
            let join_columns = resolved.join_columns.clone();
            resolved_joins.push(resolved);
            Ok((join_lf, join_columns))
        },
    )?;

    let compiled_assignments = validate_and_compile_update_assignments(
        &arguments.assignments,
        working_columns,
        &resolved_joins,
    )?;
    let projected = project_operation_columns(
        apply_update_assignments(joined, &compiled_assignments)?,
        working_columns,
        &compiled_assignments,
    );

    for (resolved, join_dataset) in resolved_joins.into_iter().zip(join_datasets.into_iter()) {
        let resolver = metadata_store
            .get_resolver(&resolved.resolver_id)
            .map_err(|error| JoinError::MetadataLookupFailed {
                dataset_id: resolved.dataset_id,
                version: Some(resolved.dataset_version),
                detail: format!(
                    "resolver lookup failed for '{}': {error}",
                    resolved.resolver_id
                ),
            })?;

        run.project_snapshot
            .resolver_snapshots
            .push(ResolverSnapshot {
                dataset_id: resolved.dataset_id,
                resolver_id: resolved.resolver_id,
                resolver_version: resolver.version,
                join_datasets: vec![join_dataset],
            });
    }

    Ok(projected)
}

#[allow(clippy::too_many_arguments)]
pub fn apply_update_operation_runtime_joins<M, R, L>(
    run: &mut Run,
    operation: &OperationInstance,
    working_lf: LazyFrame,
    working_table_name: &str,
    working_columns: &[String],
    project_overrides: &BTreeMap<Uuid, String>,
    system_default_resolver: &str,
    period: &Period,
    metadata_store: &M,
    resolve_location: R,
    loader: &L,
) -> Result<LazyFrame, JoinError>
where
    M: MetadataStore,
    R: Fn(&Dataset, &str, &Period) -> Result<ResolvedLocation, String>,
    L: DataLoader,
{
    if operation.kind != OperationKind::Update {
        return Ok(working_lf);
    }

    let arguments = serde_json::from_value::<UpdateArguments>(operation.parameters.clone())
        .map_err(|error| {
            JoinError::InvalidJoinCondition(format!("invalid update arguments: {error}"))
        })?;

    apply_update_runtime_joins(
        run,
        &arguments,
        working_lf,
        working_table_name,
        working_columns,
        project_overrides,
        system_default_resolver,
        period,
        metadata_store,
        resolve_location,
        loader,
    )
}

#[allow(clippy::too_many_arguments)]
pub fn apply_runtime_joins_for_operation_pipeline<M, R, L>(
    run: &mut Run,
    operations: &[OperationInstance],
    mut working_lf: LazyFrame,
    working_table_name: &str,
    working_columns: &[String],
    project_overrides: &BTreeMap<Uuid, String>,
    system_default_resolver: &str,
    period: &Period,
    metadata_store: &M,
    resolve_location: R,
    loader: &L,
) -> Result<LazyFrame, JoinError>
where
    M: MetadataStore,
    R: Fn(&Dataset, &str, &Period) -> Result<ResolvedLocation, String>,
    L: DataLoader,
{
    let mut available_columns = working_columns.to_vec();
    for operation in operations {
        working_lf = apply_update_operation_runtime_joins(
            run,
            operation,
            working_lf,
            working_table_name,
            &available_columns,
            project_overrides,
            system_default_resolver,
            period,
            metadata_store,
            &resolve_location,
            loader,
        )?;

        if operation.kind == OperationKind::Update {
            let arguments = serde_json::from_value::<UpdateArguments>(operation.parameters.clone())
                .map_err(|error| {
                    JoinError::InvalidJoinCondition(format!("invalid update arguments: {error}"))
                })?;
            for column in extract_assignment_columns(&arguments.assignments)? {
                if !available_columns
                    .iter()
                    .any(|candidate| candidate == &column)
                {
                    available_columns.push(column);
                }
            }
        }
    }

    Ok(working_lf)
}

/// Validate alias.column references used in assignment expressions.
pub fn validate_assignment_alias_references(
    expressions: &[String],
    join_alias_columns: &HashMap<String, Vec<String>>,
) -> Result<(), JoinError> {
    for expression in expressions {
        for (alias, column) in extract_assignment_alias_column_references(expression) {
            let Some(columns) = join_alias_columns.get(&alias) else {
                return Err(JoinError::UnknownJoinAlias(alias));
            };

            if !columns.iter().any(|candidate| candidate == &column) {
                return Err(JoinError::UnknownJoinColumn { alias, column });
            }
        }
    }

    Ok(())
}

#[cfg(test)]
mod tests {
    use anyhow::{anyhow, Result};

    use super::*;

    fn make_dataset(id: Uuid, version: i32, status: DatasetStatus) -> Dataset {
        use crate::model::{ColumnDef, ColumnType, TableRef};

        Dataset {
            id,
            name: "test".to_string(),
            description: None,
            owner: "test".to_string(),
            version,
            status,
            resolver_id: Some("dataset-resolver".to_string()),
            main_table: TableRef {
                name: "main".to_string(),
                temporal_mode: Some(TemporalMode::Period),
                columns: vec![ColumnDef {
                    name: "id".to_string(),
                    column_type: ColumnType::Integer,
                    nullable: Some(false),
                    description: None,
                }],
            },
            lookups: vec![],
            natural_key_columns: vec![],
            created_at: None,
            updated_at: None,
        }
    }

    struct SingleDatasetStore {
        dataset: Option<Dataset>,
    }

    impl MetadataStore for SingleDatasetStore {
        fn get_dataset(
            &self,
            id: &Uuid,
            version: Option<i32>,
        ) -> std::result::Result<Dataset, DatasetLookupError> {
            let Some(dataset) = self.dataset.as_ref() else {
                return Err(DatasetLookupError::DatasetNotFound { dataset_id: *id });
            };
            if &dataset.id != id {
                return Err(DatasetLookupError::DatasetNotFound { dataset_id: *id });
            }
            if let Some(version) = version {
                if dataset.version != version {
                    return Err(DatasetLookupError::VersionNotFound {
                        dataset_id: *id,
                        version,
                    });
                }
            }
            Ok(dataset.clone())
        }

        fn get_project(&self, _id: &Uuid) -> Result<crate::model::Project> {
            Err(anyhow!("not implemented"))
        }

        fn get_resolver(&self, _id: &str) -> Result<crate::model::Resolver> {
            Err(anyhow!("not implemented"))
        }

        fn update_run_status(&self, _id: &Uuid, _status: crate::model::RunStatus) -> Result<()> {
            Err(anyhow!("not implemented"))
        }
    }

    #[test]
    fn test_resolver_source_project_override() {
        let dataset_id = Uuid::new_v4();
        let mut overrides = BTreeMap::new();
        overrides.insert(dataset_id, "project".to_string());

        let (_, source) = resolve_resolver_with_source(
            &dataset_id,
            Some("dataset"),
            &overrides,
            "system-default",
        );
        assert_eq!(source, ResolverSource::ProjectOverride);
    }

    #[test]
    fn test_alias_column_reference_extraction() {
        let references = extract_assignment_alias_column_references(
            "amount_local * fx.rate + IF(customers.tier = 'gold', 1, 0)",
        );

        assert!(references.contains(&("fx".to_string(), "rate".to_string())));
        assert!(references.contains(&("customers".to_string(), "tier".to_string())));
    }

    #[test]
    fn test_validate_assignment_alias_references() {
        let mut alias_columns = HashMap::new();
        alias_columns.insert("fx".to_string(), vec!["rate".to_string()]);

        let expressions = vec!["amount_local * fx.rate".to_string()];
        assert!(validate_assignment_alias_references(&expressions, &alias_columns).is_ok());
    }

    #[test]
    fn test_resolve_pinned_version() {
        let dataset_id = Uuid::new_v4();
        let dataset = make_dataset(dataset_id, 5, DatasetStatus::Active);
        let store = SingleDatasetStore {
            dataset: Some(dataset),
        };

        let result = resolve_dataset_version(&dataset_id, Some(5), &store);
        assert!(result.is_ok());
        let (_, version) = result.expect("resolved version");
        assert_eq!(version, 5);
    }
}
