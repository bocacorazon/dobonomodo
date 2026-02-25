use dobo_core::model::{Calendar, CalendarStatus, LevelDef, Period, PeriodStatus};
use dobo_core::resolver::expander::expand_period;
use uuid::Uuid;

fn create_calendar() -> Calendar {
    Calendar {
        id: Uuid::parse_str("550e8400-e29b-41d4-a716-446655440001").unwrap(),
        name: "Calendar".to_string(),
        description: None,
        status: CalendarStatus::Active,
        is_default: true,
        levels: vec![
            LevelDef {
                name: "quarter".to_string(),
                parent_level: None,
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

fn quarter(parent_id: Option<Uuid>) -> Period {
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
        parent_id,
        created_at: None,
        updated_at: None,
    }
}

#[test]
fn test_empty_children_returns_error() {
    let requested = quarter(None);
    let result = expand_period(&requested, "quarter", "month", &create_calendar(), &[requested]);
    assert!(result.is_err());
}

#[test]
fn test_cycle_detection_returns_error() {
    let requested = quarter(None);
    let q2 = Period {
        id: Uuid::parse_str("550e8400-e29b-41d4-a716-446655440201").unwrap(),
        identifier: "2024-Q2".to_string(),
        name: "Q2 2024".to_string(),
        description: None,
        calendar_id: Uuid::parse_str("550e8400-e29b-41d4-a716-446655440001").unwrap(),
        year: 2024,
        sequence: 2,
        start_date: "2024-04-01".to_string(),
        end_date: "2024-06-30".to_string(),
        status: PeriodStatus::Open,
        parent_id: Some(requested.id),
        created_at: None,
        updated_at: None,
    };
    let cycle_back = Period {
        parent_id: Some(q2.id),
        ..requested.clone()
    };

    let result = expand_period(
        &cycle_back,
        "quarter",
        "month",
        &create_calendar(),
        &[cycle_back, q2],
    );
    assert!(result.is_err());
    assert!(result.unwrap_err().contains("cycle"));
}

#[test]
fn test_missing_parent_link_returns_error() {
    let requested = quarter(None);
    let orphan_month = Period {
        id: Uuid::parse_str("550e8400-e29b-41d4-a716-446655440301").unwrap(),
        identifier: "2024-02".to_string(),
        name: "February 2024".to_string(),
        description: None,
        calendar_id: Uuid::parse_str("550e8400-e29b-41d4-a716-446655440001").unwrap(),
        year: 2024,
        sequence: 2,
        start_date: "2024-02-01".to_string(),
        end_date: "2024-02-29".to_string(),
        status: PeriodStatus::Open,
        parent_id: Some(Uuid::parse_str("550e8400-e29b-41d4-a716-446655449999").unwrap()),
        created_at: None,
        updated_at: None,
    };

    let result = expand_period(
        &requested,
        "quarter",
        "month",
        &create_calendar(),
        &[requested, orphan_month],
    );
    assert!(result.is_err());
}
