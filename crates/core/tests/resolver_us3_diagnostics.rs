use dobo_core::model::{
    Calendar, CalendarStatus, LevelDef, Period, PeriodStatus, ResolutionRule, ResolutionStrategy,
    Resolver, ResolverStatus,
};
use dobo_core::resolver::context::ResolutionRequest;
use dobo_core::resolver::diagnostics::{DiagnosticOutcome, ResolverSource};
use dobo_core::resolver::engine::{resolve_with_source, ResolutionError};
use uuid::Uuid;

fn create_calendar() -> Calendar {
    Calendar {
        id: Uuid::parse_str("550e8400-e29b-41d4-a716-446655440001").unwrap(),
        name: "Fiscal Calendar".to_string(),
        description: None,
        status: CalendarStatus::Active,
        is_default: true,
        levels: vec![
            LevelDef {
                name: "year".to_string(),
                parent_level: None,
                identifier_pattern: Some(r"^\d{4}$".to_string()),
                date_rules: vec![],
            },
            LevelDef {
                name: "quarter".to_string(),
                parent_level: Some("year".to_string()),
                identifier_pattern: Some(r"^\d{4}-Q[1-4]$".to_string()),
                date_rules: vec![],
            },
            LevelDef {
                name: "month".to_string(),
                parent_level: Some("quarter".to_string()),
                identifier_pattern: Some(r"^\d{4}-(0[1-9]|1[0-2])$".to_string()),
                date_rules: vec![],
            },
        ],
        created_at: None,
        updated_at: None,
    }
}

fn create_periods() -> Vec<Period> {
    let calendar_id = Uuid::parse_str("550e8400-e29b-41d4-a716-446655440001").unwrap();
    let year_id = Uuid::parse_str("550e8400-e29b-41d4-a716-446655440100").unwrap();
    let q1_id = Uuid::parse_str("550e8400-e29b-41d4-a716-446655440200").unwrap();

    vec![
        Period {
            id: year_id,
            identifier: "2024".to_string(),
            name: "2024".to_string(),
            description: None,
            calendar_id,
            year: 2024,
            sequence: 1,
            start_date: "2024-01-01".to_string(),
            end_date: "2024-12-31".to_string(),
            status: PeriodStatus::Open,
            parent_id: None,
            created_at: None,
            updated_at: None,
        },
        Period {
            id: q1_id,
            identifier: "2024-Q1".to_string(),
            name: "Q1 2024".to_string(),
            description: None,
            calendar_id,
            year: 2024,
            sequence: 1,
            start_date: "2024-01-01".to_string(),
            end_date: "2024-03-31".to_string(),
            status: PeriodStatus::Open,
            parent_id: Some(year_id),
            created_at: None,
            updated_at: None,
        },
        Period {
            id: Uuid::parse_str("550e8400-e29b-41d4-a716-446655440300").unwrap(),
            identifier: "2024-01".to_string(),
            name: "January 2024".to_string(),
            description: None,
            calendar_id,
            year: 2024,
            sequence: 1,
            start_date: "2024-01-01".to_string(),
            end_date: "2024-01-31".to_string(),
            status: PeriodStatus::Open,
            parent_id: Some(q1_id),
            created_at: None,
            updated_at: None,
        },
        Period {
            id: Uuid::parse_str("550e8400-e29b-41d4-a716-446655440301").unwrap(),
            identifier: "2024-02".to_string(),
            name: "February 2024".to_string(),
            description: None,
            calendar_id,
            year: 2024,
            sequence: 2,
            start_date: "2024-02-01".to_string(),
            end_date: "2024-02-29".to_string(),
            status: PeriodStatus::Open,
            parent_id: Some(q1_id),
            created_at: None,
            updated_at: None,
        },
        Period {
            id: Uuid::parse_str("550e8400-e29b-41d4-a716-446655440302").unwrap(),
            identifier: "2024-03".to_string(),
            name: "March 2024".to_string(),
            description: None,
            calendar_id,
            year: 2024,
            sequence: 3,
            start_date: "2024-03-01".to_string(),
            end_date: "2024-03-31".to_string(),
            status: PeriodStatus::Open,
            parent_id: Some(q1_id),
            created_at: None,
            updated_at: None,
        },
    ]
}

fn create_request() -> ResolutionRequest {
    ResolutionRequest {
        dataset_id: "test_dataset".to_string(),
        table_name: "fact_sales".to_string(),
        period_id: Uuid::parse_str("550e8400-e29b-41d4-a716-446655440200").unwrap(),
        project_id: None,
    }
}

#[test]
fn test_no_match_diagnostic_completeness() {
    let resolver = Resolver {
        id: "diag_resolver".to_string(),
        name: "Diagnostics".to_string(),
        description: None,
        version: 1,
        status: ResolverStatus::Active,
        is_default: Some(false),
        rules: vec![
            ResolutionRule {
                name: "sales_only".to_string(),
                when_expression: Some("table == 'inventory'".to_string()),
                data_level: "any".to_string(),
                strategy: ResolutionStrategy::Path {
                    datasource_id: "s3".to_string(),
                    path: "/data/{period_id}.parquet".to_string(),
                },
            },
            ResolutionRule {
                name: "prod_only".to_string(),
                when_expression: Some("dataset == 'prod'".to_string()),
                data_level: "any".to_string(),
                strategy: ResolutionStrategy::Path {
                    datasource_id: "s3".to_string(),
                    path: "/data/{period_id}.parquet".to_string(),
                },
            },
        ],
        created_at: None,
        updated_at: None,
    };

    let result = resolve_with_source(
        create_request(),
        resolver,
        create_calendar(),
        create_periods(),
        ResolverSource::DatasetReference,
    );

    match result {
        Err(ResolutionError::NoMatchingRule(diagnostic)) => {
            assert_eq!(diagnostic.outcome, DiagnosticOutcome::NoMatchingRule);
            assert_eq!(diagnostic.evaluated_rules.len(), 2);
            assert_eq!(diagnostic.evaluated_rules[0].rule_name, "sales_only");
            assert_eq!(diagnostic.evaluated_rules[1].rule_name, "prod_only");
        }
        other => panic!("expected NoMatchingRule, got: {:?}", other),
    }
}

#[test]
fn test_no_match_reasons() {
    let resolver = Resolver {
        id: "diag_resolver".to_string(),
        name: "Diagnostics".to_string(),
        description: None,
        version: 1,
        status: ResolverStatus::Active,
        is_default: Some(false),
        rules: vec![ResolutionRule {
            name: "sales_only".to_string(),
            when_expression: Some("table == 'inventory'".to_string()),
            data_level: "any".to_string(),
            strategy: ResolutionStrategy::Path {
                datasource_id: "s3".to_string(),
                path: "/data/{period_id}.parquet".to_string(),
            },
        }],
        created_at: None,
        updated_at: None,
    };

    let result = resolve_with_source(
        create_request(),
        resolver,
        create_calendar(),
        create_periods(),
        ResolverSource::DatasetReference,
    );

    match result {
        Err(ResolutionError::NoMatchingRule(diagnostic)) => {
            assert!(diagnostic.evaluated_rules[0]
                .reason
                .contains("evaluated to false"));
        }
        other => panic!("expected NoMatchingRule, got: {:?}", other),
    }
}

#[test]
fn test_template_error_diagnostic() {
    let resolver = Resolver {
        id: "template_resolver".to_string(),
        name: "Template Diagnostics".to_string(),
        description: None,
        version: 1,
        status: ResolverStatus::Active,
        is_default: Some(false),
        rules: vec![ResolutionRule {
            name: "bad_template".to_string(),
            when_expression: None,
            data_level: "any".to_string(),
            strategy: ResolutionStrategy::Path {
                datasource_id: "s3".to_string(),
                path: "/data/{unknown-token}.parquet".to_string(),
            },
        }],
        created_at: None,
        updated_at: None,
    };

    let result = resolve_with_source(
        create_request(),
        resolver,
        create_calendar(),
        create_periods(),
        ResolverSource::DatasetReference,
    );

    match result {
        Err(ResolutionError::TemplateRenderFailed { reason, diagnostic }) => {
            assert!(reason.contains("unknown-token"));
            assert_eq!(diagnostic.outcome, DiagnosticOutcome::TemplateRenderError);
            assert_eq!(diagnostic.evaluated_rules[0].rule_name, "bad_template");
        }
        other => panic!("expected TemplateRenderFailed, got: {:?}", other),
    }
}

#[test]
fn test_expression_error_diagnostic() {
    let resolver = Resolver {
        id: "expression_resolver".to_string(),
        name: "Expression Diagnostics".to_string(),
        description: None,
        version: 1,
        status: ResolverStatus::Active,
        is_default: Some(false),
        rules: vec![ResolutionRule {
            name: "bad_expression".to_string(),
            when_expression: Some("period >=".to_string()),
            data_level: "any".to_string(),
            strategy: ResolutionStrategy::Path {
                datasource_id: "s3".to_string(),
                path: "/data/{period_id}.parquet".to_string(),
            },
        }],
        created_at: None,
        updated_at: None,
    };

    let result = resolve_with_source(
        create_request(),
        resolver,
        create_calendar(),
        create_periods(),
        ResolverSource::DatasetReference,
    );

    match result {
        Err(ResolutionError::InvalidExpression { rule_name, reason }) => {
            assert_eq!(rule_name, "bad_expression");
            assert!(reason.contains("invalid expression"));
            assert!(reason.contains("period >="));
        }
        other => panic!("expected InvalidExpression, got: {:?}", other),
    }
}

#[test]
fn test_success_diagnostic_all_rules() {
    let resolver = Resolver {
        id: "success_resolver".to_string(),
        name: "Success Diagnostics".to_string(),
        description: None,
        version: 1,
        status: ResolverStatus::Active,
        is_default: Some(false),
        rules: vec![
            ResolutionRule {
                name: "first".to_string(),
                when_expression: Some("table == 'fact_sales'".to_string()),
                data_level: "any".to_string(),
                strategy: ResolutionStrategy::Path {
                    datasource_id: "s3".to_string(),
                    path: "/data/{period_id}.parquet".to_string(),
                },
            },
            ResolutionRule {
                name: "second".to_string(),
                when_expression: None,
                data_level: "any".to_string(),
                strategy: ResolutionStrategy::Path {
                    datasource_id: "other".to_string(),
                    path: "/other/{period_id}.parquet".to_string(),
                },
            },
        ],
        created_at: None,
        updated_at: None,
    };

    let result = resolve_with_source(
        create_request(),
        resolver,
        create_calendar(),
        create_periods(),
        ResolverSource::DatasetReference,
    )
    .unwrap();

    assert_eq!(result.diagnostic.evaluated_rules.len(), 2);
    assert!(result.diagnostic.evaluated_rules[0].matched);
    assert!(!result.diagnostic.evaluated_rules[1].matched);
    assert!(result.diagnostic.evaluated_rules[1]
        .reason
        .contains("earlier rule already matched"));
}

#[test]
fn test_resolver_source_dataset() {
    let resolver = Resolver {
        id: "dataset_resolver".to_string(),
        name: "Dataset Source".to_string(),
        description: None,
        version: 1,
        status: ResolverStatus::Active,
        is_default: Some(false),
        rules: vec![ResolutionRule {
            name: "rule".to_string(),
            when_expression: None,
            data_level: "any".to_string(),
            strategy: ResolutionStrategy::Path {
                datasource_id: "s3".to_string(),
                path: "/data/{period_id}.parquet".to_string(),
            },
        }],
        created_at: None,
        updated_at: None,
    };

    let result = resolve_with_source(
        create_request(),
        resolver,
        create_calendar(),
        create_periods(),
        ResolverSource::DatasetReference,
    )
    .unwrap();

    assert_eq!(
        result.diagnostic.resolver_source,
        ResolverSource::DatasetReference
    );
}

#[test]
fn test_location_traceability() {
    let resolver = Resolver {
        id: "trace_resolver".to_string(),
        name: "Traceability".to_string(),
        description: None,
        version: 1,
        status: ResolverStatus::Active,
        is_default: Some(false),
        rules: vec![ResolutionRule {
            name: "expand".to_string(),
            when_expression: None,
            data_level: "month".to_string(),
            strategy: ResolutionStrategy::Path {
                datasource_id: "s3".to_string(),
                path: "/data/{period_id}.parquet".to_string(),
            },
        }],
        created_at: None,
        updated_at: None,
    };

    let result = resolve_with_source(
        create_request(),
        resolver,
        create_calendar(),
        create_periods(),
        ResolverSource::DatasetReference,
    )
    .unwrap();

    assert_eq!(result.locations.len(), 3);
    for location in result.locations {
        assert_eq!(location.resolver_id, Some("trace_resolver".to_string()));
        assert_eq!(location.rule_name, Some("expand".to_string()));
    }
}
