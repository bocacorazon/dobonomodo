use dobo_core::model::{Calendar, Period, Resolver};
use dobo_core::resolver::context::ResolutionRequest;
use dobo_core::resolver::diagnostics::ResolverSource;
use dobo_core::resolver::engine::resolve_with_source;
use serde::Deserialize;
use std::fs;
use std::path::PathBuf;
use uuid::Uuid;

#[derive(Debug, Deserialize)]
struct PeriodFixture {
    periods: Vec<Period>,
}

fn fixture_path(rel: &str) -> PathBuf {
    PathBuf::from(env!("CARGO_MANIFEST_DIR"))
        .join("tests")
        .join("fixtures")
        .join(rel)
}

#[test]
fn quickstart_examples_work_with_fixtures() {
    let calendar: Calendar = serde_yaml::from_str(
        &fs::read_to_string(fixture_path("calendars/fiscal_calendar.yaml")).unwrap(),
    )
    .unwrap();
    let resolver: Resolver = serde_yaml::from_str(
        &fs::read_to_string(fixture_path("resolvers/sales_resolver.yaml")).unwrap(),
    )
    .unwrap();
    let periods: PeriodFixture = serde_yaml::from_str(
        &fs::read_to_string(fixture_path("periods/test_periods.yaml")).unwrap(),
    )
    .unwrap();

    let request = ResolutionRequest {
        dataset_id: "sales_dataset".to_string(),
        table_name: "daily_transactions".to_string(),
        period_id: Uuid::parse_str("550e8400-e29b-41d4-a716-446655440200").unwrap(),
        project_id: None,
    };

    let result = resolve_with_source(
        request,
        resolver,
        calendar,
        periods.periods,
        ResolverSource::DatasetReference,
    )
    .unwrap();

    assert_eq!(result.locations.len(), 3);
    assert_eq!(
        result.diagnostic.expanded_periods,
        vec!["2024-01", "2024-02", "2024-03"]
    );
}
