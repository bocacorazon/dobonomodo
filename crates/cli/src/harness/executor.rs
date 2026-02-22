use anyhow::{bail, Context, Result};
use chrono::Utc;
use dobo_core::engine::io_traits::DataLoader;
use dobo_core::model::metadata_store::MetadataStore;
use dobo_core::model::{
    DataBlock, Dataset, ErrorType, LookupTarget, Project, ProjectDef, TableRef, TemporalMode,
    TestErrorDetail, TestResult, TestScenario, TestStatus,
};
use polars::prelude::*;
use std::collections::HashMap;
use std::path::{Path, PathBuf};
use test_resolver::{inject_system_metadata_for_mode, InMemoryDataLoader, InMemoryMetadataStore};
use uuid::Uuid;
use walkdir::WalkDir;

use super::comparator::{compare_output, validate_trace_events};

/// Execute a single test scenario
#[allow(dead_code)]
pub fn execute_scenario(scenario: &TestScenario) -> Result<TestResult> {
    execute_scenario_with_base_dir(scenario, None)
}

/// Execute a single test scenario with scenario-relative file resolution.
pub fn execute_scenario_with_base_dir(
    scenario: &TestScenario,
    scenario_base_dir: Option<&Path>,
) -> Result<TestResult> {
    let metadata_store = InMemoryMetadataStore::new();
    execute_scenario_with_base_dir_and_store(scenario, scenario_base_dir, &metadata_store)
}

fn execute_scenario_with_base_dir_and_store(
    scenario: &TestScenario,
    scenario_base_dir: Option<&Path>,
    metadata_store: &impl MetadataStore,
) -> Result<TestResult> {
    // Build result
    let mut result = TestResult {
        scenario_name: scenario.name.clone(),
        status: TestStatus::Pass,
        warnings: Vec::new(),
        data_mismatches: Vec::new(),
        trace_mismatches: Vec::new(),
        error: None,
        actual_snapshot: None,
    };

    // Load input data
    let mut loader = InMemoryDataLoader::new();
    for (table_name, data_block) in &scenario.input.data {
        match load_data_block(
            data_block,
            table_name,
            &scenario.input.dataset,
            scenario_base_dir,
            true,
        ) {
            Ok(Some(frame)) => {
                loader.add_table(table_name.clone(), frame);
            }
            Ok(None) => continue,
            Err(e) => {
                result.status = TestStatus::Error;
                result.error = Some(TestErrorDetail {
                    error_type: ErrorType::FileNotFound,
                    message: e.to_string(),
                    details: Some(format!("{:?}", e)),
                });
                return Ok(result);
            }
        }
    }

    // Execute pipeline (currently passthrough mock)
    let project = match resolve_project(
        &scenario.project,
        &scenario.input.dataset,
        metadata_store,
        &mut result,
    ) {
        Ok(project) => project,
        Err(e) => {
            result.status = TestStatus::Error;
            result.error = Some(TestErrorDetail {
                error_type: ErrorType::ProjectNotFound,
                message: e.to_string(),
                details: Some(format!("{:?}", e)),
            });
            return Ok(result);
        }
    };

    let actual_df = match execute_pipeline_mock(&scenario.input.dataset, &project, &loader) {
        Ok(df) => df,
        Err(e) => {
            result.status = TestStatus::Error;
            result.error = Some(TestErrorDetail {
                error_type: ErrorType::ExecutionError,
                message: e.to_string(),
                details: Some(format!("{:?}", e)),
            });
            return Ok(result);
        }
    };

    // Load expected output
    let expected_df = match load_data_block(
        &scenario.expected_output.data,
        "output",
        &scenario.input.dataset,
        scenario_base_dir,
        false,
    ) {
        Ok(Some(frame)) => match frame.collect() {
            Ok(collected) => collected,
            Err(e) => {
                result.status = TestStatus::Error;
                result.error = Some(TestErrorDetail {
                    error_type: ErrorType::SchemaValidationError,
                    message: e.to_string(),
                    details: Some(format!("{:?}", e)),
                });
                return Ok(result);
            }
        },
        Err(e) => {
            result.status = TestStatus::Error;
            result.error = Some(TestErrorDetail {
                error_type: ErrorType::FileNotFound,
                message: e.to_string(),
                details: Some(format!("{:?}", e)),
            });
            return Ok(result);
        }
        Ok(None) => {
            result.status = TestStatus::Error;
            result.error = Some(TestErrorDetail {
                error_type: ErrorType::SchemaValidationError,
                message: "Expected output is empty".to_string(),
                details: None,
            });
            return Ok(result);
        }
    };

    // Compare output
    let mismatches = match compare_output(
        &actual_df,
        &expected_df,
        scenario.config.match_mode,
        scenario.config.validate_metadata,
        scenario.config.order_sensitive,
    ) {
        Ok(mismatches) => mismatches,
        Err(e) => {
            let message = e.to_string();
            let error_type = if message.contains("Schema mismatch") {
                ErrorType::SchemaValidationError
            } else {
                ErrorType::ExecutionError
            };

            result.status = TestStatus::Error;
            result.error = Some(TestErrorDetail {
                error_type,
                message,
                details: Some(format!("{:?}", e)),
            });
            return Ok(result);
        }
    };

    if !mismatches.is_empty() {
        result.status = TestStatus::Fail;
        result.data_mismatches = mismatches;

        // Save snapshot if configured
        if scenario.config.snapshot_on_failure {
            result.actual_snapshot = Some(dataframe_to_datablock(&actual_df)?);
        }
    }

    // T078: Integrate validate_trace_events() when validate_traceability is true
    if scenario.config.validate_traceability {
        // For now, we use an empty trace since the mock executor doesn't generate traces
        // This will be replaced when the real pipeline executor is integrated
        let actual_trace: Vec<serde_json::Value> = vec![];

        // T079: Add trace mismatches to TestResult
        let trace_mismatches = validate_trace_events(&actual_trace, &scenario.expected_trace)?;

        if !trace_mismatches.is_empty() {
            result.status = TestStatus::Fail;
            result.trace_mismatches = trace_mismatches;
        }
    }

    Ok(result)
}

/// Load data from a DataBlock
fn load_data_block(
    block: &DataBlock,
    table_name: &str,
    dataset: &Dataset,
    scenario_base_dir: Option<&Path>,
    inject_metadata: bool,
) -> Result<Option<LazyFrame>> {
    let temporal_mode = table_temporal_mode(dataset, table_name);

    if let Some(rows) = &block.rows {
        let frame = if inject_metadata {
            let enriched_rows = inject_system_metadata_for_mode(
                rows.clone(),
                table_name,
                temporal_mode,
                dataset.id,
            )?;
            InMemoryDataLoader::build_lazyframe(enriched_rows)?
        } else {
            InMemoryDataLoader::build_lazyframe(rows.clone())?
        };

        Ok(Some(frame))
    } else if let Some(file_path) = &block.file {
        let resolved = resolve_data_file_path(file_path, scenario_base_dir);
        if !resolved.is_file() {
            anyhow::bail!("Data file not found or unreadable: {}", resolved.display());
        }

        let extension = resolved
            .extension()
            .and_then(|value| value.to_str())
            .unwrap_or_default()
            .to_ascii_lowercase();

        let frame = match extension.as_str() {
            "csv" => InMemoryDataLoader::load_csv(&resolved)?,
            "parquet" | "parq" => InMemoryDataLoader::load_parquet(&resolved)?,
            _ => anyhow::bail!(
                "Unsupported data file format '{}' for '{}'",
                extension,
                resolved.display()
            ),
        };

        if inject_metadata {
            Ok(Some(inject_file_metadata_lazy(
                frame,
                table_name,
                temporal_mode,
                dataset.id.to_string(),
            )?))
        } else {
            Ok(Some(frame))
        }
    } else {
        Ok(None)
    }
}

fn resolve_data_file_path(file_path: &str, scenario_base_dir: Option<&Path>) -> PathBuf {
    let path = PathBuf::from(file_path);
    if path.is_absolute() {
        path
    } else if let Some(base_dir) = scenario_base_dir {
        base_dir.join(path)
    } else {
        path
    }
}

fn resolve_project(
    project_def: &ProjectDef,
    dataset: &Dataset,
    metadata_store: &impl MetadataStore,
    result: &mut TestResult,
) -> Result<Project> {
    match project_def {
        ProjectDef::Inline(project) => {
            // T099: Check for version drift between project and input dataset
            if project.input_dataset_id != dataset.id {
                result.warnings.push(format!(
                    "Project input_dataset_id ({}) differs from scenario dataset id ({})",
                    project.input_dataset_id, dataset.id
                ));
            }
            if project.input_dataset_version != dataset.version {
                result.warnings.push(format!(
                    "Project input_dataset_version ({}) differs from scenario dataset version ({})",
                    project.input_dataset_version, dataset.version
                ));
            }
            Ok(project.as_ref().clone())
        }
        ProjectDef::Ref { id, version } => {
            let project = metadata_store
                .get_project(id)
                .with_context(|| format!("ProjectRef '{}' could not be resolved", id))?;

            if *version != project.version {
                result.warnings.push(format!(
                    "ProjectRef version drift detected for {}: requested v{}, current v{}",
                    id, version, project.version
                ));
            }

            Ok(project)
        }
    }
}

fn table_temporal_mode(dataset: &Dataset, table_name: &str) -> Option<TemporalMode> {
    if table_name == dataset.main_table.name {
        return dataset.main_table.temporal_mode.clone();
    }

    for lookup in &dataset.lookups {
        let alias_matches = lookup
            .alias
            .as_deref()
            .is_some_and(|alias| alias == table_name);

        if let LookupTarget::Table {
            name,
            temporal_mode,
            ..
        } = &lookup.target
        {
            if alias_matches || name == table_name {
                return temporal_mode.clone();
            }
        }
    }

    None
}

fn inject_file_metadata_lazy(
    frame: LazyFrame,
    table_name: &str,
    temporal_mode: Option<TemporalMode>,
    dataset_id: String,
) -> Result<LazyFrame> {
    if let Some(mode) = temporal_mode {
        validate_file_temporal_columns(&frame, table_name, &mode)?;
    }

    let now = Utc::now().to_rfc3339();
    let mut materialized = frame.collect()?;
    let row_count = materialized.height();

    let row_ids: Vec<String> = (0..row_count).map(|_| Uuid::now_v7().to_string()).collect();

    materialized.with_column(Series::new("_row_id".into(), row_ids))?;
    materialized.with_column(Series::new("_deleted".into(), vec![false; row_count]))?;
    materialized.with_column(Series::new(
        "_created_at".into(),
        vec![now.clone(); row_count],
    ))?;
    materialized.with_column(Series::new(
        "_updated_at".into(),
        vec![now.clone(); row_count],
    ))?;
    materialized.with_column(Series::new(
        "_source_dataset_id".into(),
        vec![dataset_id; row_count],
    ))?;
    materialized.with_column(Series::new(
        "_source_table".into(),
        vec![table_name.to_string(); row_count],
    ))?;

    Ok(materialized.lazy())
}

fn validate_file_temporal_columns(
    frame: &LazyFrame,
    table_name: &str,
    temporal_mode: &TemporalMode,
) -> Result<()> {
    let required_columns: &[&str] = match temporal_mode {
        TemporalMode::Period => &["_period"],
        TemporalMode::Bitemporal => &["_period_from", "_period_to"],
    };

    for required in required_columns {
        let has_nulls = frame
            .clone()
            .select([col(*required).is_null().any(true).alias("__has_missing")])
            .collect()
            .with_context(|| {
                format!(
                    "Table '{}' is missing required temporal column '{}'",
                    table_name, required
                )
            })?
            .column("__has_missing")?
            .bool()?
            .get(0)
            .unwrap_or(false);

        if has_nulls {
            bail!(
                "Table '{}' row data is missing required temporal column '{}'",
                table_name,
                required
            );
        }
    }

    Ok(())
}

/// Passthrough mock pipeline execution
/// Returns input data unchanged until real pipeline executor is available
fn execute_pipeline_mock(
    _dataset: &Dataset,
    _project: &Project,
    loader: &InMemoryDataLoader,
) -> Result<DataFrame> {
    // For passthrough, just return the first table's data
    // In real implementation, this would call core::engine::execute_pipeline

    // Get the table name from the dataset
    let table_name = &_dataset.main_table.name;

    // Load via the location/schema (simplified for mock)
    let location = dobo_core::model::ResolvedLocation {
        datasource_id: "test".to_string(),
        path: None,
        table: Some(table_name.to_string()),
        schema: None,
        period_identifier: None,
    };

    let table_ref = TableRef {
        name: table_name.to_string(),
        temporal_mode: _dataset.main_table.temporal_mode.clone(),
        columns: _dataset.main_table.columns.clone(),
    };

    let frame = loader.load(&location, &table_ref)?;
    Ok(frame.collect()?)
}

/// Convert DataFrame to DataBlock
fn dataframe_to_datablock(df: &DataFrame) -> Result<DataBlock> {
    Ok(DataBlock {
        rows: Some(dataframe_to_rows(df)),
        file: None,
    })
}

fn dataframe_to_rows(df: &DataFrame) -> Vec<HashMap<String, serde_json::Value>> {
    let mut rows = Vec::new();
    let height = df.height();
    let columns = df.get_columns();

    for row_idx in 0..height {
        let mut row = HashMap::new();
        for col in columns {
            let col_name = col.name().to_string();
            let value = column_value_at(col, row_idx);
            row.insert(col_name, value);
        }
        rows.push(row);
    }

    rows
}

/// Extract value from column at specific index
fn column_value_at(col: &Column, idx: usize) -> serde_json::Value {
    let series = col.as_materialized_series();

    if let Ok(ca) = series.bool() {
        ca.get(idx)
            .map(serde_json::Value::Bool)
            .unwrap_or(serde_json::Value::Null)
    } else if let Ok(ca) = series.i64() {
        ca.get(idx)
            .map(serde_json::Value::from)
            .unwrap_or(serde_json::Value::Null)
    } else if let Ok(ca) = series.f64() {
        ca.get(idx)
            .map(serde_json::Value::from)
            .unwrap_or(serde_json::Value::Null)
    } else if let Ok(ca) = series.str() {
        ca.get(idx)
            .map(|s| serde_json::Value::String(s.to_string()))
            .unwrap_or(serde_json::Value::Null)
    } else {
        serde_json::Value::String(format!("{:?}", series.get(idx).unwrap()))
    }
}

/// Discover test scenarios in a directory
pub fn discover_scenarios(suite_path: &Path) -> Result<Vec<PathBuf>> {
    let mut scenarios = Vec::new();

    for entry in WalkDir::new(suite_path)
        .into_iter()
        .filter_map(|e| e.ok())
        .filter(|e| e.file_type().is_file())
    {
        let path = entry.path();
        if let Some(ext) = path.extension() {
            if ext == "yaml" || ext == "yml" {
                // Skip hidden files and underscore-prefixed files
                if let Some(file_name) = path.file_name().and_then(|n| n.to_str()) {
                    if !file_name.starts_with('.') && !file_name.starts_with('_') {
                        scenarios.push(path.to_path_buf());
                    }
                }
            }
        }
    }

    scenarios.sort();
    Ok(scenarios)
}

/// Execute a test suite
pub fn execute_suite(scenarios: &[PathBuf]) -> Result<dobo_core::model::SuiteResult> {
    use super::parser::parse_scenario;

    let mut results = Vec::new();
    let mut passed = 0;
    let mut failed = 0;
    let mut errors = 0;

    for scenario_path in scenarios {
        match parse_scenario(scenario_path) {
            Ok(scenario) => match execute_scenario_with_base_dir(&scenario, scenario_path.parent())
            {
                Ok(result) => {
                    match result.status {
                        TestStatus::Pass => passed += 1,
                        TestStatus::Fail => failed += 1,
                        TestStatus::Error => errors += 1,
                    }
                    results.push(result);
                }
                Err(e) => {
                    errors += 1;
                    results.push(TestResult {
                        scenario_name: scenario_path.display().to_string(),
                        status: TestStatus::Error,
                        warnings: Vec::new(),
                        data_mismatches: Vec::new(),
                        trace_mismatches: Vec::new(),
                        error: Some(TestErrorDetail {
                            error_type: ErrorType::ExecutionError,
                            message: e.to_string(),
                            details: Some(format!("{:?}", e)),
                        }),
                        actual_snapshot: None,
                    });
                }
            },
            Err(e) => {
                errors += 1;
                results.push(TestResult {
                    scenario_name: scenario_path.display().to_string(),
                    status: TestStatus::Error,
                    warnings: Vec::new(),
                    data_mismatches: Vec::new(),
                    trace_mismatches: Vec::new(),
                    error: Some(TestErrorDetail {
                        error_type: ErrorType::ParseError,
                        message: e.to_string(),
                        details: Some(format!("{:?}", e)),
                    }),
                    actual_snapshot: None,
                });
            }
        }
    }

    Ok(dobo_core::model::SuiteResult {
        total: scenarios.len(),
        passed,
        failed,
        errors,
        results,
    })
}

#[cfg(test)]
mod tests {
    use super::*;
    use chrono::DateTime;
    use dobo_core::model::{
        ColumnDef, ColumnType, DatasetStatus, JoinCondition, LookupDef, LookupTarget,
        Materialization, ProjectStatus, TableRef, TemporalMode, TestConfig, TestInput, TestOutput,
        Visibility,
    };
    use serde_json::json;
    use std::collections::BTreeMap;
    use std::collections::HashMap;
    use std::collections::HashSet;
    use std::fs;
    use std::io::Write;
    use tempfile::TempDir;
    use uuid::Uuid;

    fn sample_dataset() -> Dataset {
        Dataset {
            id: Uuid::now_v7(),
            name: "dataset".to_string(),
            description: None,
            owner: "owner".to_string(),
            version: 2,
            status: DatasetStatus::Active,
            resolver_id: None,
            main_table: TableRef {
                name: "orders".to_string(),
                temporal_mode: Some(TemporalMode::Period),
                columns: vec![
                    ColumnDef {
                        name: "id".to_string(),
                        column_type: ColumnType::Integer,
                        nullable: Some(false),
                        description: None,
                    },
                    ColumnDef {
                        name: "value".to_string(),
                        column_type: ColumnType::Integer,
                        nullable: Some(false),
                        description: None,
                    },
                ],
            },
            lookups: vec![],
            natural_key_columns: vec!["id".to_string()],
            created_at: None,
            updated_at: None,
        }
    }

    fn sample_inline_project(dataset: &Dataset) -> ProjectDef {
        ProjectDef::Inline(Box::new(Project {
            id: Uuid::now_v7(),
            name: "project".to_string(),
            description: None,
            owner: "owner".to_string(),
            version: 1,
            status: ProjectStatus::Active,
            visibility: Visibility::Private,
            input_dataset_id: dataset.id,
            input_dataset_version: dataset.version,
            materialization: Materialization::Eager,
            operations: vec![],
            selectors: BTreeMap::new(),
            resolver_overrides: BTreeMap::new(),
            conflict_report: None,
            created_at: None,
            updated_at: None,
        }))
    }

    fn sample_project(dataset: &Dataset, project_id: Uuid, version: i32) -> Project {
        Project {
            id: project_id,
            name: "project".to_string(),
            description: None,
            owner: "owner".to_string(),
            version,
            status: ProjectStatus::Active,
            visibility: Visibility::Private,
            input_dataset_id: dataset.id,
            input_dataset_version: dataset.version,
            materialization: Materialization::Eager,
            operations: vec![],
            selectors: BTreeMap::new(),
            resolver_overrides: BTreeMap::new(),
            conflict_report: None,
            created_at: None,
            updated_at: None,
        }
    }

    fn sample_scenario(dataset: Dataset) -> TestScenario {
        TestScenario {
            name: "scenario".to_string(),
            description: None,
            periods: vec![dobo_core::model::PeriodDef {
                identifier: "2026-01".to_string(),
                level: "month".to_string(),
                start_date: chrono::NaiveDate::from_ymd_opt(2026, 1, 1).unwrap(),
                end_date: chrono::NaiveDate::from_ymd_opt(2026, 1, 31).unwrap(),
            }],
            input: TestInput {
                data: HashMap::from([(
                    "orders".to_string(),
                    DataBlock {
                        rows: Some(vec![HashMap::from([
                            ("id".to_string(), json!(1)),
                            ("value".to_string(), json!(10)),
                            ("_period".to_string(), json!("2026-01")),
                        ])]),
                        file: None,
                    },
                )]),
                dataset: dataset.clone(),
            },
            project: sample_inline_project(&dataset),
            expected_output: TestOutput {
                data: DataBlock {
                    rows: Some(vec![HashMap::from([
                        ("id".to_string(), json!(1)),
                        ("value".to_string(), json!(10)),
                    ])]),
                    file: None,
                },
            },
            expected_trace: vec![],
            config: TestConfig::default(),
        }
    }

    #[test]
    fn resolves_data_file_relative_to_scenario_directory() {
        let temp = TempDir::new().unwrap();
        let file_path = temp.path().join("input.csv");
        let mut file = fs::File::create(&file_path).unwrap();
        writeln!(file, "id,value").unwrap();
        writeln!(file, "1,10").unwrap();

        let dataset = sample_dataset();
        let block = DataBlock {
            rows: None,
            file: Some("input.csv".to_string()),
        };

        let frame = load_data_block(&block, "orders", &dataset, Some(temp.path()), false)
            .unwrap()
            .unwrap()
            .collect()
            .unwrap();
        assert_eq!(frame.height(), 1);
    }

    #[test]
    fn rejects_file_input_missing_required_temporal_columns() {
        let temp = TempDir::new().unwrap();
        let file_path = temp.path().join("input.csv");
        let mut file = fs::File::create(&file_path).unwrap();
        writeln!(file, "id,value").unwrap();
        writeln!(file, "1,10").unwrap();

        let dataset = sample_dataset();
        let block = DataBlock {
            rows: None,
            file: Some("input.csv".to_string()),
        };

        let error = match load_data_block(&block, "orders", &dataset, Some(temp.path()), true) {
            Ok(_) => panic!("expected missing temporal column error"),
            Err(error) => error.to_string(),
        };
        assert!(error.contains("_period"));
    }

    #[test]
    fn file_input_metadata_injection_uses_unique_uuid_v7_row_ids() {
        let temp = TempDir::new().unwrap();
        let file_path = temp.path().join("input.csv");
        let mut file = fs::File::create(&file_path).unwrap();
        writeln!(file, "id,value,_period").unwrap();
        writeln!(file, "1,10,2026-01").unwrap();
        writeln!(file, "2,20,2026-01").unwrap();

        let dataset = sample_dataset();
        let block = DataBlock {
            rows: None,
            file: Some("input.csv".to_string()),
        };

        let frame = load_data_block(&block, "orders", &dataset, Some(temp.path()), true)
            .unwrap()
            .unwrap()
            .collect()
            .unwrap();

        let rows = dataframe_to_rows(&frame);
        assert_eq!(rows.len(), 2);

        let row_ids: Vec<String> = rows
            .iter()
            .map(|row| {
                row.get("_row_id")
                    .and_then(|value| value.as_str())
                    .unwrap()
                    .to_string()
            })
            .collect();

        let unique_row_ids: HashSet<String> = row_ids.iter().cloned().collect();
        assert_eq!(unique_row_ids.len(), rows.len());

        for row_id in row_ids {
            let parsed = Uuid::parse_str(&row_id).expect("_row_id should be UUID");
            assert_eq!(parsed.get_version_num(), 7);
        }

        let created_at = rows[0]
            .get("_created_at")
            .and_then(|value| value.as_str())
            .unwrap();
        let updated_at = rows[0]
            .get("_updated_at")
            .and_then(|value| value.as_str())
            .unwrap();

        DateTime::parse_from_rfc3339(created_at).expect("_created_at should be RFC3339");
        DateTime::parse_from_rfc3339(updated_at).expect("_updated_at should be RFC3339");
    }

    #[test]
    fn schema_mismatch_is_reported_as_error_envelope() {
        let dataset = sample_dataset();
        let mut scenario = sample_scenario(dataset);
        scenario.expected_output = TestOutput {
            data: DataBlock {
                rows: Some(vec![HashMap::from([
                    ("id".to_string(), json!(1)),
                    ("amount".to_string(), json!(10)),
                ])]),
                file: None,
            },
        };

        let result = execute_scenario_with_base_dir(&scenario, None).unwrap();
        assert_eq!(result.status, TestStatus::Error);
        assert_eq!(
            result.error.as_ref().map(|error| error.error_type),
            Some(ErrorType::SchemaValidationError)
        );
        assert!(result
            .error
            .as_ref()
            .map(|error| error.message.contains("Schema mismatch"))
            .unwrap_or(false));
    }

    #[test]
    fn validate_traceability_populates_trace_mismatches() {
        let dataset = sample_dataset();
        let mut scenario = sample_scenario(dataset);
        scenario.config.validate_traceability = true;
        scenario.expected_trace = vec![dobo_core::model::TraceAssertion {
            operation_order: 1,
            change_type: dobo_core::model::TraceChangeType::Created,
            row_match: HashMap::from([("id".to_string(), json!(1))]),
            expected_diff: None,
        }];

        let result = execute_scenario_with_base_dir(&scenario, None).unwrap();
        assert_eq!(result.status, TestStatus::Fail);
        assert_eq!(result.trace_mismatches.len(), 1);
    }

    #[test]
    fn project_ref_version_drift_adds_warning_but_executes() {
        let dataset = sample_dataset();
        let mut scenario = sample_scenario(dataset.clone());
        let project_id = Uuid::now_v7();
        let mut metadata_store = InMemoryMetadataStore::new();
        metadata_store.add_project(sample_project(&dataset, project_id, 3));

        scenario.project = ProjectDef::Ref {
            id: project_id,
            version: 1,
        };

        let result =
            execute_scenario_with_base_dir_and_store(&scenario, None, &metadata_store).unwrap();
        assert_eq!(result.status, TestStatus::Pass);
        assert_eq!(result.warnings.len(), 1);
        assert!(result.warnings[0].contains("version drift"));
    }

    #[test]
    fn project_ref_missing_project_returns_error() {
        let dataset = sample_dataset();
        let mut scenario = sample_scenario(dataset);
        scenario.project = ProjectDef::Ref {
            id: Uuid::now_v7(),
            version: 1,
        };

        let result = execute_scenario_with_base_dir(&scenario, None).unwrap();
        assert_eq!(result.status, TestStatus::Error);
        assert_eq!(
            result.error.as_ref().map(|e| e.error_type),
            Some(ErrorType::ProjectNotFound)
        );
    }

    #[test]
    fn lookup_table_without_temporal_mode_does_not_require_period_columns() {
        let mut dataset = sample_dataset();
        dataset.lookups = vec![LookupDef {
            alias: Some("products".to_string()),
            target: LookupTarget::Table {
                name: "products".to_string(),
                temporal_mode: None,
                columns: vec![ColumnDef {
                    name: "sku".to_string(),
                    column_type: ColumnType::String,
                    nullable: Some(false),
                    description: None,
                }],
            },
            join_conditions: vec![JoinCondition {
                source_column: "id".to_string(),
                target_column: "sku".to_string(),
            }],
        }];

        let block = DataBlock {
            rows: Some(vec![HashMap::from([("sku".to_string(), json!("P001"))])]),
            file: None,
        };

        let frame = load_data_block(&block, "products", &dataset, None, true)
            .unwrap()
            .unwrap()
            .collect()
            .unwrap();

        let column_names: Vec<String> = frame
            .get_column_names()
            .iter()
            .map(|name| name.to_string())
            .collect();
        assert!(column_names.contains(&"_row_id".to_string()));
        assert!(!column_names.contains(&"_period".to_string()));
    }
}
