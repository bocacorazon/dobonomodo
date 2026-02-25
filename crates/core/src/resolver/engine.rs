// Resolver engine - main resolution entry point
// This module implements the core resolve() function that evaluates rules,
// expands periods, and returns resolved locations.

use crate::model::{
    Calendar, Dataset, Period, Project, ResolutionRule, ResolvedLocation, Resolver, ResolverStatus,
};
use crate::resolver::calendar_matcher::CalendarMatcher;
use crate::resolver::context::{build_context_with_matcher, ResolutionContext, ResolutionRequest};
use crate::resolver::diagnostics::{
    build_no_match_diagnostic, determine_outcome, format_rule_reason, ResolutionDiagnostic,
    ResolutionResult, ResolverSource, RuleDiagnostic,
};
use crate::resolver::expander::expand_period;
use crate::resolver::matcher::evaluate_rule_with_detail;
use crate::resolver::renderer::{render_template, render_template_with_context, TemplateContext};
use std::collections::HashMap;
use thiserror::Error;
use uuid::Uuid;

#[derive(Debug, Error)]
pub enum ResolutionError {
    #[error("no rules matched: {0:?}")]
    NoMatchingRule(ResolutionDiagnostic),

    #[error("period expansion failed: {reason}")]
    PeriodExpansionFailed {
        reason: String,
        diagnostic: ResolutionDiagnostic,
    },

    #[error("template render failed: {reason}")]
    TemplateRenderFailed {
        reason: String,
        diagnostic: ResolutionDiagnostic,
    },

    #[error("invalid expression in rule '{rule_name}': {reason}")]
    InvalidExpression { rule_name: String, reason: String },

    #[error("context build failed: {0}")]
    ContextBuildFailed(String),

    #[error("period not found: {0}")]
    PeriodNotFound(String),

    #[error("resolver selection failed: {0}")]
    ResolverSelectionFailed(String),
}

/// Contract-compatible entry point with implicit dataset-reference source.
pub fn resolve(
    request: ResolutionRequest,
    resolver: Resolver,
    calendar: Calendar,
    periods: Vec<Period>,
) -> Result<ResolutionResult, ResolutionError> {
    resolve_with_source(
        request,
        resolver,
        calendar,
        periods,
        ResolverSource::DatasetReference,
    )
}

/// Main resolution function (T025, T024) with explicit resolver source.
pub fn resolve_with_source(
    request: ResolutionRequest,
    resolver: Resolver,
    calendar: Calendar,
    periods: Vec<Period>,
    resolver_source: ResolverSource,
) -> Result<ResolutionResult, ResolutionError> {
    let mut matcher = CalendarMatcher::new(&calendar);

    // Find the requested period
    let requested_period = periods
        .iter()
        .find(|p| p.id == request.period_id)
        .cloned()
        .ok_or_else(|| {
            ResolutionError::PeriodNotFound(format!("period {} not found", request.period_id))
        })?;

    // Build resolution context
    let mut context = build_context_with_matcher(&request, requested_period.clone(), &mut matcher)
        .map_err(ResolutionError::ContextBuildFailed)?;
    context.resolver_source = Some(resolver_source.clone());

    // Initialize diagnostic
    let mut diagnostic = ResolutionDiagnostic::new(resolver.id.clone(), resolver_source.clone());

    // Select matching rule (T024)
    let matched_rule = select_matching_rule(&resolver.rules, &context, &mut diagnostic)?;

    let expanded_periods = expand_period(
        &requested_period,
        &context.period_level,
        &matched_rule.data_level,
        &calendar,
        &periods,
    )
    .map_err(|reason| {
        diagnostic.set_outcome(determine_outcome(true, true, false));
        ResolutionError::PeriodExpansionFailed {
            reason,
            diagnostic: diagnostic.clone(),
        }
    })?;

    let mut locations = Vec::with_capacity(expanded_periods.len());
    let mut expanded_identifiers = Vec::with_capacity(expanded_periods.len());
    for expanded_period in expanded_periods {
        let mut expanded_context =
            build_context_with_matcher(&request, expanded_period.clone(), &mut matcher)
                .map_err(ResolutionError::ContextBuildFailed)?;
        expanded_context.resolver_source = Some(resolver_source.clone());
        let location =
            render_location(matched_rule, &expanded_context, &resolver.id).map_err(|reason| {
                diagnostic.set_outcome(determine_outcome(true, false, true));
                ResolutionError::TemplateRenderFailed {
                    reason,
                    diagnostic: diagnostic.clone(),
                }
            })?;
        expanded_identifiers.push(expanded_period.identifier);
        locations.push(location);
    }

    diagnostic.set_outcome(determine_outcome(true, false, false));
    diagnostic.set_expanded_periods(expanded_identifiers);

    Ok(ResolutionResult {
        locations,
        diagnostic,
    })
}

/// Resolve using FR-005 precedence: project override -> dataset resolver -> system default.
pub fn resolve_with_precedence(
    request: ResolutionRequest,
    project: Option<Project>,
    dataset: Option<Dataset>,
    resolvers: Vec<Resolver>,
    calendar: Calendar,
    periods: Vec<Period>,
) -> Result<ResolutionResult, ResolutionError> {
    let (resolver, source) =
        select_resolver_with_precedence(&request, project.as_ref(), dataset.as_ref(), &resolvers)?;
    resolve_with_source(request, resolver, calendar, periods, source)
}

/// Select the first matching rule (T024)
fn select_matching_rule<'a>(
    rules: &'a [ResolutionRule],
    context: &ResolutionContext,
    diagnostic: &mut ResolutionDiagnostic,
) -> Result<&'a ResolutionRule, ResolutionError> {
    for (rule_index, rule) in rules.iter().enumerate() {
        match evaluate_rule_with_detail(rule, context) {
            Ok(detail) if detail.matched => {
                // Rule matched
                let reason =
                    format_rule_reason(detail.evaluated_expression.as_deref(), true, false, None);

                diagnostic.add_rule_diagnostic(RuleDiagnostic::matched(
                    rule.name.clone(),
                    reason,
                    detail.evaluated_expression.clone(),
                ));

                // Mark remaining rules as skipped (T027)
                for remaining_rule in rules.iter().skip(rule_index + 1) {
                    diagnostic
                        .add_rule_diagnostic(RuleDiagnostic::skipped(remaining_rule.name.clone()));
                }

                return Ok(rule);
            }
            Ok(detail) => {
                // Rule did not match
                let reason =
                    format_rule_reason(detail.evaluated_expression.as_deref(), false, false, None);

                diagnostic.add_rule_diagnostic(RuleDiagnostic::not_matched(
                    rule.name.clone(),
                    reason,
                    detail.evaluated_expression.clone(),
                ));
            }
            Err(e) => {
                let reason =
                    format_rule_reason(rule.when_expression.as_deref(), false, false, Some(&e));
                diagnostic.add_rule_diagnostic(RuleDiagnostic::not_matched(
                    rule.name.clone(),
                    reason.clone(),
                    rule.when_expression.clone(),
                ));
                // Expression evaluation error
                return Err(ResolutionError::InvalidExpression {
                    rule_name: rule.name.clone(),
                    reason,
                });
            }
        }
    }

    // No rules matched (T028)
    diagnostic.set_outcome(determine_outcome(false, false, false));
    Err(ResolutionError::NoMatchingRule(build_no_match_diagnostic(
        diagnostic.clone(),
    )))
}

/// Render a location from a rule and context
fn render_location(
    rule: &ResolutionRule,
    context: &ResolutionContext,
    resolver_id: &str,
) -> Result<ResolvedLocation, String> {
    // Build template context
    let mut template_ctx = HashMap::new();
    template_ctx.insert("period_id".to_string(), context.period.identifier.clone());
    template_ctx.insert("period_name".to_string(), context.period.name.clone());
    template_ctx.insert("table_name".to_string(), context.table_name.clone());
    template_ctx.insert("dataset_id".to_string(), context.dataset_id.clone());

    // Render based on strategy type
    use crate::model::ResolutionStrategy;
    let mut location = match &rule.strategy {
        ResolutionStrategy::Path {
            datasource_id,
            path,
        } => {
            template_ctx.insert("datasource_id".to_string(), datasource_id.clone());
            let rendered_path =
                render_template_with_context(path, &template_ctx, TemplateContext::Path)?;

            ResolvedLocation {
                datasource_id: datasource_id.clone(),
                path: Some(rendered_path),
                table: None,
                schema: None,
                period_identifier: Some(context.period.identifier.clone()),
                resolver_id: Some(resolver_id.to_string()),
                rule_name: Some(rule.name.clone()),
                catalog_response: None,
            }
        }
        ResolutionStrategy::Table {
            datasource_id,
            table,
            schema,
        } => {
            template_ctx.insert("datasource_id".to_string(), datasource_id.clone());
            let rendered_table = render_template(table, &template_ctx)?;

            let rendered_schema = schema
                .as_ref()
                .map(|s| render_template(s, &template_ctx))
                .transpose()?;

            ResolvedLocation {
                datasource_id: datasource_id.clone(),
                path: None,
                table: Some(rendered_table),
                schema: rendered_schema,
                period_identifier: Some(context.period.identifier.clone()),
                resolver_id: Some(resolver_id.to_string()),
                rule_name: Some(rule.name.clone()),
                catalog_response: None,
            }
        }
        ResolutionStrategy::Catalog {
            endpoint,
            method,
            auth,
            params,
            headers,
        } => {
            template_ctx.insert("datasource_id".to_string(), "catalog".to_string());
            let rendered_endpoint =
                render_template_with_context(endpoint, &template_ctx, TemplateContext::Endpoint)?;
            let rendered_method =
                render_template_with_context(method, &template_ctx, TemplateContext::Endpoint)?;
            let rendered_auth = auth
                .as_ref()
                .map(|value| {
                    render_template_with_context(value, &template_ctx, TemplateContext::Endpoint)
                })
                .transpose()?;
            let rendered_params = render_catalog_template_value(params, &template_ctx)?;
            let rendered_headers = render_catalog_template_value(headers, &template_ctx)?;

            ResolvedLocation {
                datasource_id: "catalog".to_string(),
                path: Some(rendered_endpoint),
                table: None,
                schema: None,
                period_identifier: Some(context.period.identifier.clone()),
                resolver_id: Some(resolver_id.to_string()),
                rule_name: Some(rule.name.clone()),
                catalog_response: Some(serde_json::json!({
                    "method": rendered_method,
                    "auth": rendered_auth,
                    "params": rendered_params,
                    "headers": rendered_headers,
                })),
            }
        }
    };

    // Ensure traceability fields are set
    location.resolver_id = Some(resolver_id.to_string());
    location.rule_name = Some(rule.name.clone());

    Ok(location)
}

fn render_catalog_template_value(
    value: &serde_json::Value,
    context: &HashMap<String, String>,
) -> Result<serde_json::Value, String> {
    match value {
        serde_json::Value::String(raw) => Ok(serde_json::Value::String(
            render_template_with_context(raw, context, TemplateContext::Endpoint)?,
        )),
        serde_json::Value::Array(items) => {
            let rendered = items
                .iter()
                .map(|item| render_catalog_template_value(item, context))
                .collect::<Result<Vec<_>, _>>()?;
            Ok(serde_json::Value::Array(rendered))
        }
        serde_json::Value::Object(map) => {
            let mut rendered = serde_json::Map::with_capacity(map.len());
            for (key, entry) in map {
                rendered.insert(key.clone(), render_catalog_template_value(entry, context)?);
            }
            Ok(serde_json::Value::Object(rendered))
        }
        _ => Ok(value.clone()),
    }
}

fn select_resolver_with_precedence(
    request: &ResolutionRequest,
    project: Option<&Project>,
    dataset: Option<&Dataset>,
    resolvers: &[Resolver],
) -> Result<(Resolver, ResolverSource), ResolutionError> {
    if let Some(project) = project {
        let dataset_uuid = dataset
            .map(|d| d.id)
            .or_else(|| Uuid::parse_str(&request.dataset_id).ok());
        if let Some(dataset_uuid) = dataset_uuid {
            if let Some(resolver_id) = project.resolver_overrides.get(&dataset_uuid) {
                if let Some(resolver) = find_active_resolver(resolvers, resolver_id) {
                    return Ok((resolver.clone(), ResolverSource::ProjectOverride));
                }
            }
        }
    }

    if let Some(dataset) = dataset {
        if let Some(resolver_id) = dataset.resolver_id.as_deref() {
            if let Some(resolver) = find_active_resolver(resolvers, resolver_id) {
                return Ok((resolver.clone(), ResolverSource::DatasetReference));
            }
        }
    }

    if let Some(resolver) = resolvers.iter().find(|resolver| {
        resolver.status == ResolverStatus::Active && resolver.is_default == Some(true)
    }) {
        return Ok((resolver.clone(), ResolverSource::SystemDefault));
    }

    Err(ResolutionError::ResolverSelectionFailed(
        "no resolver available from project override, dataset resolver reference, or system default"
            .to_string(),
    ))
}

fn find_active_resolver<'a>(resolvers: &'a [Resolver], resolver_id: &str) -> Option<&'a Resolver> {
    resolvers
        .iter()
        .find(|resolver| resolver.id == resolver_id && resolver.status == ResolverStatus::Active)
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::model::{CalendarStatus, LevelDef, PeriodStatus, ResolutionStrategy};
    use serde_json::json;

    fn sample_context() -> ResolutionContext {
        ResolutionContext {
            dataset_id: "team/a".to_string(),
            table_name: "sales report".to_string(),
            period: Period {
                id: Uuid::parse_str("550e8400-e29b-41d4-a716-446655440000").unwrap(),
                identifier: "2024-01".to_string(),
                name: "January".to_string(),
                description: None,
                calendar_id: Uuid::parse_str("550e8400-e29b-41d4-a716-446655440010").unwrap(),
                year: 2024,
                sequence: 1,
                start_date: "2024-01-01".to_string(),
                end_date: "2024-01-31".to_string(),
                status: PeriodStatus::Open,
                parent_id: None,
                created_at: None,
                updated_at: None,
            },
            period_level: "month".to_string(),
            resolver_source: None,
            additional_context: HashMap::new(),
        }
    }

    #[test]
    fn resolve_defaults_to_dataset_reference_source() {
        let period_id = Uuid::parse_str("550e8400-e29b-41d4-a716-446655440000").unwrap();
        let calendar_id = Uuid::parse_str("550e8400-e29b-41d4-a716-446655440010").unwrap();
        let request = ResolutionRequest {
            dataset_id: "sales_dataset".to_string(),
            table_name: "daily_transactions".to_string(),
            period_id,
            project_id: None,
        };
        let resolver = Resolver {
            id: "resolver_v1".to_string(),
            name: "Resolver".to_string(),
            description: None,
            version: 1,
            status: ResolverStatus::Active,
            is_default: Some(true),
            rules: vec![ResolutionRule {
                name: "default".to_string(),
                when_expression: None,
                data_level: "any".to_string(),
                strategy: ResolutionStrategy::Path {
                    datasource_id: "s3".to_string(),
                    path: "/data/{period_id}/{table_name}.parquet".to_string(),
                },
            }],
            created_at: None,
            updated_at: None,
        };
        let calendar = Calendar {
            id: calendar_id,
            name: "Gregorian".to_string(),
            description: None,
            status: CalendarStatus::Active,
            is_default: true,
            levels: vec![LevelDef {
                name: "month".to_string(),
                parent_level: None,
                identifier_pattern: Some(r"^\d{4}-(0[1-9]|1[0-2])$".to_string()),
                date_rules: vec![],
            }],
            created_at: None,
            updated_at: None,
        };
        let periods = vec![Period {
            id: period_id,
            identifier: "2024-01".to_string(),
            name: "January".to_string(),
            description: None,
            calendar_id,
            year: 2024,
            sequence: 1,
            start_date: "2024-01-01".to_string(),
            end_date: "2024-01-31".to_string(),
            status: PeriodStatus::Open,
            parent_id: None,
            created_at: None,
            updated_at: None,
        }];

        let result = resolve(request, resolver, calendar, periods).unwrap();
        assert_eq!(
            result.diagnostic.resolver_source,
            ResolverSource::DatasetReference
        );
    }

    #[test]
    fn catalog_strategy_renders_params_and_headers_templates() {
        let context = sample_context();
        let rule = ResolutionRule {
            name: "catalog_rule".to_string(),
            when_expression: None,
            data_level: "any".to_string(),
            strategy: ResolutionStrategy::Catalog {
                endpoint: "https://catalog.example.com/{dataset_id}/{table_name}".to_string(),
                method: "GET".to_string(),
                auth: Some("Bearer {dataset_id}".to_string()),
                params: json!({
                    "period": "{period_id}",
                    "nested": { "table": "{table_name}" }
                }),
                headers: json!({
                    "X-Dataset": "{dataset_id}",
                    "X-Table": "{table_name}"
                }),
            },
        };

        let location = render_location(&rule, &context, "resolver_v1").unwrap();
        assert_eq!(
            location.path,
            Some("https://catalog.example.com/team%2Fa/sales%20report".to_string())
        );
    }

    #[test]
    fn catalog_strategy_fails_on_unknown_token_in_params() {
        let context = sample_context();
        let rule = ResolutionRule {
            name: "catalog_rule".to_string(),
            when_expression: None,
            data_level: "any".to_string(),
            strategy: ResolutionStrategy::Catalog {
                endpoint: "https://catalog.example.com/{dataset_id}".to_string(),
                method: "GET".to_string(),
                auth: None,
                params: json!({ "bad": "{unknown_token}" }),
                headers: json!({}),
            },
        };

        let error = render_location(&rule, &context, "resolver_v1").unwrap_err();
        assert!(error.contains("unknown token"));
    }
}
