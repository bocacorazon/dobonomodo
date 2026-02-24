// User Story 2: Expand Periods to Data Granularity
// Integration tests for period expansion using calendar hierarchy

use dobo_core::model::{
    Calendar, CalendarStatus, LevelDef, Period, PeriodStatus, ResolutionRule, ResolutionStrategy,
    Resolver, ResolverStatus,
};
use dobo_core::resolver::context::ResolutionRequest;
use dobo_core::resolver::diagnostics::ResolverSource;
use dobo_core::resolver::engine::resolve_with_source;
use uuid::Uuid;

fn create_hierarchy_calendar() -> Calendar {
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

fn create_cyclic_hierarchy_calendar() -> Calendar {
    Calendar {
        levels: vec![
            LevelDef {
                name: "year".to_string(),
                parent_level: None,
                identifier_pattern: Some(r"^\d{4}$".to_string()),
                date_rules: vec![],
            },
            LevelDef {
                name: "quarter".to_string(),
                parent_level: Some("month".to_string()),
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
        ..create_hierarchy_calendar()
    }
}

fn create_test_periods() -> Vec<Period> {
    let calendar_id = Uuid::parse_str("550e8400-e29b-41d4-a716-446655440001").unwrap();
    let year_id = Uuid::parse_str("550e8400-e29b-41d4-a716-446655440100").unwrap();
    let mut periods = vec![Period {
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
    }];

    let quarter_ids = [
        Uuid::parse_str("550e8400-e29b-41d4-a716-446655440200").unwrap(),
        Uuid::parse_str("550e8400-e29b-41d4-a716-446655440201").unwrap(),
        Uuid::parse_str("550e8400-e29b-41d4-a716-446655440202").unwrap(),
        Uuid::parse_str("550e8400-e29b-41d4-a716-446655440203").unwrap(),
    ];
    for (index, quarter_id) in quarter_ids.iter().enumerate() {
        let quarter_number = index + 1;
        periods.push(Period {
            id: *quarter_id,
            identifier: format!("2024-Q{}", quarter_number),
            name: format!("Q{} 2024", quarter_number),
            description: None,
            calendar_id,
            year: 2024,
            sequence: quarter_number as i32,
            start_date: "2024-01-01".to_string(),
            end_date: "2024-12-31".to_string(),
            status: PeriodStatus::Open,
            parent_id: Some(year_id),
            created_at: None,
            updated_at: None,
        });
    }

    for month in 1..=12 {
        let quarter_index = (month - 1) / 3;
        periods.push(Period {
            id: Uuid::parse_str(&format!(
                "550e8400-e29b-41d4-a716-44665544{:04}",
                299 + month
            ))
            .unwrap(),
            identifier: format!("2024-{:02}", month),
            name: format!("Month {:02} 2024", month),
            description: None,
            calendar_id,
            year: 2024,
            sequence: month,
            start_date: "2024-01-01".to_string(),
            end_date: "2024-01-31".to_string(),
            status: PeriodStatus::Open,
            parent_id: Some(quarter_ids[quarter_index as usize]),
            created_at: None,
            updated_at: None,
        });
    }

    periods
}

fn create_test_periods_with_duplicate_month_sequence_per_quarter() -> Vec<Period> {
    let mut periods = create_test_periods();
    for period in &mut periods {
        if period.identifier.starts_with("2024-")
            && period.identifier.len() == 7
            && !period.identifier.contains("-Q")
        {
            let month = period.identifier[5..7].parse::<i32>().unwrap();
            period.sequence = ((month - 1) % 3) + 1;
        }
    }
    periods
}

#[test]
fn test_year_to_month_expansion() {
    // Requesting year should expand to all 12 months
    let resolver = Resolver {
        id: "test_resolver".to_string(),
        name: "Test".to_string(),
        description: None,
        version: 1,
        status: ResolverStatus::Active,
        is_default: Some(false),
        rules: vec![ResolutionRule {
            name: "expand_rule".to_string(),
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
        period_id: Uuid::parse_str("550e8400-e29b-41d4-a716-446655440100").unwrap(), // Year
        project_id: None,
    };

    let result = resolve_with_source(
        request,
        resolver,
        create_hierarchy_calendar(),
        create_test_periods(),
        ResolverSource::DatasetReference,
    );

    assert!(result.is_ok());
    let res = result.unwrap();

    // Should expand to 12 months
    assert_eq!(res.locations.len(), 12);
    assert_eq!(
        res.locations[0].period_identifier,
        Some("2024-01".to_string())
    );
    assert_eq!(
        res.locations[1].period_identifier,
        Some("2024-02".to_string())
    );
    assert_eq!(
        res.locations[11].period_identifier,
        Some("2024-12".to_string())
    );
}

#[test]
fn test_quarter_to_month_expansion() {
    // Quarter to month (3 locations)
    let resolver = Resolver {
        id: "test_resolver".to_string(),
        name: "Test".to_string(),
        description: None,
        version: 1,
        status: ResolverStatus::Active,
        is_default: Some(false),
        rules: vec![ResolutionRule {
            name: "expand_rule".to_string(),
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
        period_id: Uuid::parse_str("550e8400-e29b-41d4-a716-446655440200").unwrap(), // Q1
        project_id: None,
    };

    let result = resolve_with_source(
        request,
        resolver,
        create_hierarchy_calendar(),
        create_test_periods(),
        ResolverSource::DatasetReference,
    );

    assert!(result.is_ok());
    let res = result.unwrap();

    assert_eq!(res.locations.len(), 3);
    assert_eq!(res.diagnostic.expanded_periods.len(), 3);
}

#[test]
fn test_same_level_no_expansion() {
    // Requesting month with data_level=month should not expand
    let resolver = Resolver {
        id: "test_resolver".to_string(),
        name: "Test".to_string(),
        description: None,
        version: 1,
        status: ResolverStatus::Active,
        is_default: Some(false),
        rules: vec![ResolutionRule {
            name: "no_expand".to_string(),
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
        period_id: Uuid::parse_str("550e8400-e29b-41d4-a716-446655440300").unwrap(), // Jan
        project_id: None,
    };

    let result = resolve_with_source(
        request,
        resolver,
        create_hierarchy_calendar(),
        create_test_periods(),
        ResolverSource::DatasetReference,
    );

    assert!(result.is_ok());
    let res = result.unwrap();

    // Should return single location
    assert_eq!(res.locations.len(), 1);
    assert_eq!(
        res.locations[0].period_identifier,
        Some("2024-01".to_string())
    );
}

#[test]
fn test_any_level_single_location() {
    // data_level="any" should not expand
    let resolver = Resolver {
        id: "test_resolver".to_string(),
        name: "Test".to_string(),
        description: None,
        version: 1,
        status: ResolverStatus::Active,
        is_default: Some(false),
        rules: vec![ResolutionRule {
            name: "any_level".to_string(),
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
        period_id: Uuid::parse_str("550e8400-e29b-41d4-a716-446655440200").unwrap(), // Q1
        project_id: None,
    };

    let result = resolve_with_source(
        request,
        resolver,
        create_hierarchy_calendar(),
        create_test_periods(),
        ResolverSource::DatasetReference,
    );

    assert!(result.is_ok());
    let res = result.unwrap();

    // Should return single location for Q1
    assert_eq!(res.locations.len(), 1);
    assert_eq!(
        res.locations[0].period_identifier,
        Some("2024-Q1".to_string())
    );
}

#[test]
fn test_invalid_hierarchy_expansion_fails() {
    // Requesting finer level than exists should fail
    let resolver = Resolver {
        id: "test_resolver".to_string(),
        name: "Test".to_string(),
        description: None,
        version: 1,
        status: ResolverStatus::Active,
        is_default: Some(false),
        rules: vec![ResolutionRule {
            name: "bad_level".to_string(),
            when_expression: None,
            data_level: "day".to_string(), // Does not exist in calendar
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
        create_hierarchy_calendar(),
        create_test_periods(),
        ResolverSource::DatasetReference,
    );

    // Should fail with expansion error
    assert!(result.is_err());
}

#[test]
fn test_cyclic_hierarchy_is_rejected_without_hanging() {
    let resolver = Resolver {
        id: "test_resolver".to_string(),
        name: "Test".to_string(),
        description: None,
        version: 1,
        status: ResolverStatus::Active,
        is_default: Some(false),
        rules: vec![ResolutionRule {
            name: "expand_to_month".to_string(),
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
        period_id: Uuid::parse_str("550e8400-e29b-41d4-a716-446655440100").unwrap(),
        project_id: None,
    };

    let result = resolve_with_source(
        request,
        resolver,
        create_cyclic_hierarchy_calendar(),
        create_test_periods(),
        ResolverSource::DatasetReference,
    );

    assert!(result.is_err());
}

#[test]
fn test_diagnostic_expanded_periods() {
    // Diagnostic should list all expanded periods
    let resolver = Resolver {
        id: "test_resolver".to_string(),
        name: "Test".to_string(),
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

    let request = ResolutionRequest {
        dataset_id: "test".to_string(),
        table_name: "data".to_string(),
        period_id: Uuid::parse_str("550e8400-e29b-41d4-a716-446655440200").unwrap(), // Q1
        project_id: None,
    };

    let result = resolve_with_source(
        request,
        resolver,
        create_hierarchy_calendar(),
        create_test_periods(),
        ResolverSource::DatasetReference,
    );

    assert!(result.is_ok());
    let res = result.unwrap();

    assert_eq!(
        res.diagnostic.expanded_periods,
        vec!["2024-01", "2024-02", "2024-03"]
    );
}

#[test]
fn test_year_to_month_expansion_with_duplicate_per_parent_sequence() {
    let resolver = Resolver {
        id: "test_resolver".to_string(),
        name: "Test".to_string(),
        description: None,
        version: 1,
        status: ResolverStatus::Active,
        is_default: Some(false),
        rules: vec![ResolutionRule {
            name: "expand_rule".to_string(),
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
        period_id: Uuid::parse_str("550e8400-e29b-41d4-a716-446655440100").unwrap(),
        project_id: None,
    };

    let result = resolve_with_source(
        request,
        resolver,
        create_hierarchy_calendar(),
        create_test_periods_with_duplicate_month_sequence_per_quarter(),
        ResolverSource::DatasetReference,
    );

    assert!(result.is_ok());
    let res = result.unwrap();

    let expanded: Vec<String> = res
        .locations
        .iter()
        .map(|location| location.period_identifier.clone().unwrap())
        .collect();
    assert_eq!(
        expanded,
        vec![
            "2024-01", "2024-02", "2024-03", "2024-04", "2024-05", "2024-06", "2024-07", "2024-08",
            "2024-09", "2024-10", "2024-11", "2024-12",
        ]
    );
}
