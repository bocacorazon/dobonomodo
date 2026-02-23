use crate::errors::LoaderError;
use dobo_core::engine::io_traits::{DataLoader, DataLoaderError};
use dobo_core::model::{ColumnType, ResolvedLocation, TableRef};
use polars::prelude::*;
use std::collections::{HashMap, HashSet};
use std::path::Path;

/// In-memory data loader for test scenarios
pub struct InMemoryDataLoader {
    data: HashMap<String, LazyFrame>,
}

impl InMemoryDataLoader {
    /// Create a new in-memory data loader
    pub fn new() -> Self {
        Self {
            data: HashMap::new(),
        }
    }

    /// Add data for a table
    pub fn add_table(&mut self, table_name: String, frame: LazyFrame) {
        self.data.insert(table_name, frame);
    }

    /// Build a LazyFrame from rows (HashMap format from YAML)
    pub fn build_lazyframe(
        rows: Vec<HashMap<String, serde_json::Value>>,
    ) -> std::result::Result<LazyFrame, LoaderError> {
        if rows.is_empty() {
            // Create empty DataFrame with no columns
            return Ok(DataFrame::default().lazy());
        }

        // Collect all unique column names
        let mut columns: Vec<String> = rows
            .iter()
            .flat_map(|row| row.keys().cloned())
            .collect::<std::collections::HashSet<_>>()
            .into_iter()
            .collect();
        columns.sort();

        // Build series for each column
        let mut series_vec = Vec::new();
        for col_name in &columns {
            let mut inferred_type: Option<InlineType> = None;
            for (row_index, row) in rows.iter().enumerate() {
                let Some(value) = row.get(col_name) else {
                    continue;
                };
                if value.is_null() {
                    continue;
                }

                let value_type = Self::infer_inline_type(value);
                if value_type == InlineType::Unsupported {
                    return Err(LoaderError::InlineUnsupportedValueType {
                        row_index,
                        column: col_name.clone(),
                        actual_type: Self::json_value_type_name(value),
                    });
                }

                inferred_type = match inferred_type {
                    None => Some(value_type),
                    Some(current) => match Self::merge_inline_types(current, value_type) {
                        Some(merged) => Some(merged),
                        None => {
                            return Err(LoaderError::InlineValueTypeMismatch {
                                row_index,
                                column: col_name.clone(),
                                expected_type: Self::inline_type_name(current),
                                actual_type: Self::json_value_type_name(value),
                            });
                        }
                    },
                };
            }

            let series = if let Some(inferred_type) = inferred_type {
                match inferred_type {
                    InlineType::Bool => {
                        let vals: std::result::Result<Vec<Option<bool>>, LoaderError> = rows
                            .iter()
                            .enumerate()
                            .map(|(idx, row)| {
                                Self::extract_typed_value::<bool>(
                                    row.get(col_name),
                                    idx,
                                    col_name,
                                    "boolean",
                                    |value| match value {
                                        serde_json::Value::Bool(boolean) => Some(*boolean),
                                        _ => None,
                                    },
                                )
                            })
                            .collect();
                        Series::new(col_name.into(), vals?)
                    }
                    InlineType::Int64 => {
                        let vals: std::result::Result<Vec<Option<i64>>, LoaderError> = rows
                            .iter()
                            .enumerate()
                            .map(|(idx, row)| {
                                Self::extract_typed_value::<i64>(
                                    row.get(col_name),
                                    idx,
                                    col_name,
                                    "integer",
                                    |value| match value {
                                        serde_json::Value::Number(number) => number.as_i64(),
                                        _ => None,
                                    },
                                )
                            })
                            .collect();
                        Series::new(col_name.into(), vals?)
                    }
                    InlineType::Float64 => {
                        let vals: std::result::Result<Vec<Option<f64>>, LoaderError> = rows
                            .iter()
                            .enumerate()
                            .map(|(idx, row)| {
                                Self::extract_typed_value::<f64>(
                                    row.get(col_name),
                                    idx,
                                    col_name,
                                    "number",
                                    |value| match value {
                                        serde_json::Value::Number(number) => number.as_f64(),
                                        _ => None,
                                    },
                                )
                            })
                            .collect();
                        Series::new(col_name.into(), vals?)
                    }
                    InlineType::String => {
                        let vals: std::result::Result<Vec<Option<&str>>, LoaderError> = rows
                            .iter()
                            .enumerate()
                            .map(|(idx, row)| {
                                Self::extract_typed_value::<&str>(
                                    row.get(col_name),
                                    idx,
                                    col_name,
                                    "string",
                                    |value| match value {
                                        serde_json::Value::String(string) => Some(string.as_str()),
                                        _ => None,
                                    },
                                )
                            })
                            .collect();
                        Series::new(col_name.into(), vals?)
                    }
                    InlineType::Unsupported => unreachable!("handled before match"),
                }
            } else {
                // All nulls - create string series
                Series::new(col_name.into(), vec![None::<&str>; rows.len()])
            };

            series_vec.push(series.into());
        }

        let df =
            DataFrame::new(series_vec).map_err(|source| LoaderError::DataFrameBuild { source })?;
        Ok(df.lazy())
    }

    /// Load data from CSV file
    pub fn load_csv(path: &Path) -> std::result::Result<LazyFrame, LoaderError> {
        LazyCsvReader::new(path)
            .finish()
            .map_err(|source| LoaderError::CsvLoad {
                path: path.to_path_buf(),
                source,
            })
    }

    /// Load data from Parquet file
    pub fn load_parquet(path: &Path) -> std::result::Result<LazyFrame, LoaderError> {
        LazyFrame::scan_parquet(path, Default::default()).map_err(|source| {
            LoaderError::ParquetLoad {
                path: path.to_path_buf(),
                source,
            }
        })
    }

    fn load_table(&self, table_name: &str) -> std::result::Result<LazyFrame, LoaderError> {
        self.data
            .get(table_name)
            .cloned()
            .ok_or_else(|| LoaderError::TableNotFound {
                table: table_name.to_string(),
                available: self.data.keys().cloned().collect(),
            })
    }

    fn enforce_schema(
        frame: LazyFrame,
        schema: &TableRef,
    ) -> std::result::Result<LazyFrame, LoaderError> {
        if schema.columns.is_empty() {
            return Ok(frame);
        }

        let collected_schema =
            frame
                .clone()
                .collect_schema()
                .map_err(|source| LoaderError::SchemaInspection {
                    table: schema.name.clone(),
                    source,
                })?;

        let actual_columns: HashSet<String> = collected_schema
            .iter_names()
            .map(|name| name.to_string())
            .collect();
        let expected_columns: HashSet<String> = schema
            .columns
            .iter()
            .map(|column| column.name.clone())
            .collect();

        let mut missing: Vec<String> = expected_columns
            .difference(&actual_columns)
            .cloned()
            .collect();
        missing.sort();
        if !missing.is_empty() {
            return Err(LoaderError::MissingColumns {
                table: schema.name.clone(),
                missing,
            });
        }

        let mut unexpected: Vec<String> = actual_columns
            .difference(&expected_columns)
            .filter(|name| !Self::is_allowed_extra_column(name))
            .cloned()
            .collect();
        unexpected.sort();
        if !unexpected.is_empty() {
            return Err(LoaderError::UnexpectedColumns {
                table: schema.name.clone(),
                unexpected,
            });
        }

        let cast_exprs: Vec<_> = schema
            .columns
            .iter()
            .map(|column| {
                col(&column.name)
                    .strict_cast(Self::polars_type(&column.column_type))
                    .alias(&column.name)
            })
            .collect();

        Ok(frame.with_columns(cast_exprs))
    }

    fn is_allowed_extra_column(name: &str) -> bool {
        name.starts_with('_')
    }

    fn polars_type(column_type: &ColumnType) -> DataType {
        match column_type {
            ColumnType::String => DataType::String,
            ColumnType::Integer => DataType::Int64,
            ColumnType::Decimal => DataType::Float64,
            ColumnType::Boolean => DataType::Boolean,
            ColumnType::Date => DataType::Date,
            ColumnType::Timestamp => DataType::Datetime(TimeUnit::Milliseconds, None),
        }
    }

    fn extract_typed_value<'a, T>(
        value: Option<&'a serde_json::Value>,
        row_index: usize,
        column: &str,
        expected_type: &'static str,
        extractor: impl FnOnce(&'a serde_json::Value) -> Option<T>,
    ) -> std::result::Result<Option<T>, LoaderError> {
        match value {
            None | Some(serde_json::Value::Null) => Ok(None),
            Some(non_null) => {
                extractor(non_null)
                    .map(Some)
                    .ok_or_else(|| LoaderError::InlineValueTypeMismatch {
                        row_index,
                        column: column.to_string(),
                        expected_type,
                        actual_type: Self::json_value_type_name(non_null),
                    })
            }
        }
    }

    fn infer_inline_type(value: &serde_json::Value) -> InlineType {
        match value {
            serde_json::Value::Bool(_) => InlineType::Bool,
            serde_json::Value::Number(number) => {
                if number.is_i64() {
                    InlineType::Int64
                } else {
                    InlineType::Float64
                }
            }
            serde_json::Value::String(_) => InlineType::String,
            _ => InlineType::Unsupported,
        }
    }

    fn merge_inline_types(current: InlineType, incoming: InlineType) -> Option<InlineType> {
        if current == incoming {
            return Some(current);
        }

        match (current, incoming) {
            (InlineType::Int64, InlineType::Float64) | (InlineType::Float64, InlineType::Int64) => {
                Some(InlineType::Float64)
            }
            _ => None,
        }
    }

    fn inline_type_name(value: InlineType) -> &'static str {
        match value {
            InlineType::Bool => "boolean",
            InlineType::Int64 => "integer",
            InlineType::Float64 => "number",
            InlineType::String => "string",
            InlineType::Unsupported => "unsupported",
        }
    }

    fn json_value_type_name(value: &serde_json::Value) -> &'static str {
        match value {
            serde_json::Value::Null => "null",
            serde_json::Value::Bool(_) => "boolean",
            serde_json::Value::Number(number) => {
                if number.is_i64() {
                    "integer"
                } else {
                    "number"
                }
            }
            serde_json::Value::String(_) => "string",
            serde_json::Value::Array(_) => "array",
            serde_json::Value::Object(_) => "object",
        }
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
enum InlineType {
    Bool,
    Int64,
    Float64,
    String,
    Unsupported,
}

impl Default for InMemoryDataLoader {
    fn default() -> Self {
        Self::new()
    }
}

impl DataLoader for InMemoryDataLoader {
    fn load(
        &self,
        location: &ResolvedLocation,
        schema: &TableRef,
    ) -> std::result::Result<LazyFrame, DataLoaderError> {
        let table_name = location
            .table
            .as_deref()
            .or((!schema.name.is_empty()).then_some(schema.name.as_str()))
            .unwrap_or("unknown");
        let frame = self
            .load_table(table_name)
            .map_err(|error| DataLoaderError::LoadFailed {
                message: error.to_string(),
            })?;
        Self::enforce_schema(frame, schema).map_err(|error| DataLoaderError::LoadFailed {
            message: error.to_string(),
        })
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use dobo_core::model::{ColumnDef, ColumnType};
    use serde_json::json;
    use std::fs;
    use std::io::Write;
    use tempfile::TempDir;

    #[test]
    fn test_build_lazyframe_from_rows() {
        let rows = vec![
            HashMap::from([
                ("id".to_string(), json!(1)),
                ("name".to_string(), json!("Alice")),
                ("value".to_string(), json!(100.5)),
            ]),
            HashMap::from([
                ("id".to_string(), json!(2)),
                ("name".to_string(), json!("Bob")),
                ("value".to_string(), json!(200.75)),
            ]),
        ];

        let result = InMemoryDataLoader::build_lazyframe(rows);
        assert!(result.is_ok());

        let df = result.unwrap().collect().unwrap();
        assert_eq!(df.height(), 2);
        assert_eq!(df.width(), 3);
    }

    #[test]
    fn test_build_lazyframe_handles_nulls() {
        let rows = vec![
            HashMap::from([
                ("id".to_string(), json!(1)),
                ("name".to_string(), json!("Alice")),
            ]),
            HashMap::from([("id".to_string(), json!(2))]), // name is missing
        ];

        let result = InMemoryDataLoader::build_lazyframe(rows);
        assert!(result.is_ok());

        let df = result.unwrap().collect().unwrap();
        assert_eq!(df.height(), 2);
    }

    #[test]
    fn test_build_lazyframe_rejects_inline_type_mismatch() {
        let rows = vec![
            HashMap::from([("value".to_string(), json!(1))]),
            HashMap::from([("value".to_string(), json!("invalid"))]),
        ];

        let error = match InMemoryDataLoader::build_lazyframe(rows) {
            Ok(_) => panic!("expected inline type mismatch"),
            Err(error) => error.to_string(),
        };
        assert!(error.contains("Inline data type mismatch"));
        assert!(error.contains("column 'value'"));
    }

    #[test]
    fn test_build_lazyframe_allows_mixed_integer_and_decimal_values() {
        let rows = vec![
            HashMap::from([("value".to_string(), json!(100))]),
            HashMap::from([("value".to_string(), json!(200.5))]),
        ];

        let dataframe = InMemoryDataLoader::build_lazyframe(rows)
            .unwrap()
            .collect()
            .unwrap();
        let column = dataframe.column("value").unwrap().as_materialized_series();
        assert_eq!(column.dtype(), &DataType::Float64);
    }

    #[test]
    fn test_build_lazyframe_rejects_inline_unsupported_type() {
        let rows = vec![HashMap::from([("value".to_string(), json!({"nested": 1}))])];

        let error = match InMemoryDataLoader::build_lazyframe(rows) {
            Ok(_) => panic!("expected unsupported inline value type"),
            Err(error) => error.to_string(),
        };
        assert!(error.contains("unsupported value type"));
        assert!(error.contains("object"));
    }

    #[test]
    fn test_build_lazyframe_empty_rows() {
        let rows: Vec<HashMap<String, serde_json::Value>> = vec![];

        let result = InMemoryDataLoader::build_lazyframe(rows);
        assert!(result.is_ok());

        let df = result.unwrap().collect().unwrap();
        assert_eq!(df.height(), 0);
    }

    #[test]
    fn test_in_memory_loader_add_and_load() {
        let mut loader = InMemoryDataLoader::new();
        let rows = vec![HashMap::from([("id".to_string(), json!(1))])];
        let frame = InMemoryDataLoader::build_lazyframe(rows).unwrap();

        loader.add_table("test_table".to_string(), frame);

        let location = ResolvedLocation {
            datasource_id: "test".to_string(),
            path: None,
            table: Some("test_table".to_string()),
            schema: None,
            period_identifier: None,
            catalog_response: None,
        };

        let table_ref = TableRef {
            name: "test_table".to_string(),
            temporal_mode: None,
            columns: vec![],
        };

        let result = loader.load(&location, &table_ref);
        assert!(result.is_ok());
    }

    #[test]
    fn test_in_memory_loader_falls_back_to_schema_name_when_location_table_missing() {
        let mut loader = InMemoryDataLoader::new();
        let rows = vec![HashMap::from([("id".to_string(), json!(1))])];
        let frame = InMemoryDataLoader::build_lazyframe(rows).unwrap();
        loader.add_table("orders".to_string(), frame);

        let location = ResolvedLocation {
            datasource_id: "test".to_string(),
            path: None,
            table: None,
            schema: None,
            period_identifier: None,
            catalog_response: None,
        };

        let schema = table_ref("orders", vec![("id", ColumnType::Integer)]);

        let result = loader.load(&location, &schema);
        assert!(result.is_ok());
    }

    #[test]
    fn test_in_memory_loader_missing_table() {
        let loader = InMemoryDataLoader::new();

        let location = ResolvedLocation {
            datasource_id: "test".to_string(),
            path: None,
            table: Some("missing_table".to_string()),
            schema: None,
            period_identifier: None,
            catalog_response: None,
        };

        let table_ref = TableRef {
            name: "missing_table".to_string(),
            temporal_mode: None,
            columns: vec![],
        };

        let result = loader.load(&location, &table_ref);
        assert!(result.is_err());
        if let Err(e) = result {
            assert!(e.to_string().contains("not found in test data"));
        }
    }

    fn table_ref(name: &str, columns: Vec<(&str, ColumnType)>) -> TableRef {
        TableRef {
            name: name.to_string(),
            temporal_mode: None,
            columns: columns
                .into_iter()
                .map(|(name, column_type)| ColumnDef {
                    name: name.to_string(),
                    column_type,
                    nullable: Some(false),
                    description: None,
                })
                .collect(),
        }
    }

    #[test]
    fn test_in_memory_loader_rejects_missing_required_columns() {
        let mut loader = InMemoryDataLoader::new();
        let rows = vec![HashMap::from([("id".to_string(), json!(1))])];
        let frame = InMemoryDataLoader::build_lazyframe(rows).unwrap();
        loader.add_table("orders".to_string(), frame);

        let location = ResolvedLocation {
            datasource_id: "test".to_string(),
            path: None,
            table: Some("orders".to_string()),
            schema: None,
            period_identifier: None,
            catalog_response: None,
        };

        let result = loader.load(
            &location,
            &table_ref(
                "orders",
                vec![("id", ColumnType::Integer), ("value", ColumnType::Integer)],
            ),
        );

        let error = match result {
            Ok(_) => panic!("expected missing column validation error"),
            Err(error) => error.to_string(),
        };
        assert!(error.contains("missing required columns"));
        assert!(error.contains("value"));
    }

    #[test]
    fn test_in_memory_loader_rejects_unexpected_non_system_columns() {
        let mut loader = InMemoryDataLoader::new();
        let rows = vec![HashMap::from([
            ("id".to_string(), json!(1)),
            ("rogue".to_string(), json!("x")),
        ])];
        let frame = InMemoryDataLoader::build_lazyframe(rows).unwrap();
        loader.add_table("orders".to_string(), frame);

        let location = ResolvedLocation {
            datasource_id: "test".to_string(),
            path: None,
            table: Some("orders".to_string()),
            schema: None,
            period_identifier: None,
            catalog_response: None,
        };

        let result = loader.load(
            &location,
            &table_ref("orders", vec![("id", ColumnType::Integer)]),
        );

        let error = match result {
            Ok(_) => panic!("expected unexpected column validation error"),
            Err(error) => error.to_string(),
        };
        assert!(error.contains("unexpected columns"));
        assert!(error.contains("rogue"));
    }

    #[test]
    fn test_in_memory_loader_rejects_type_mismatch() {
        let mut loader = InMemoryDataLoader::new();
        let rows = vec![HashMap::from([("id".to_string(), json!("not-an-integer"))])];
        let frame = InMemoryDataLoader::build_lazyframe(rows).unwrap();
        loader.add_table("orders".to_string(), frame);

        let location = ResolvedLocation {
            datasource_id: "test".to_string(),
            path: None,
            table: Some("orders".to_string()),
            schema: None,
            period_identifier: None,
            catalog_response: None,
        };

        let frame = loader
            .load(
                &location,
                &table_ref("orders", vec![("id", ColumnType::Integer)]),
            )
            .expect("schema checks should remain lazy until collect");

        let error = frame
            .collect()
            .expect_err("expected cast failure at collect boundary")
            .to_string();
        assert!(error.contains("conversion") || error.contains("cast") || error.contains("strict"));
    }

    #[test]
    fn test_in_memory_loader_accepts_lazy_cast_when_collect_succeeds() {
        let mut loader = InMemoryDataLoader::new();
        let rows = vec![HashMap::from([("id".to_string(), json!(1))])];
        let frame = InMemoryDataLoader::build_lazyframe(rows).unwrap();
        loader.add_table("orders".to_string(), frame);

        let location = ResolvedLocation {
            datasource_id: "test".to_string(),
            path: None,
            table: Some("orders".to_string()),
            schema: None,
            period_identifier: None,
            catalog_response: None,
        };

        let frame = loader
            .load(
                &location,
                &table_ref("orders", vec![("id", ColumnType::Integer)]),
            )
            .expect("load should succeed lazily");

        let collected = frame.collect().expect("collect should succeed");
        assert_eq!(collected.height(), 1);
        assert_eq!(collected.column("id").unwrap().dtype(), &DataType::Int64);
    }

    #[test]
    fn test_in_memory_loader_casts_date_and_timestamp_columns() {
        let mut loader = InMemoryDataLoader::new();
        let rows = vec![HashMap::from([
            ("id".to_string(), json!(1)),
            ("order_date".to_string(), json!("2026-01-15")),
            (
                "captured_at".to_string(),
                json!("2026-01-15T10:30:00"),
            ),
        ])];
        let frame = InMemoryDataLoader::build_lazyframe(rows).unwrap();
        loader.add_table("orders".to_string(), frame);

        let location = ResolvedLocation {
            datasource_id: "test".to_string(),
            path: None,
            table: Some("orders".to_string()),
            schema: None,
            period_identifier: None,
            catalog_response: None,
        };

        let frame = loader
            .load(
                &location,
                &table_ref(
                    "orders",
                    vec![
                        ("id", ColumnType::Integer),
                        ("order_date", ColumnType::Date),
                        ("captured_at", ColumnType::Timestamp),
                    ],
                ),
            )
            .expect("load should succeed lazily");

        let collected = frame.collect().expect("collect should succeed");
        assert_eq!(collected.column("order_date").unwrap().dtype(), &DataType::Date);
        assert_eq!(
            collected.column("captured_at").unwrap().dtype(),
            &DataType::Datetime(TimeUnit::Milliseconds, None)
        );
    }

    #[test]
    fn test_in_memory_loader_allows_system_metadata_columns() {
        let mut loader = InMemoryDataLoader::new();
        let rows = vec![HashMap::from([
            ("id".to_string(), json!(1)),
            ("_period".to_string(), json!("2026-01")),
        ])];
        let frame = InMemoryDataLoader::build_lazyframe(rows).unwrap();
        loader.add_table("orders".to_string(), frame);

        let location = ResolvedLocation {
            datasource_id: "test".to_string(),
            path: None,
            table: Some("orders".to_string()),
            schema: None,
            period_identifier: None,
            catalog_response: None,
        };

        let result = loader.load(
            &location,
            &table_ref("orders", vec![("id", ColumnType::Integer)]),
        );
        assert!(result.is_ok());
    }

    /// T104: Integration test for external CSV file loading
    #[test]
    fn test_load_csv_file_integration() {
        let temp_dir = TempDir::new().unwrap();
        let csv_path = temp_dir.path().join("test_data.csv");

        // Create a CSV file
        let mut file = fs::File::create(&csv_path).unwrap();
        writeln!(file, "id,name,value").unwrap();
        writeln!(file, "1,Alice,100.5").unwrap();
        writeln!(file, "2,Bob,200.75").unwrap();
        file.flush().unwrap();

        let result = InMemoryDataLoader::load_csv(&csv_path);
        assert!(result.is_ok());

        let df = result.unwrap().collect().unwrap();
        assert_eq!(df.height(), 2);
        assert_eq!(df.width(), 3);
    }

    /// T105: Integration test for external Parquet file loading
    #[test]
    fn test_load_parquet_file_integration() {
        let temp_dir = TempDir::new().unwrap();
        let parquet_path = temp_dir.path().join("test_data.parquet");

        // Create a simple DataFrame and save as Parquet
        use polars::prelude::*;
        let _df = df! {
            "id" => &[1i64, 2i64, 3i64],
            "name" => &["Alice", "Bob", "Charlie"],
            "value" => &[100.5, 200.75, 150.25],
        }
        .unwrap();

        let mut file = fs::File::create(&parquet_path).unwrap();
        let mut df = df! {
            "id" => &[1i64, 2i64, 3i64],
            "name" => &["Alice", "Bob", "Charlie"],
            "value" => &[100.5, 200.75, 150.25],
        }
        .unwrap();
        ParquetWriter::new(&mut file).finish(&mut df).unwrap();

        let result = InMemoryDataLoader::load_parquet(&parquet_path);
        assert!(result.is_ok());

        let loaded_df = result.unwrap().collect().unwrap();
        assert_eq!(loaded_df.height(), 3);
        assert_eq!(loaded_df.width(), 3);
    }
}
