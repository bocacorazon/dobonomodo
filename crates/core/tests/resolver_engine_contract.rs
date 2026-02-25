// Contract tests for resolver engine
// Tests verify API behavior contracts as specified in contracts/resolver-engine-api.md

use dobo_core::model::{
    Calendar, CalendarStatus, ColumnDef, ColumnType, Dataset, DatasetStatus, LevelDef,
    Materialization, Period, PeriodStatus, Project, ProjectStatus, ResolutionRule,
    ResolutionStrategy, Resolver, ResolverStatus, TableRef, Visibility,
};
use dobo_core::resolver::context::ResolutionRequest;
use dobo_core::resolver::diagnostics::{DiagnosticOutcome, ResolverSource};
use dobo_core::resolver::engine::{
    resolve, resolve_with_precedence, resolve_with_source, ResolutionError,
};
use serde_json::json;
use std::collections::BTreeMap;
use uuid::Uuid;

// Helper to create test calendar
fn create_test_calendar() -> Calendar {
    Calendar {
        id: Uuid::parse_str("550e8400-e29b-41d4-a716-446655440001").unwrap(),
        name: "Test Calendar".to_string(),
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

// Helper to create test period (quarter)
fn create_test_quarter() -> Period {
    Period {
        id: Uuid::parse_str("550e8400-e29b-41d4-a716-446655440200").unwrap(),
        identifier: "2024-Q1".to_string(),
        name: "Q1 2024".to_string(),
        description: None,
        calendar_id: Uuid::parse_str("550e8400-e29b-41d4-a716-446655440001").unwrap(),
        year: 2024,
        sequence: 1,
        start_date: "2024-01-01".to_string(),
        end_date: "2024-03-31".to_string(),
        status: PeriodStatus::Open,
        parent_id: Some(Uuid::parse_str("550e8400-e29b-41d4-a716-446655440100").unwrap()),
        created_at: None,
        updated_at: None,
    }
}

#[test]
fn test_first_match_semantics() {
    // Contract 1: Resolution evaluates rules in order and stops at the first match
    let resolver = Resolver {
        id: "test_resolver".to_string(),
        name: "Test Resolver".to_string(),
        description: None,
        version: 1,
        status: ResolverStatus::Active,
        is_default: Some(false),
        rules: vec![
            ResolutionRule {
                name: "rule1_legacy".to_string(),
                when_expression: Some("period < '2024-Q1'".to_string()),
                data_level: "month".to_string(),
                strategy: ResolutionStrategy::Table {
                    datasource_id: "legacy_db".to_string(),
                    table: "old_data".to_string(),
                    schema: None,
                },
            },
            ResolutionRule {
                name: "rule2_new".to_string(),
                when_expression: None, // catch-all
                data_level: "any".to_string(),
                strategy: ResolutionStrategy::Path {
                    datasource_id: "new_s3".to_string(),
                    path: "/data/new/{period_id}.parquet".to_string(),
                },
            },
        ],
        created_at: None,
        updated_at: None,
    };

    let request = ResolutionRequest {
        dataset_id: "test_dataset".to_string(),
        table_name: "test_table".to_string(),
        period_id: Uuid::parse_str("550e8400-e29b-41d4-a716-446655440200").unwrap(),
        project_id: None,
    };

    let calendar = create_test_calendar();
    let period = create_test_quarter(); // 2024-Q1
    let periods = vec![period.clone()];

    let result = resolve(request, resolver, calendar, periods);

    assert!(result.is_ok(), "resolution should succeed");
    let res = result.unwrap();

    // Rule 1 should not match (period is not < '2024-Q1')
    // Rule 2 should match (unconditional)
    assert!(!res.diagnostic.evaluated_rules[0].matched);
    assert!(res.diagnostic.evaluated_rules[1].matched);
    assert_eq!(
        res.diagnostic.evaluated_rules[1].reason,
        "no when condition (unconditional match)"
    );
}

#[test]
fn test_unconditional_rule_match() {
    // Contract: Rules with when_expression=None always match
    let resolver = Resolver {
        id: "catch_all_resolver".to_string(),
        name: "Catch All".to_string(),
        description: None,
        version: 1,
        status: ResolverStatus::Active,
        is_default: Some(false),
        rules: vec![ResolutionRule {
            name: "catch_all".to_string(),
            when_expression: None,
            data_level: "any".to_string(),
            strategy: ResolutionStrategy::Path {
                datasource_id: "default_s3".to_string(),
                path: "/default/path.parquet".to_string(),
            },
        }],
        created_at: None,
        updated_at: None,
    };

    let request = ResolutionRequest {
        dataset_id: "any_dataset".to_string(),
        table_name: "any_table".to_string(),
        period_id: Uuid::parse_str("550e8400-e29b-41d4-a716-446655440200").unwrap(),
        project_id: None,
    };

    let calendar = create_test_calendar();
    let period = create_test_quarter();
    let periods = vec![period.clone()];

    let result = resolve_with_source(
        request,
        resolver,
        calendar,
        periods,
        ResolverSource::SystemDefault,
    );

    assert!(result.is_ok());
    let res = result.unwrap();

    // Unconditional rule should always match
    assert!(res.diagnostic.evaluated_rules[0].matched);
    assert_eq!(res.locations.len(), 1);
    assert_eq!(res.locations[0].datasource_id, "default_s3");
}

#[test]
fn test_period_expansion() {
    // Contract 2: Expand to child periods when data_level is finer
    let calendar_id = Uuid::parse_str("550e8400-e29b-41d4-a716-446655440001").unwrap();
    let q1_id = Uuid::parse_str("550e8400-e29b-41d4-a716-446655440200").unwrap();

    let calendar = create_test_calendar();
    let periods = vec![
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
            parent_id: None,
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
    ];

    let resolver = Resolver {
        id: "expansion_resolver".to_string(),
        name: "Expansion Test".to_string(),
        description: None,
        version: 1,
        status: ResolverStatus::Active,
        is_default: Some(false),
        rules: vec![ResolutionRule {
            name: "month_level".to_string(),
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

    let request = ResolutionRequest {
        dataset_id: "test".to_string(),
        table_name: "data".to_string(),
        period_id: q1_id,
        project_id: None,
    };

    let result = resolve_with_source(
        request,
        resolver,
        calendar,
        periods,
        ResolverSource::DatasetReference,
    );

    assert!(result.is_ok());
    let res = result.unwrap();

    // Should expand to 3 months
    assert_eq!(res.locations.len(), 3);
    assert_eq!(
        res.locations[0].period_identifier,
        Some("2024-01".to_string())
    );
    assert_eq!(
        res.locations[1].period_identifier,
        Some("2024-02".to_string())
    );
    assert_eq!(
        res.locations[2].period_identifier,
        Some("2024-03".to_string())
    );
}

#[test]
fn test_no_expansion_for_any_level() {
    // Contract 3: data_level="any" returns single location
    let resolver = Resolver {
        id: "any_level_resolver".to_string(),
        name: "Any Level".to_string(),
        description: None,
        version: 1,
        status: ResolverStatus::Active,
        is_default: Some(false),
        rules: vec![ResolutionRule {
            name: "any_rule".to_string(),
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

    let request = ResolutionRequest {
        dataset_id: "test".to_string(),
        table_name: "data".to_string(),
        period_id: Uuid::parse_str("550e8400-e29b-41d4-a716-446655440200").unwrap(),
        project_id: None,
    };

    let calendar = create_test_calendar();
    let period = create_test_quarter();
    let periods = vec![period.clone()];

    let result = resolve_with_source(
        request,
        resolver,
        calendar,
        periods,
        ResolverSource::DatasetReference,
    );

    assert!(result.is_ok());
    let res = result.unwrap();

    // Should return exactly one location
    assert_eq!(res.locations.len(), 1);
    assert_eq!(
        res.locations[0].period_identifier,
        Some("2024-Q1".to_string())
    );
}

#[test]
fn test_deterministic_ordering() {
    // Contract 4: Locations ordered by period sequence
    let calendar_id = Uuid::parse_str("550e8400-e29b-41d4-a716-446655440001").unwrap();
    let q1_id = Uuid::parse_str("550e8400-e29b-41d4-a716-446655440200").unwrap();

    let calendar = create_test_calendar();

    // Create periods out of order
    let periods = vec![
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
            parent_id: None,
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
    ];

    let resolver = Resolver {
        id: "order_resolver".to_string(),
        name: "Order Test".to_string(),
        description: None,
        version: 1,
        status: ResolverStatus::Active,
        is_default: Some(false),
        rules: vec![ResolutionRule {
            name: "month_level".to_string(),
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

    let request = ResolutionRequest {
        dataset_id: "test".to_string(),
        table_name: "data".to_string(),
        period_id: q1_id,
        project_id: None,
    };

    let result = resolve_with_source(
        request,
        resolver,
        calendar,
        periods,
        ResolverSource::DatasetReference,
    );

    assert!(result.is_ok());
    let res = result.unwrap();

    // Despite input order, output should be sorted by sequence
    assert_eq!(
        res.locations[0].period_identifier,
        Some("2024-01".to_string())
    );
    assert_eq!(
        res.locations[1].period_identifier,
        Some("2024-02".to_string())
    );
    assert_eq!(
        res.locations[2].period_identifier,
        Some("2024-03".to_string())
    );
}

#[test]
fn test_traceability() {
    let calendar_id = Uuid::parse_str("550e8400-e29b-41d4-a716-446655440001").unwrap();
    let q1_id = Uuid::parse_str("550e8400-e29b-41d4-a716-446655440200").unwrap();

    let resolver = Resolver {
        id: "traceability_resolver".to_string(),
        name: "Traceability".to_string(),
        description: None,
        version: 1,
        status: ResolverStatus::Active,
        is_default: Some(false),
        rules: vec![ResolutionRule {
            name: "trace_rule".to_string(),
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

    let periods = vec![
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
            parent_id: None,
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
    ];

    let request = ResolutionRequest {
        dataset_id: "test".to_string(),
        table_name: "data".to_string(),
        period_id: q1_id,
        project_id: None,
    };

    let result = resolve_with_source(
        request,
        resolver,
        create_test_calendar(),
        periods,
        ResolverSource::DatasetReference,
    )
    .unwrap();

    for location in result.locations {
        assert_eq!(
            location.resolver_id,
            Some("traceability_resolver".to_string())
        );
        assert_eq!(location.rule_name, Some("trace_rule".to_string()));
    }
}

#[test]
fn test_no_matching_rule_contains_all_rule_diagnostics() {
    let resolver = Resolver {
        id: "no_match_resolver".to_string(),
        name: "No Match".to_string(),
        description: None,
        version: 1,
        status: ResolverStatus::Active,
        is_default: Some(false),
        rules: vec![
            ResolutionRule {
                name: "sales_only".to_string(),
                when_expression: Some("table == 'sales'".to_string()),
                data_level: "any".to_string(),
                strategy: ResolutionStrategy::Path {
                    datasource_id: "s3".to_string(),
                    path: "/data/{period_id}.parquet".to_string(),
                },
            },
            ResolutionRule {
                name: "prod_dataset".to_string(),
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

    let request = ResolutionRequest {
        dataset_id: "staging".to_string(),
        table_name: "inventory".to_string(),
        period_id: Uuid::parse_str("550e8400-e29b-41d4-a716-446655440200").unwrap(),
        project_id: None,
    };

    let result = resolve_with_source(
        request,
        resolver,
        create_test_calendar(),
        vec![create_test_quarter()],
        ResolverSource::DatasetReference,
    );

    match result {
        Err(ResolutionError::NoMatchingRule(diagnostic)) => {
            assert_eq!(diagnostic.outcome, DiagnosticOutcome::NoMatchingRule);
            assert_eq!(diagnostic.evaluated_rules.len(), 2);
            assert_eq!(diagnostic.evaluated_rules[0].rule_name, "sales_only");
            assert!(diagnostic.evaluated_rules[0]
                .reason
                .contains("evaluated to false"));
            assert_eq!(diagnostic.evaluated_rules[1].rule_name, "prod_dataset");
            assert!(diagnostic.evaluated_rules[1]
                .reason
                .contains("evaluated to false"));
        }
        other => panic!("expected NoMatchingRule error, got: {:?}", other),
    }
}

#[test]
fn test_template_render_error_preserves_diagnostic_context() {
    let resolver = Resolver {
        id: "template_error_resolver".to_string(),
        name: "Template Error".to_string(),
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
                path: "/data/{unknown_token}.parquet".to_string(),
            },
        }],
        created_at: None,
        updated_at: None,
    };

    let request = ResolutionRequest {
        dataset_id: "test_dataset".to_string(),
        table_name: "test_table".to_string(),
        period_id: Uuid::parse_str("550e8400-e29b-41d4-a716-446655440200").unwrap(),
        project_id: None,
    };

    let result = resolve_with_source(
        request,
        resolver,
        create_test_calendar(),
        vec![create_test_quarter()],
        ResolverSource::ProjectOverride,
    );

    match result {
        Err(ResolutionError::TemplateRenderFailed { reason, diagnostic }) => {
            assert!(reason.contains("unknown token"));
            assert_eq!(diagnostic.resolver_source, ResolverSource::ProjectOverride);
            assert_eq!(diagnostic.evaluated_rules.len(), 1);
            assert_eq!(diagnostic.evaluated_rules[0].rule_name, "bad_template");
            assert_eq!(diagnostic.outcome, DiagnosticOutcome::TemplateRenderError);
        }
        other => panic!("expected TemplateRenderFailed, got: {:?}", other),
    }
}

#[test]
fn test_template_render_error_for_unsupported_placeholder_format() {
    let resolver = Resolver {
        id: "template_error_resolver".to_string(),
        name: "Template Error".to_string(),
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

    let request = ResolutionRequest {
        dataset_id: "test_dataset".to_string(),
        table_name: "test_table".to_string(),
        period_id: Uuid::parse_str("550e8400-e29b-41d4-a716-446655440200").unwrap(),
        project_id: None,
    };

    let result = resolve_with_source(
        request,
        resolver,
        create_test_calendar(),
        vec![create_test_quarter()],
        ResolverSource::ProjectOverride,
    );

    match result {
        Err(ResolutionError::TemplateRenderFailed { reason, .. }) => {
            assert!(reason.contains("unknown-token"));
        }
        other => panic!("expected TemplateRenderFailed, got: {:?}", other),
    }
}

fn create_precedence_resolver(
    resolver_id: &str,
    datasource_id: &str,
    is_default: bool,
) -> Resolver {
    Resolver {
        id: resolver_id.to_string(),
        name: resolver_id.to_string(),
        description: None,
        version: 1,
        status: ResolverStatus::Active,
        is_default: Some(is_default),
        rules: vec![ResolutionRule {
            name: format!("{resolver_id}_rule"),
            when_expression: None,
            data_level: "any".to_string(),
            strategy: ResolutionStrategy::Path {
                datasource_id: datasource_id.to_string(),
                path: format!("/{datasource_id}/{{period_id}}.parquet"),
            },
        }],
        created_at: None,
        updated_at: None,
    }
}

fn create_precedence_dataset(dataset_uuid: Uuid, resolver_id: Option<&str>) -> Dataset {
    Dataset {
        id: dataset_uuid,
        name: "Test Dataset".to_string(),
        description: None,
        owner: "owner".to_string(),
        version: 1,
        status: DatasetStatus::Active,
        resolver_id: resolver_id.map(str::to_string),
        main_table: TableRef {
            name: "fact_sales".to_string(),
            temporal_mode: None,
            columns: vec![ColumnDef {
                name: "id".to_string(),
                column_type: ColumnType::String,
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

fn create_precedence_project(dataset_uuid: Uuid, override_resolver: Option<&str>) -> Project {
    let mut resolver_overrides = BTreeMap::new();
    if let Some(override_resolver) = override_resolver {
        resolver_overrides.insert(dataset_uuid, override_resolver.to_string());
    }

    Project {
        id: Uuid::parse_str("550e8400-e29b-41d4-a716-446655440020").unwrap(),
        name: "Project".to_string(),
        description: None,
        owner: "owner".to_string(),
        version: 1,
        status: ProjectStatus::Active,
        visibility: Visibility::Private,
        input_dataset_id: dataset_uuid,
        input_dataset_version: 1,
        materialization: Materialization::Runtime,
        operations: vec![],
        selectors: BTreeMap::new(),
        resolver_overrides,
        conflict_report: None,
        created_at: None,
        updated_at: None,
    }
}

#[test]
fn test_precedence_selects_project_override_first() {
    let dataset_uuid = Uuid::parse_str("550e8400-e29b-41d4-a716-446655440010").unwrap();
    let period_id = Uuid::parse_str("550e8400-e29b-41d4-a716-446655440200").unwrap();
    let calendar_id = Uuid::parse_str("550e8400-e29b-41d4-a716-446655440001").unwrap();

    let project_resolver = Resolver {
        id: "project_override_resolver".to_string(),
        name: "Project Override".to_string(),
        description: None,
        version: 1,
        status: ResolverStatus::Active,
        is_default: Some(false),
        rules: vec![ResolutionRule {
            name: "project_rule".to_string(),
            when_expression: None,
            data_level: "any".to_string(),
            strategy: ResolutionStrategy::Path {
                datasource_id: "project_ds".to_string(),
                path: "/project/{period_id}.parquet".to_string(),
            },
        }],
        created_at: None,
        updated_at: None,
    };

    let dataset_resolver = Resolver {
        id: "dataset_resolver".to_string(),
        name: "Dataset Resolver".to_string(),
        description: None,
        version: 1,
        status: ResolverStatus::Active,
        is_default: Some(false),
        rules: vec![ResolutionRule {
            name: "dataset_rule".to_string(),
            when_expression: None,
            data_level: "any".to_string(),
            strategy: ResolutionStrategy::Path {
                datasource_id: "dataset_ds".to_string(),
                path: "/dataset/{period_id}.parquet".to_string(),
            },
        }],
        created_at: None,
        updated_at: None,
    };

    let system_resolver = Resolver {
        id: "system_default_resolver".to_string(),
        name: "System Default".to_string(),
        description: None,
        version: 1,
        status: ResolverStatus::Active,
        is_default: Some(true),
        rules: vec![ResolutionRule {
            name: "system_rule".to_string(),
            when_expression: None,
            data_level: "any".to_string(),
            strategy: ResolutionStrategy::Path {
                datasource_id: "system_ds".to_string(),
                path: "/system/{period_id}.parquet".to_string(),
            },
        }],
        created_at: None,
        updated_at: None,
    };

    let dataset = Dataset {
        id: dataset_uuid,
        name: "Test Dataset".to_string(),
        description: None,
        owner: "owner".to_string(),
        version: 1,
        status: DatasetStatus::Active,
        resolver_id: Some("dataset_resolver".to_string()),
        main_table: TableRef {
            name: "fact_sales".to_string(),
            temporal_mode: None,
            columns: vec![ColumnDef {
                name: "id".to_string(),
                column_type: ColumnType::String,
                nullable: Some(false),
                description: None,
            }],
        },
        lookups: vec![],
        natural_key_columns: vec![],
        created_at: None,
        updated_at: None,
    };

    let mut resolver_overrides = BTreeMap::new();
    resolver_overrides.insert(dataset_uuid, "project_override_resolver".to_string());
    let project = Project {
        id: Uuid::parse_str("550e8400-e29b-41d4-a716-446655440020").unwrap(),
        name: "Project".to_string(),
        description: None,
        owner: "owner".to_string(),
        version: 1,
        status: ProjectStatus::Active,
        visibility: Visibility::Private,
        input_dataset_id: dataset_uuid,
        input_dataset_version: 1,
        materialization: Materialization::Runtime,
        operations: vec![],
        selectors: BTreeMap::new(),
        resolver_overrides,
        conflict_report: None,
        created_at: None,
        updated_at: None,
    };

    let request = ResolutionRequest {
        dataset_id: dataset_uuid.to_string(),
        table_name: "fact_sales".to_string(),
        period_id,
        project_id: Some(project.id.to_string()),
    };

    let quarter = Period {
        id: period_id,
        identifier: "2024-Q1".to_string(),
        name: "Q1 2024".to_string(),
        description: None,
        calendar_id,
        year: 2024,
        sequence: 1,
        start_date: "2024-01-01".to_string(),
        end_date: "2024-03-31".to_string(),
        status: PeriodStatus::Open,
        parent_id: None,
        created_at: None,
        updated_at: None,
    };

    let result = resolve_with_precedence(
        request,
        Some(project),
        Some(dataset),
        vec![system_resolver, dataset_resolver, project_resolver],
        create_test_calendar(),
        vec![quarter],
    )
    .unwrap();

    assert_eq!(
        result.diagnostic.resolver_source,
        ResolverSource::ProjectOverride
    );
    assert_eq!(result.diagnostic.resolver_id, "project_override_resolver");
    assert_eq!(result.locations[0].datasource_id, "project_ds");
}

#[test]
fn test_precedence_falls_back_to_dataset_resolver() {
    let dataset_uuid = Uuid::parse_str("550e8400-e29b-41d4-a716-446655440010").unwrap();
    let period_id = Uuid::parse_str("550e8400-e29b-41d4-a716-446655440200").unwrap();

    let project = create_precedence_project(dataset_uuid, None);
    let dataset = create_precedence_dataset(dataset_uuid, Some("dataset_resolver"));
    let resolvers = vec![
        create_precedence_resolver("system_default_resolver", "system_ds", true),
        create_precedence_resolver("dataset_resolver", "dataset_ds", false),
        create_precedence_resolver("project_override_resolver", "project_ds", false),
    ];

    let request = ResolutionRequest {
        dataset_id: dataset_uuid.to_string(),
        table_name: "fact_sales".to_string(),
        period_id,
        project_id: Some(project.id.to_string()),
    };

    let result = resolve_with_precedence(
        request,
        Some(project),
        Some(dataset),
        resolvers,
        create_test_calendar(),
        vec![create_test_quarter()],
    )
    .unwrap();

    assert_eq!(
        result.diagnostic.resolver_source,
        ResolverSource::DatasetReference
    );
    assert_eq!(result.diagnostic.resolver_id, "dataset_resolver");
    assert_eq!(result.locations[0].datasource_id, "dataset_ds");
}

#[test]
fn test_precedence_falls_back_to_system_default() {
    let dataset_uuid = Uuid::parse_str("550e8400-e29b-41d4-a716-446655440010").unwrap();
    let period_id = Uuid::parse_str("550e8400-e29b-41d4-a716-446655440200").unwrap();

    let project = create_precedence_project(dataset_uuid, None);
    let dataset = create_precedence_dataset(dataset_uuid, None);
    let resolvers = vec![
        create_precedence_resolver("dataset_resolver", "dataset_ds", false),
        create_precedence_resolver("system_default_resolver", "system_ds", true),
    ];

    let request = ResolutionRequest {
        dataset_id: dataset_uuid.to_string(),
        table_name: "fact_sales".to_string(),
        period_id,
        project_id: Some(project.id.to_string()),
    };

    let result = resolve_with_precedence(
        request,
        Some(project),
        Some(dataset),
        resolvers,
        create_test_calendar(),
        vec![create_test_quarter()],
    )
    .unwrap();

    assert_eq!(
        result.diagnostic.resolver_source,
        ResolverSource::SystemDefault
    );
    assert_eq!(result.diagnostic.resolver_id, "system_default_resolver");
    assert_eq!(result.locations[0].datasource_id, "system_ds");
}

#[test]
fn test_precedence_fails_when_no_project_dataset_or_default_resolver_exists() {
    let dataset_uuid = Uuid::parse_str("550e8400-e29b-41d4-a716-446655440010").unwrap();
    let period_id = Uuid::parse_str("550e8400-e29b-41d4-a716-446655440200").unwrap();

    let project = create_precedence_project(dataset_uuid, None);
    let dataset = create_precedence_dataset(dataset_uuid, Some("missing_dataset_resolver"));
    let resolvers = vec![create_precedence_resolver(
        "non_default_resolver",
        "non_default_ds",
        false,
    )];

    let request = ResolutionRequest {
        dataset_id: dataset_uuid.to_string(),
        table_name: "fact_sales".to_string(),
        period_id,
        project_id: Some(project.id.to_string()),
    };

    let result = resolve_with_precedence(
        request,
        Some(project),
        Some(dataset),
        resolvers,
        create_test_calendar(),
        vec![create_test_quarter()],
    );

    match result {
        Err(ResolutionError::ResolverSelectionFailed(reason)) => {
            assert!(reason.contains("no resolver available"));
        }
        other => panic!("expected ResolverSelectionFailed, got: {:?}", other),
    }
}

#[test]
fn test_period_expansion_fails_when_no_descendants_at_target_level() {
    let resolver = Resolver {
        id: "missing_descendant_resolver".to_string(),
        name: "Missing Descendant".to_string(),
        description: None,
        version: 1,
        status: ResolverStatus::Active,
        is_default: Some(false),
        rules: vec![ResolutionRule {
            name: "needs_months".to_string(),
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

    let request = ResolutionRequest {
        dataset_id: "test".to_string(),
        table_name: "data".to_string(),
        period_id: Uuid::parse_str("550e8400-e29b-41d4-a716-446655440200").unwrap(),
        project_id: None,
    };

    let result = resolve_with_source(
        request,
        resolver,
        create_test_calendar(),
        vec![create_test_quarter()],
        ResolverSource::DatasetReference,
    );

    match result {
        Err(ResolutionError::PeriodExpansionFailed { diagnostic, .. }) => {
            assert_eq!(
                diagnostic.outcome,
                DiagnosticOutcome::PeriodExpansionFailure
            );
        }
        other => panic!("expected PeriodExpansionFailed, got: {:?}", other),
    }
}

#[test]
fn test_boolean_literal_when_expression_matches() {
    let resolver = Resolver {
        id: "boolean_literal_resolver".to_string(),
        name: "Boolean Literal".to_string(),
        description: None,
        version: 1,
        status: ResolverStatus::Active,
        is_default: Some(false),
        rules: vec![ResolutionRule {
            name: "literal_true".to_string(),
            when_expression: Some("true".to_string()),
            data_level: "any".to_string(),
            strategy: ResolutionStrategy::Path {
                datasource_id: "s3".to_string(),
                path: "/data/{period_id}/{table_name}.parquet".to_string(),
            },
        }],
        created_at: None,
        updated_at: None,
    };

    let request = ResolutionRequest {
        dataset_id: "test_dataset".to_string(),
        table_name: "test_table".to_string(),
        period_id: Uuid::parse_str("550e8400-e29b-41d4-a716-446655440200").unwrap(),
        project_id: None,
    };

    let result = resolve_with_source(
        request,
        resolver,
        create_test_calendar(),
        vec![create_test_quarter()],
        ResolverSource::DatasetReference,
    )
    .unwrap();

    assert!(result.diagnostic.evaluated_rules[0].matched);
}

#[test]
fn test_boolean_literal_with_logical_expression_matches() {
    let resolver = Resolver {
        id: "boolean_logical_resolver".to_string(),
        name: "Boolean Logical".to_string(),
        description: None,
        version: 1,
        status: ResolverStatus::Active,
        is_default: Some(false),
        rules: vec![ResolutionRule {
            name: "literal_and_comparison".to_string(),
            when_expression: Some("true AND table == 'test_table'".to_string()),
            data_level: "any".to_string(),
            strategy: ResolutionStrategy::Path {
                datasource_id: "s3".to_string(),
                path: "/data/{period_id}/{table_name}.parquet".to_string(),
            },
        }],
        created_at: None,
        updated_at: None,
    };

    let request = ResolutionRequest {
        dataset_id: "test_dataset".to_string(),
        table_name: "test_table".to_string(),
        period_id: Uuid::parse_str("550e8400-e29b-41d4-a716-446655440200").unwrap(),
        project_id: None,
    };

    let result = resolve_with_source(
        request,
        resolver,
        create_test_calendar(),
        vec![create_test_quarter()],
        ResolverSource::DatasetReference,
    )
    .unwrap();

    assert!(result.diagnostic.evaluated_rules[0].matched);
}

#[test]
fn test_catalog_templates_render_endpoint_params_and_headers() {
    let resolver = Resolver {
        id: "catalog_render_resolver".to_string(),
        name: "Catalog Render".to_string(),
        description: None,
        version: 1,
        status: ResolverStatus::Active,
        is_default: Some(false),
        rules: vec![ResolutionRule {
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
                    "X-Period": "{period_id}"
                }),
            },
        }],
        created_at: None,
        updated_at: None,
    };

    let request = ResolutionRequest {
        dataset_id: "team/a".to_string(),
        table_name: "sales report".to_string(),
        period_id: Uuid::parse_str("550e8400-e29b-41d4-a716-446655440200").unwrap(),
        project_id: None,
    };

    let result = resolve_with_source(
        request,
        resolver,
        create_test_calendar(),
        vec![create_test_quarter()],
        ResolverSource::DatasetReference,
    )
    .unwrap();

    assert_eq!(
        result.locations[0].path,
        Some("https://catalog.example.com/team%2Fa/sales%20report".to_string())
    );
}

#[test]
fn test_catalog_headers_unknown_token_returns_template_error() {
    let resolver = Resolver {
        id: "catalog_error_resolver".to_string(),
        name: "Catalog Error".to_string(),
        description: None,
        version: 1,
        status: ResolverStatus::Active,
        is_default: Some(false),
        rules: vec![ResolutionRule {
            name: "catalog_rule".to_string(),
            when_expression: None,
            data_level: "any".to_string(),
            strategy: ResolutionStrategy::Catalog {
                endpoint: "https://catalog.example.com/{dataset_id}".to_string(),
                method: "GET".to_string(),
                auth: None,
                params: json!({}),
                headers: json!({ "X-Bad": "{unknown_token}" }),
            },
        }],
        created_at: None,
        updated_at: None,
    };

    let request = ResolutionRequest {
        dataset_id: "team/a".to_string(),
        table_name: "sales report".to_string(),
        period_id: Uuid::parse_str("550e8400-e29b-41d4-a716-446655440200").unwrap(),
        project_id: None,
    };

    match resolve_with_source(
        request,
        resolver,
        create_test_calendar(),
        vec![create_test_quarter()],
        ResolverSource::DatasetReference,
    ) {
        Err(ResolutionError::TemplateRenderFailed { reason, .. }) => {
            assert!(reason.contains("unknown token"));
        }
        other => panic!("expected TemplateRenderFailed, got: {:?}", other),
    }
}
