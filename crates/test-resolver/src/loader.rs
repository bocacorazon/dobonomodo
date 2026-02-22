use crate::errors::LoaderError;
use anyhow::Result;
use dobo_core::engine::io_traits::DataLoader;
use dobo_core::model::{ResolvedLocation, TableRef};
use polars::prelude::*;
use std::collections::HashMap;
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
            let values: Vec<_> = rows.iter().map(|row| row.get(col_name)).collect();

            // Infer type from first non-null value
            let series = if let Some(first_value) = values.iter().find_map(|v| *v) {
                match first_value {
                    serde_json::Value::Bool(_) => {
                        let vals: Vec<Option<bool>> = values
                            .iter()
                            .map(|v| v.and_then(|val| val.as_bool()))
                            .collect();
                        Series::new(col_name.into(), vals)
                    }
                    serde_json::Value::Number(n) => {
                        if n.is_i64() {
                            let vals: Vec<Option<i64>> = values
                                .iter()
                                .map(|v| v.and_then(|val| val.as_i64()))
                                .collect();
                            Series::new(col_name.into(), vals)
                        } else {
                            let vals: Vec<Option<f64>> = values
                                .iter()
                                .map(|v| v.and_then(|val| val.as_f64()))
                                .collect();
                            Series::new(col_name.into(), vals)
                        }
                    }
                    serde_json::Value::String(_) => {
                        let vals: Vec<Option<&str>> = values
                            .iter()
                            .map(|v| v.and_then(|val| val.as_str()))
                            .collect();
                        Series::new(col_name.into(), vals)
                    }
                    _ => {
                        // Default to string representation
                        let vals: Vec<String> = values
                            .iter()
                            .map(|v| {
                                v.map(|val| val.to_string())
                                    .unwrap_or_else(|| "null".to_string())
                            })
                            .collect();
                        Series::new(col_name.into(), vals)
                    }
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
}

impl Default for InMemoryDataLoader {
    fn default() -> Self {
        Self::new()
    }
}

impl DataLoader for InMemoryDataLoader {
    fn load(&self, location: &ResolvedLocation, _schema: &TableRef) -> Result<LazyFrame> {
        let table_name = location.table.as_deref().unwrap_or("unknown");
        self.load_table(table_name).map_err(Into::into)
    }
}

#[cfg(test)]
mod tests {
    use super::*;
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
    fn test_in_memory_loader_missing_table() {
        let loader = InMemoryDataLoader::new();

        let location = ResolvedLocation {
            datasource_id: "test".to_string(),
            path: None,
            table: Some("missing_table".to_string()),
            schema: None,
            period_identifier: None,
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
