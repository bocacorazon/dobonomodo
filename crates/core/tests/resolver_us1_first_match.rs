// User Story 1: Resolve by First Matching Rule
// Integration tests for ordered rule evaluation and first-match semantics

use dobo_core::model::{
    Calendar, CalendarStatus, LevelDef, Period, PeriodStatus, ResolutionRule, ResolutionStrategy,
    Resolver, ResolverStatus,
};
use dobo_core::resolver::context::ResolutionRequest;
use dobo_core::resolver::diagnostics::ResolverSource;
use dobo_core::resolver::engine::resolve_with_source;
use uuid::Uuid;

fn create_fiscal_calendar() -> Calendar {
    Calendar {
        id: Uuid::parse_str("550e8400-e29b-41d4-a716-446655440001").unwrap(),
        name: "Fiscal Calendar".to_string(),
        description: None,
        status: CalendarStatus::Active,
        is_default: true,
        levels: vec![LevelDef {
            name: "quarter".to_string(),
            parent_level: None,
            identifier_pattern: Some(r"^\d{4}-Q[1-4]$".to_string()),
            date_rules: vec![],
        }],
        created_at: None,
        updated_at: None,
    }
}

#[test]
fn test_ordered_rule_evaluation() {
    // Rules should be evaluated in order from first to last
    let resolver = Resolver {
        id: "sales_resolver".to_string(),
        name: "Sales Resolver".to_string(),
        description: None,
        version: 1,
        status: ResolverStatus::Active,
        is_default: Some(false),
        rules: vec![
            ResolutionRule {
                name: "specific_rule".to_string(),
                when_expression: Some("table == 'sales'".to_string()),
                data_level: "quarter".to_string(),
                strategy: ResolutionStrategy::Path {
                    datasource_id: "specific_ds".to_string(),
                    path: "/specific/path.parquet".to_string(),
                },
            },
            ResolutionRule {
                name: "general_rule".to_string(),
                when_expression: None,
                data_level: "quarter".to_string(),
                strategy: ResolutionStrategy::Path {
                    datasource_id: "general_ds".to_string(),
                    path: "/general/path.parquet".to_string(),
                },
            },
        ],
        created_at: None,
        updated_at: None,
    };

    let request = ResolutionRequest {
        dataset_id: "test".to_string(),
        table_name: "sales".to_string(),
        period_id: Uuid::parse_str("550e8400-e29b-41d4-a716-446655440200").unwrap(),
        project_id: None,
    };

    let period = Period {
        id: Uuid::parse_str("550e8400-e29b-41d4-a716-446655440200").unwrap(),
        identifier: "2024-Q1".to_string(),
        name: "Q1".to_string(),
        description: None,
        calendar_id: Uuid::parse_str("550e8400-e29b-41d4-a716-446655440001").unwrap(),
        year: 2024,
        sequence: 1,
        start_date: "2024-01-01".to_string(),
        end_date: "2024-03-31".to_string(),
        status: PeriodStatus::Open,
        parent_id: None,
        created_at: None,
        updated_at: None,
    };

    let result = resolve_with_source(
        request,
        resolver,
        create_fiscal_calendar(),
        vec![period],
        ResolverSource::DatasetReference,
    );

    assert!(result.is_ok());
    let res = result.unwrap();

    // First rule should match
    assert!(res.diagnostic.evaluated_rules[0].matched);
    assert_eq!(res.locations[0].datasource_id, "specific_ds");

    // Second rule should be skipped
    assert!(!res.diagnostic.evaluated_rules[1].matched);
    assert!(res.diagnostic.evaluated_rules[1]
        .reason
        .contains("earlier rule already matched"));
}

#[test]
fn test_period_condition_match() {
    // Test when condition with period comparison
    let resolver = Resolver {
        id: "period_resolver".to_string(),
        name: "Period Resolver".to_string(),
        description: None,
        version: 1,
        status: ResolverStatus::Active,
        is_default: Some(false),
        rules: vec![ResolutionRule {
            name: "after_cutover".to_string(),
            when_expression: Some("period >= '2024-Q1'".to_string()),
            data_level: "quarter".to_string(),
            strategy: ResolutionStrategy::Path {
                datasource_id: "new_system".to_string(),
                path: "/new/data.parquet".to_string(),
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

    let period = Period {
        id: Uuid::parse_str("550e8400-e29b-41d4-a716-446655440200").unwrap(),
        identifier: "2024-Q2".to_string(), // After cutover
        name: "Q2".to_string(),
        description: None,
        calendar_id: Uuid::parse_str("550e8400-e29b-41d4-a716-446655440001").unwrap(),
        year: 2024,
        sequence: 2,
        start_date: "2024-04-01".to_string(),
        end_date: "2024-06-30".to_string(),
        status: PeriodStatus::Open,
        parent_id: None,
        created_at: None,
        updated_at: None,
    };

    let result = resolve_with_source(
        request,
        resolver,
        create_fiscal_calendar(),
        vec![period],
        ResolverSource::DatasetReference,
    );

    assert!(result.is_ok());
    let res = result.unwrap();

    assert!(res.diagnostic.evaluated_rules[0].matched);
    assert!(res.diagnostic.evaluated_rules[0]
        .reason
        .contains("evaluated to true"));
}

#[test]
fn test_table_condition_match() {
    // Test when condition with table name match
    let resolver = Resolver {
        id: "table_resolver".to_string(),
        name: "Table Resolver".to_string(),
        description: None,
        version: 1,
        status: ResolverStatus::Active,
        is_default: Some(false),
        rules: vec![ResolutionRule {
            name: "inventory_rule".to_string(),
            when_expression: Some("table == 'inventory'".to_string()),
            data_level: "quarter".to_string(),
            strategy: ResolutionStrategy::Table {
                datasource_id: "inventory_db".to_string(),
                table: "inv_data".to_string(),
                schema: None,
            },
        }],
        created_at: None,
        updated_at: None,
    };

    let request = ResolutionRequest {
        dataset_id: "warehouse".to_string(),
        table_name: "inventory".to_string(),
        period_id: Uuid::parse_str("550e8400-e29b-41d4-a716-446655440200").unwrap(),
        project_id: None,
    };

    let period = Period {
        id: Uuid::parse_str("550e8400-e29b-41d4-a716-446655440200").unwrap(),
        identifier: "2024-Q1".to_string(),
        name: "Q1".to_string(),
        description: None,
        calendar_id: Uuid::parse_str("550e8400-e29b-41d4-a716-446655440001").unwrap(),
        year: 2024,
        sequence: 1,
        start_date: "2024-01-01".to_string(),
        end_date: "2024-03-31".to_string(),
        status: PeriodStatus::Open,
        parent_id: None,
        created_at: None,
        updated_at: None,
    };

    let result = resolve_with_source(
        request,
        resolver,
        create_fiscal_calendar(),
        vec![period],
        ResolverSource::DatasetReference,
    );

    assert!(result.is_ok());
    let res = result.unwrap();

    assert!(res.diagnostic.evaluated_rules[0].matched);
    assert_eq!(res.locations[0].datasource_id, "inventory_db");
}

#[test]
fn test_first_match_wins() {
    // When multiple rules match, only first should be selected
    let resolver = Resolver {
        id: "multi_match_resolver".to_string(),
        name: "Multi Match".to_string(),
        description: None,
        version: 1,
        status: ResolverStatus::Active,
        is_default: Some(false),
        rules: vec![
            ResolutionRule {
                name: "rule1".to_string(),
                when_expression: Some("period >= '2024-Q1'".to_string()),
                data_level: "quarter".to_string(),
                strategy: ResolutionStrategy::Path {
                    datasource_id: "ds1".to_string(),
                    path: "/path1.parquet".to_string(),
                },
            },
            ResolutionRule {
                name: "rule2".to_string(),
                when_expression: Some("period >= '2023-Q1'".to_string()), // Also matches
                data_level: "quarter".to_string(),
                strategy: ResolutionStrategy::Path {
                    datasource_id: "ds2".to_string(),
                    path: "/path2.parquet".to_string(),
                },
            },
        ],
        created_at: None,
        updated_at: None,
    };

    let request = ResolutionRequest {
        dataset_id: "test".to_string(),
        table_name: "data".to_string(),
        period_id: Uuid::parse_str("550e8400-e29b-41d4-a716-446655440200").unwrap(),
        project_id: None,
    };

    let period = Period {
        id: Uuid::parse_str("550e8400-e29b-41d4-a716-446655440200").unwrap(),
        identifier: "2024-Q1".to_string(),
        name: "Q1".to_string(),
        description: None,
        calendar_id: Uuid::parse_str("550e8400-e29b-41d4-a716-446655440001").unwrap(),
        year: 2024,
        sequence: 1,
        start_date: "2024-01-01".to_string(),
        end_date: "2024-03-31".to_string(),
        status: PeriodStatus::Open,
        parent_id: None,
        created_at: None,
        updated_at: None,
    };

    let result = resolve_with_source(
        request,
        resolver,
        create_fiscal_calendar(),
        vec![period],
        ResolverSource::DatasetReference,
    );

    assert!(result.is_ok());
    let res = result.unwrap();

    // Only first rule should have matched
    assert!(res.diagnostic.evaluated_rules[0].matched);
    assert!(!res.diagnostic.evaluated_rules[1].matched);
    assert_eq!(res.locations[0].datasource_id, "ds1");
}

#[test]
fn test_catch_all_fallback() {
    // Catch-all rule matches when earlier rules don't
    let resolver = Resolver {
        id: "fallback_resolver".to_string(),
        name: "Fallback".to_string(),
        description: None,
        version: 1,
        status: ResolverStatus::Active,
        is_default: Some(false),
        rules: vec![
            ResolutionRule {
                name: "specific".to_string(),
                when_expression: Some("table == 'special'".to_string()),
                data_level: "quarter".to_string(),
                strategy: ResolutionStrategy::Path {
                    datasource_id: "special_ds".to_string(),
                    path: "/special/path.parquet".to_string(),
                },
            },
            ResolutionRule {
                name: "catch_all".to_string(),
                when_expression: None,
                data_level: "quarter".to_string(),
                strategy: ResolutionStrategy::Path {
                    datasource_id: "default_ds".to_string(),
                    path: "/default/path.parquet".to_string(),
                },
            },
        ],
        created_at: None,
        updated_at: None,
    };

    let request = ResolutionRequest {
        dataset_id: "test".to_string(),
        table_name: "normal".to_string(), // Does not match "special"
        period_id: Uuid::parse_str("550e8400-e29b-41d4-a716-446655440200").unwrap(),
        project_id: None,
    };

    let period = Period {
        id: Uuid::parse_str("550e8400-e29b-41d4-a716-446655440200").unwrap(),
        identifier: "2024-Q1".to_string(),
        name: "Q1".to_string(),
        description: None,
        calendar_id: Uuid::parse_str("550e8400-e29b-41d4-a716-446655440001").unwrap(),
        year: 2024,
        sequence: 1,
        start_date: "2024-01-01".to_string(),
        end_date: "2024-03-31".to_string(),
        status: PeriodStatus::Open,
        parent_id: None,
        created_at: None,
        updated_at: None,
    };

    let result = resolve_with_source(
        request,
        resolver,
        create_fiscal_calendar(),
        vec![period],
        ResolverSource::DatasetReference,
    );

    assert!(result.is_ok());
    let res = result.unwrap();

    // First rule should not match
    assert!(!res.diagnostic.evaluated_rules[0].matched);
    // Catch-all should match
    assert!(res.diagnostic.evaluated_rules[1].matched);
    assert_eq!(res.locations[0].datasource_id, "default_ds");
}
