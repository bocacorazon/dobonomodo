use criterion::{criterion_group, criterion_main, Criterion};
use dobo_core::model::{
    Calendar, CalendarStatus, LevelDef, Period, PeriodStatus, ResolutionRule, ResolutionStrategy,
    Resolver, ResolverStatus,
};
use dobo_core::resolver::context::ResolutionRequest;
use dobo_core::resolver::engine::resolve;
use uuid::Uuid;

fn benchmark_100_period_expansion(c: &mut Criterion) {
    let calendar_id = Uuid::parse_str("550e8400-e29b-41d4-a716-446655440001").unwrap();
    let root_id = Uuid::parse_str("550e8400-e29b-41d4-a716-446655440100").unwrap();

    let calendar = Calendar {
        id: calendar_id,
        name: "Bench Calendar".to_string(),
        description: None,
        status: CalendarStatus::Active,
        is_default: true,
        levels: vec![
            LevelDef {
                name: "root".to_string(),
                parent_level: None,
                identifier_pattern: Some(r"^ROOT$".to_string()),
                date_rules: vec![],
            },
            LevelDef {
                name: "leaf".to_string(),
                parent_level: Some("root".to_string()),
                identifier_pattern: Some(r"^L\d{3}$".to_string()),
                date_rules: vec![],
            },
        ],
        created_at: None,
        updated_at: None,
    };

    let mut periods = vec![Period {
        id: root_id,
        identifier: "ROOT".to_string(),
        name: "Root".to_string(),
        description: None,
        calendar_id,
        year: 2024,
        sequence: 0,
        start_date: "2024-01-01".to_string(),
        end_date: "2024-12-31".to_string(),
        status: PeriodStatus::Open,
        parent_id: None,
        created_at: None,
        updated_at: None,
    }];

    for i in 1..=100 {
        periods.push(Period {
            id: Uuid::parse_str(&format!("550e8400-e29b-41d4-a716-44665545{:04}", i)).unwrap(),
            identifier: format!("L{:03}", i),
            name: format!("Leaf {}", i),
            description: None,
            calendar_id,
            year: 2024,
            sequence: i,
            start_date: "2024-01-01".to_string(),
            end_date: "2024-01-01".to_string(),
            status: PeriodStatus::Open,
            parent_id: Some(root_id),
            created_at: None,
            updated_at: None,
        });
    }

    let resolver = Resolver {
        id: "bench_resolver".to_string(),
        name: "Benchmark Resolver".to_string(),
        description: None,
        version: 1,
        status: ResolverStatus::Active,
        is_default: Some(false),
        rules: vec![ResolutionRule {
            name: "expand_to_leaf".to_string(),
            when_expression: None,
            data_level: "leaf".to_string(),
            strategy: ResolutionStrategy::Path {
                datasource_id: "bench_ds".to_string(),
                path: "/data/{period_id}.parquet".to_string(),
            },
        }],
        created_at: None,
        updated_at: None,
    };

    let request = ResolutionRequest {
        dataset_id: "bench_dataset".to_string(),
        table_name: "bench_table".to_string(),
        period_id: root_id,
        project_id: None,
    };

    c.bench_function("resolver_expand_100_periods", |b| {
        b.iter(|| {
            let result = resolve(
                request.clone(),
                resolver.clone(),
                calendar.clone(),
                periods.clone(),
            )
            .unwrap();
            assert_eq!(result.locations.len(), 100);
        })
    });
}

criterion_group!(benches, benchmark_100_period_expansion);
criterion_main!(benches);
