use crate::errors::InjectionError;
use chrono::Utc;
use dobo_core::model::TemporalMode;
use serde_json::Value;
use std::collections::HashMap;
use uuid::Uuid;

type Result<T> = std::result::Result<T, InjectionError>;

/// Derived temporal metadata values for system-column injection.
#[derive(Debug, Clone, Default)]
pub struct TemporalMetadataValues {
    pub period: Option<String>,
    pub period_from: Option<String>,
    pub period_to: Option<String>,
}

/// Inject system metadata into rows
pub fn inject_system_metadata(
    rows: Vec<HashMap<String, Value>>,
    table_name: &str,
    temporal_mode: TemporalMode,
    dataset_id: Uuid,
) -> Result<Vec<HashMap<String, Value>>> {
    inject_system_metadata_for_mode(rows, table_name, Some(temporal_mode), dataset_id)
}

/// Inject system metadata into rows with optional temporal validation.
pub fn inject_system_metadata_for_mode(
    rows: Vec<HashMap<String, Value>>,
    table_name: &str,
    temporal_mode: Option<TemporalMode>,
    dataset_id: Uuid,
) -> Result<Vec<HashMap<String, Value>>> {
    inject_system_metadata_for_mode_with_temporal_values(
        rows,
        table_name,
        temporal_mode,
        None,
        dataset_id,
    )
}

/// Inject system metadata into rows with optional derived temporal values.
pub fn inject_system_metadata_for_mode_with_temporal_values(
    rows: Vec<HashMap<String, Value>>,
    table_name: &str,
    temporal_mode: Option<TemporalMode>,
    temporal_values: Option<&TemporalMetadataValues>,
    dataset_id: Uuid,
) -> Result<Vec<HashMap<String, Value>>> {
    let now = Utc::now();
    let now_str = now.to_rfc3339();
    let mut enriched = Vec::with_capacity(rows.len());

    for (row_index, mut row) in rows.into_iter().enumerate() {
        if let Some(mode) = temporal_mode.as_ref() {
            ensure_temporal_columns(&mut row, mode, temporal_values, row_index, table_name)?;
        }

        // System columns
        row.insert(
            "_row_id".to_string(),
            Value::String(Uuid::now_v7().to_string()),
        );
        row.insert("_deleted".to_string(), Value::Bool(false));
        row.insert("_created_at".to_string(), Value::String(now_str.clone()));
        row.insert("_updated_at".to_string(), Value::String(now_str.clone()));
        row.insert(
            "_source_dataset_id".to_string(),
            Value::String(dataset_id.to_string()),
        );
        row.insert(
            "_source_table".to_string(),
            Value::String(table_name.to_string()),
        );

        enriched.push(row);
    }

    Ok(enriched)
}

fn ensure_temporal_columns(
    row: &mut HashMap<String, Value>,
    temporal_mode: &TemporalMode,
    temporal_values: Option<&TemporalMetadataValues>,
    row_index: usize,
    table_name: &str,
) -> Result<()> {
    match temporal_mode {
        TemporalMode::Period => ensure_temporal_value(
            row,
            "_period",
            temporal_values.and_then(|values| values.period.as_deref()),
            row_index,
            table_name,
        )?,
        TemporalMode::Bitemporal => {
            ensure_temporal_value(
                row,
                "_period_from",
                temporal_values.and_then(|values| values.period_from.as_deref()),
                row_index,
                table_name,
            )?;
            ensure_temporal_value(
                row,
                "_period_to",
                temporal_values.and_then(|values| values.period_to.as_deref()),
                row_index,
                table_name,
            )?;
        }
    }

    Ok(())
}

fn ensure_temporal_value(
    row: &mut HashMap<String, Value>,
    key: &'static str,
    derived_value: Option<&str>,
    row_index: usize,
    table_name: &str,
) -> Result<()> {
    if has_non_null_value(row, key) {
        return Ok(());
    }

    if let Some(value) = derived_value {
        row.insert(key.to_string(), Value::String(value.to_string()));
        return Ok(());
    }

    Err(InjectionError::MissingTemporalColumn {
        table: table_name.to_string(),
        row_index,
        column: key,
    })
}

fn has_non_null_value(row: &HashMap<String, Value>, key: &str) -> bool {
    row.get(key).is_some_and(|value| !value.is_null())
}

/// Inject temporal metadata based on temporal mode
/// This is used when users provide temporal columns separately
pub fn inject_temporal_metadata(
    mut row: HashMap<String, Value>,
    temporal_mode: TemporalMode,
    period_id: Option<&str>,
    period_from: Option<&str>,
    period_to: Option<&str>,
) -> HashMap<String, Value> {
    match temporal_mode {
        TemporalMode::Period => {
            if let Some(period) = period_id {
                row.insert("_period".to_string(), Value::String(period.to_string()));
            }
        }
        TemporalMode::Bitemporal => {
            if let Some(from) = period_from {
                row.insert("_period_from".to_string(), Value::String(from.to_string()));
            }
            if let Some(to) = period_to {
                row.insert("_period_to".to_string(), Value::String(to.to_string()));
            }
        }
    }
    row
}

/// Strip system columns from DataFrame for comparison
/// Returns list of column names to keep
pub fn get_business_columns(all_columns: &[String], include_metadata: bool) -> Vec<String> {
    if include_metadata {
        // Keep all columns
        all_columns.to_vec()
    } else {
        // Filter out system columns (those starting with _)
        all_columns
            .iter()
            .filter(|col| !col.starts_with('_'))
            .cloned()
            .collect()
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    /// T102: Comprehensive unit tests for inject_system_metadata()
    #[test]
    fn test_inject_system_metadata_basic() {
        let dataset_id = Uuid::now_v7();
        let rows = vec![HashMap::from([
            ("id".to_string(), Value::Number(1.into())),
            ("value".to_string(), Value::String("test".to_string())),
            ("_period".to_string(), Value::String("2026-01".to_string())),
        ])];

        let result = inject_system_metadata(rows, "test_table", TemporalMode::Period, dataset_id)
            .expect("metadata injection should succeed");

        assert_eq!(result.len(), 1);
        let row = &result[0];

        // Check system columns are present
        assert!(row.contains_key("_row_id"));
        assert!(row.contains_key("_deleted"));
        assert!(row.contains_key("_created_at"));
        assert!(row.contains_key("_updated_at"));
        assert!(row.contains_key("_source_dataset_id"));
        assert!(row.contains_key("_source_table"));

        // Check system column values
        assert_eq!(row["_deleted"], Value::Bool(false));
        assert_eq!(
            row["_source_dataset_id"],
            Value::String(dataset_id.to_string())
        );
        assert_eq!(
            row["_source_table"],
            Value::String("test_table".to_string())
        );

        // Check business columns are preserved
        assert_eq!(row["id"], Value::Number(1.into()));
        assert_eq!(row["value"], Value::String("test".to_string()));
    }

    #[test]
    fn test_inject_system_metadata_uuid_v7_is_valid() {
        let dataset_id = Uuid::now_v7();
        let rows = vec![HashMap::from([
            ("id".to_string(), Value::Number(1.into())),
            ("_period".to_string(), Value::String("2026-01".to_string())),
        ])];

        let result = inject_system_metadata(rows, "test_table", TemporalMode::Period, dataset_id)
            .expect("metadata injection should succeed");

        let row = &result[0];
        let row_id_str = row["_row_id"].as_str().unwrap();
        let row_id = Uuid::parse_str(row_id_str).expect("Invalid UUID");

        // Verify it's a valid UUID v7
        assert_eq!(row_id.get_version_num(), 7);
    }

    #[test]
    fn test_inject_system_metadata_preserves_temporal_columns() {
        let dataset_id = Uuid::now_v7();
        let rows = vec![HashMap::from([
            ("id".to_string(), Value::Number(1.into())),
            ("_period".to_string(), Value::String("2026-01".to_string())),
        ])];

        let result = inject_system_metadata(rows, "test_table", TemporalMode::Period, dataset_id)
            .expect("metadata injection should succeed");

        let row = &result[0];

        // Temporal column should be preserved
        assert_eq!(row["_period"], Value::String("2026-01".to_string()));
    }

    #[test]
    fn test_inject_system_metadata_multiple_rows() {
        let dataset_id = Uuid::now_v7();
        let rows = vec![
            HashMap::from([
                ("id".to_string(), Value::Number(1.into())),
                ("_period".to_string(), Value::String("2026-01".to_string())),
            ]),
            HashMap::from([
                ("id".to_string(), Value::Number(2.into())),
                ("_period".to_string(), Value::String("2026-01".to_string())),
            ]),
            HashMap::from([
                ("id".to_string(), Value::Number(3.into())),
                ("_period".to_string(), Value::String("2026-01".to_string())),
            ]),
        ];

        let result = inject_system_metadata(rows, "test_table", TemporalMode::Period, dataset_id)
            .expect("metadata injection should succeed");

        assert_eq!(result.len(), 3);

        // Each row should have unique _row_id
        let row_ids: Vec<String> = result
            .iter()
            .map(|r| r["_row_id"].as_str().unwrap().to_string())
            .collect();

        assert_eq!(row_ids.len(), 3);
        assert_ne!(row_ids[0], row_ids[1]);
        assert_ne!(row_ids[1], row_ids[2]);
        assert_ne!(row_ids[0], row_ids[2]);
    }

    #[test]
    fn test_inject_temporal_metadata_period_mode() {
        let row = HashMap::from([("id".to_string(), Value::Number(1.into()))]);

        let result =
            inject_temporal_metadata(row, TemporalMode::Period, Some("2026-01"), None, None);

        assert_eq!(result["_period"], Value::String("2026-01".to_string()));
        assert!(!result.contains_key("_period_from"));
        assert!(!result.contains_key("_period_to"));
    }

    #[test]
    fn test_inject_temporal_metadata_bitemporal_mode() {
        let row = HashMap::from([("id".to_string(), Value::Number(1.into()))]);

        let result = inject_temporal_metadata(
            row,
            TemporalMode::Bitemporal,
            None,
            Some("2026-01-01"),
            Some("2026-01-31"),
        );

        assert_eq!(
            result["_period_from"],
            Value::String("2026-01-01".to_string())
        );
        assert_eq!(
            result["_period_to"],
            Value::String("2026-01-31".to_string())
        );
        assert!(!result.contains_key("_period"));
    }

    #[test]
    fn test_get_business_columns_excludes_system_columns() {
        let columns = vec![
            "id".to_string(),
            "_row_id".to_string(),
            "value".to_string(),
            "_created_at".to_string(),
            "amount".to_string(),
        ];

        let business_cols = get_business_columns(&columns, false);

        assert_eq!(business_cols.len(), 3);
        assert!(business_cols.contains(&"id".to_string()));
        assert!(business_cols.contains(&"value".to_string()));
        assert!(business_cols.contains(&"amount".to_string()));
        assert!(!business_cols.contains(&"_row_id".to_string()));
        assert!(!business_cols.contains(&"_created_at".to_string()));
    }

    #[test]
    fn test_get_business_columns_includes_all_when_requested() {
        let columns = vec![
            "id".to_string(),
            "_row_id".to_string(),
            "value".to_string(),
            "_created_at".to_string(),
        ];

        let all_cols = get_business_columns(&columns, true);

        assert_eq!(all_cols.len(), 4);
        assert_eq!(all_cols, columns);
    }

    #[test]
    fn test_inject_system_metadata_period_mode_requires_period_column() {
        let dataset_id = Uuid::now_v7();
        let rows = vec![HashMap::from([("id".to_string(), Value::Number(1.into()))])];

        let result = inject_system_metadata(rows, "test_table", TemporalMode::Period, dataset_id);
        assert!(result.is_err());
        assert!(result.unwrap_err().to_string().contains("_period"));
    }

    #[test]
    fn test_inject_system_metadata_bitemporal_mode_requires_range_columns() {
        let dataset_id = Uuid::now_v7();
        let rows = vec![HashMap::from([
            ("id".to_string(), Value::Number(1.into())),
            (
                "_period_from".to_string(),
                Value::String("2026-01-01".to_string()),
            ),
        ])];

        let result =
            inject_system_metadata(rows, "test_table", TemporalMode::Bitemporal, dataset_id);
        assert!(result.is_err());
        assert!(result.unwrap_err().to_string().contains("_period_to"));
    }

    #[test]
    fn test_inject_system_metadata_for_mode_non_temporal_skips_temporal_validation() {
        let dataset_id = Uuid::now_v7();
        let rows = vec![HashMap::from([("id".to_string(), Value::Number(1.into()))])];

        let result = inject_system_metadata_for_mode(rows, "test_table", None, dataset_id);
        assert!(result.is_ok());
    }

    #[test]
    fn test_inject_system_metadata_for_mode_injects_period_from_context() {
        let dataset_id = Uuid::now_v7();
        let rows = vec![HashMap::from([("id".to_string(), Value::Number(1.into()))])];
        let temporal_values = TemporalMetadataValues {
            period: Some("2026-01".to_string()),
            period_from: None,
            period_to: None,
        };

        let result = inject_system_metadata_for_mode_with_temporal_values(
            rows,
            "test_table",
            Some(TemporalMode::Period),
            Some(&temporal_values),
            dataset_id,
        )
        .expect("temporal metadata injection should succeed");

        assert_eq!(result[0]["_period"], Value::String("2026-01".to_string()));
    }

    #[test]
    fn test_inject_system_metadata_for_mode_injects_bitemporal_range_from_context() {
        let dataset_id = Uuid::now_v7();
        let rows = vec![HashMap::from([("id".to_string(), Value::Number(1.into()))])];
        let temporal_values = TemporalMetadataValues {
            period: None,
            period_from: Some("2026-01-01".to_string()),
            period_to: Some("2026-01-31".to_string()),
        };

        let result = inject_system_metadata_for_mode_with_temporal_values(
            rows,
            "test_table",
            Some(TemporalMode::Bitemporal),
            Some(&temporal_values),
            dataset_id,
        )
        .expect("bitemporal metadata injection should succeed");

        assert_eq!(
            result[0]["_period_from"],
            Value::String("2026-01-01".to_string())
        );
        assert_eq!(
            result[0]["_period_to"],
            Value::String("2026-01-31".to_string())
        );
    }
}
