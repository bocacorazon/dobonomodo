use dobo_core::engine::filter::{apply_filter, FilterContext};
use dobo_core::model::dataset::TemporalMode;
use dobo_core::model::period::{Period, PeriodStatus};
use polars::prelude::*;
use uuid::Uuid;

fn create_dummy_period(identifier: &str) -> Period {
    Period {
        id: Uuid::now_v7(),
        identifier: identifier.to_string(),
        name: "Test Period".to_string(),
        description: None,
        calendar_id: Uuid::now_v7(),
        year: 2024,
        sequence: 1,
        start_date: "2024-01-01".to_string(),
        end_date: "2024-02-01".to_string(),
        status: PeriodStatus::Open,
        parent_id: None,
        created_at: None,
        updated_at: None,
    }
}

#[test]
fn test_integration_period_filter() {
    let period = create_dummy_period("2024-01");
    // FilterContext needs public visibility of fields or constructor
    // I added FilterContext::new method.
    let context = FilterContext::new(period, TemporalMode::Period);

    let df = df!(
        "id" => &[1, 2],
        "_period" => &["2024-01", "2024-02"],
        "_deleted" => &[false, false]
    )
    .unwrap()
    .lazy();

    let result = apply_filter(df, &context).unwrap().collect().unwrap();
    assert_eq!(result.height(), 1);
    assert_eq!(result.column("id").unwrap().i32().unwrap().get(0), Some(1));
}
