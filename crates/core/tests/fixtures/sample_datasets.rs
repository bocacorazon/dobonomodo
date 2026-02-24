use dobo_core::model::{
    ColumnDef, ColumnType, Dataset, DatasetStatus, Period, PeriodStatus, TableRef, TemporalMode,
};
use polars::prelude::*;
use uuid::Uuid;

pub fn run_period_2026_01() -> Period {
    Period {
        id: Uuid::new_v4(),
        identifier: "2026-01".to_string(),
        name: "2026-01".to_string(),
        description: None,
        calendar_id: Uuid::new_v4(),
        year: 2026,
        sequence: 1,
        start_date: "2026-01-01".to_string(),
        end_date: "2026-01-31".to_string(),
        status: PeriodStatus::Open,
        parent_id: None,
        created_at: None,
        updated_at: None,
    }
}

fn column(name: &str, column_type: ColumnType) -> ColumnDef {
    ColumnDef {
        name: name.to_string(),
        column_type,
        nullable: Some(true),
        description: None,
    }
}

#[allow(dead_code)]
pub fn gl_dataset(id: Uuid) -> Dataset {
    Dataset {
        id,
        name: "gl_transactions".to_string(),
        description: None,
        owner: "tests".to_string(),
        version: 1,
        status: DatasetStatus::Active,
        resolver_id: Some("dataset-resolver".to_string()),
        main_table: TableRef {
            name: "gl".to_string(),
            temporal_mode: Some(TemporalMode::Period),
            columns: vec![
                column("journal_id", ColumnType::String),
                column("currency", ColumnType::String),
                column("customer_id", ColumnType::String),
                column("product_id", ColumnType::String),
                column("amount_local", ColumnType::Decimal),
                column("_period", ColumnType::String),
            ],
        },
        lookups: vec![],
        natural_key_columns: vec![],
        created_at: None,
        updated_at: None,
    }
}

pub fn exchange_rates_dataset(id: Uuid, version: i32) -> Dataset {
    Dataset {
        id,
        name: "exchange_rates".to_string(),
        description: None,
        owner: "tests".to_string(),
        version,
        status: DatasetStatus::Active,
        resolver_id: Some("fx-resolver".to_string()),
        main_table: TableRef {
            name: "exchange_rates".to_string(),
            temporal_mode: Some(TemporalMode::Bitemporal),
            columns: vec![
                column("from_currency", ColumnType::String),
                column("to_currency", ColumnType::String),
                column("rate", ColumnType::Decimal),
                column("rate_type", ColumnType::String),
                column("_period_from", ColumnType::Date),
                column("_period_to", ColumnType::Date),
            ],
        },
        lookups: vec![],
        natural_key_columns: vec![],
        created_at: None,
        updated_at: None,
    }
}

#[allow(dead_code)]
pub fn customers_dataset(id: Uuid) -> Dataset {
    Dataset {
        id,
        name: "customers".to_string(),
        description: None,
        owner: "tests".to_string(),
        version: 1,
        status: DatasetStatus::Active,
        resolver_id: Some("customers-resolver".to_string()),
        main_table: TableRef {
            name: "customers".to_string(),
            temporal_mode: Some(TemporalMode::Period),
            columns: vec![
                column("id", ColumnType::String),
                column("tier", ColumnType::String),
                column("_period", ColumnType::String),
            ],
        },
        lookups: vec![],
        natural_key_columns: vec![],
        created_at: None,
        updated_at: None,
    }
}

#[allow(dead_code)]
pub fn products_dataset(id: Uuid) -> Dataset {
    Dataset {
        id,
        name: "products".to_string(),
        description: None,
        owner: "tests".to_string(),
        version: 1,
        status: DatasetStatus::Active,
        resolver_id: Some("products-resolver".to_string()),
        main_table: TableRef {
            name: "products".to_string(),
            temporal_mode: Some(TemporalMode::Period),
            columns: vec![
                column("id", ColumnType::String),
                column("category", ColumnType::String),
                column("_period", ColumnType::String),
            ],
        },
        lookups: vec![],
        natural_key_columns: vec![],
        created_at: None,
        updated_at: None,
    }
}

pub fn gl_transactions_frame() -> DataFrame {
    df! {
        "journal_id" => &["JE-001", "JE-002", "JE-003", "JE-005"],
        "currency" => &["USD", "EUR", "GBP", "JPY"],
        "customer_id" => &["C1", "C2", "C3", "C2"],
        "product_id" => &["P1", "P1", "P2", "P3"],
        "amount_local" => &[15000.0_f64, 8500.0, 22000.0, 2_500_000.0],
        "_period" => &["2026-01", "2026-01", "2026-01", "2026-01"],
    }
    .expect("valid gl frame")
}

pub fn exchange_rates_frame() -> DataFrame {
    df! {
        "from_currency" => &["EUR", "EUR", "GBP", "GBP", "JPY", "JPY", "USD"],
        "to_currency" => &["USD", "USD", "USD", "USD", "USD", "USD", "USD"],
        "rate_type" => &["closing", "closing", "closing", "closing", "closing", "closing", "closing"],
        "_period_from" => &["2025-01-01", "2026-01-01", "2025-01-01", "2026-01-01", "2025-01-01", "2026-01-01", "2020-01-01"],
        "_period_to" => &[Some("2026-01-01"), None, Some("2026-01-01"), None, Some("2026-01-01"), None, None],
        "rate" => &[1.0850_f64, 1.0920, 1.2650, 1.2710, 0.00667, 0.00672, 1.0000],
    }
    .expect("valid fx frame")
}

pub fn customers_frame() -> DataFrame {
    df! {
        "id" => &["C1", "C2", "C3"],
        "tier" => &["silver", "gold", "bronze"],
        "_period" => &["2026-01", "2026-01", "2026-01"],
    }
    .expect("valid customers frame")
}

pub fn products_frame() -> DataFrame {
    df! {
        "id" => &["P1", "P2", "P3"],
        "category" => &["software", "hardware", "services"],
        "_period" => &["2026-01", "2026-01", "2026-01"],
    }
    .expect("valid products frame")
}
