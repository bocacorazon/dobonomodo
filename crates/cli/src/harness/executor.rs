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
use std::path::{Component, Path, PathBuf};
use test_resolver::errors::{InjectionError, LoaderError};
use test_resolver::{
    inject_system_metadata_for_mode, InMemoryDataLoader, InMemoryMetadataStore,
    InMemoryTraceWriter,
};
use uuid::Uuid;
use walkdir::WalkDir;

use super::comparator::{compare_output, validate_trace_events};

const PROJECT_FIXTURE_FILE: &str = "projects.yaml";

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
    let metadata_store = load_metadata_store_from_fixture(scenario_base_dir)?;
    execute_scenario_with_base_dir_and_store(scenario, scenario_base_dir, &metadata_store)
}

fn load_metadata_store_from_fixture(
    scenario_base_dir: Option<&Path>,
) -> Result<InMemoryMetadataStore> {
    let mut metadata_store = InMemoryMetadataStore::new();
    let Some(base_dir) = scenario_base_dir else {
        return Ok(metadata_store);
    };

    let fixture_path = base_dir.join(PROJECT_FIXTURE_FILE);
    if !fixture_path.is_file() {
        return Ok(metadata_store);
    }

    let fixture = std::fs::read_to_string(&fixture_path)
        .with_context(|| format!("Failed to read '{}'", fixture_path.display()))?;
    let projects: Vec<Project> = serde_yaml::from_str(&fixture)
        .with_context(|| format!("Failed to parse '{}'", fixture_path.display()))?;
    for project in projects {
        metadata_store.add_project(project);
    }

    Ok(metadata_store)
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
                    error_type: classify_data_block_error(&e),
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

    let trace_writer = InMemoryTraceWriter::new();
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
                error_type: classify_data_block_error(&e),
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

    if scenario.config.validate_traceability {
        let collected_trace_events = trace_writer.get_events();
        if collected_trace_events.is_empty() {
            if scenario.expected_trace.is_empty() {
                result.warnings.push(
                    "Trace validation requested, but no trace events were emitted by the current pipeline executor"
                        .to_string(),
                );
                return Ok(result);
            }

            result.status = TestStatus::Error;
            result.error = Some(TestErrorDetail {
                error_type: ErrorType::ExecutionError,
                message: "Trace validation is unavailable because the current pipeline executor did not emit trace events"
                    .to_string(),
                details: None,
            });
            return Ok(result);
        }

        let actual_trace = trace_events_to_json(&collected_trace_events);
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
    let source_table = table_name.to_string();
    let source_dataset_id = dataset_id;

    Ok(frame.map(
        move |mut dataframe| {
            let row_count = dataframe.height();
            let row_ids: Vec<String> = (0..row_count).map(|_| Uuid::now_v7().to_string()).collect();

            dataframe.with_column(Series::new("_row_id".into(), row_ids))?;
            dataframe.with_column(Series::new("_deleted".into(), vec![false; row_count]))?;
            dataframe.with_column(Series::new(
                "_created_at".into(),
                vec![now.clone(); row_count],
            ))?;
            dataframe.with_column(Series::new(
                "_updated_at".into(),
                vec![now.clone(); row_count],
            ))?;
            dataframe.with_column(Series::new(
                "_source_dataset_id".into(),
                vec![source_dataset_id.clone(); row_count],
            ))?;
            dataframe.with_column(Series::new(
                "_source_table".into(),
                vec![source_table.clone(); row_count],
            ))?;

            Ok(dataframe)
        },
        AllowedOptimizations::default(),
        None,
        Some("inject_file_metadata"),
    ))
}

fn validate_file_temporal_columns(
    frame: &LazyFrame,
    table_name: &str,
    temporal_mode: &TemporalMode,
) -> Result<()> {
    let schema = frame
        .clone()
        .collect_schema()
        .with_context(|| format!("Table '{}' schema could not be inferred", table_name))?;

    let required_columns: &[&str] = match temporal_mode {
        TemporalMode::Period => &["_period"],
        TemporalMode::Bitemporal => &["_period_from", "_period_to"],
    };

    for required in required_columns {
        if schema.get(required).is_none() {
            bail!(
                "Table '{}' is missing required temporal column '{}'",
                table_name,
                required
            );
        }
    }

    let temporal_values = frame
        .clone()
        .select(
            required_columns
                .iter()
                .map(|column| col(*column))
                .collect::<Vec<_>>(),
        )
        .collect()
        .with_context(|| format!("Table '{}' temporal values could not be read", table_name))?;

    for required in required_columns {
        let series = temporal_values
            .column(required)
            .with_context(|| {
                format!(
                    "Table '{}' temporal column '{}' could not be accessed",
                    table_name, required
                )
            })?
            .as_materialized_series();

        for row_idx in 0..series.len() {
            let value = series.get(row_idx).with_context(|| {
                format!(
                    "Table '{}' temporal column '{}' row {} could not be read",
                    table_name,
                    required,
                    row_idx + 1
                )
            })?;
            if matches!(value, AnyValue::Null) {
                bail!(
                    "Table '{}' has missing required temporal value in column '{}' at row {}",
                    table_name,
                    required,
                    row_idx + 1
                );
            }
        }

        if let Ok(strings) = series.str() {
            for (row_idx, value) in strings.into_iter().enumerate() {
                if value.is_some_and(|text| text.trim().is_empty()) {
                    bail!(
                        "Table '{}' has missing required temporal value in column '{}' at row {}",
                        table_name,
                        required,
                        row_idx + 1
                    );
                }
            }
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

fn trace_events_to_json(trace_events: &[dobo_core::trace::types::TraceEvent]) -> Vec<serde_json::Value> {
    trace_events
        .iter()
        .map(|event| {
            match serde_json::from_str::<serde_json::Value>(&event.message) {
                Ok(mut value @ serde_json::Value::Object(_)) => {
                    if value.get("operation_order").is_none() {
                        value["operation_order"] = serde_json::json!(event.operation_order as i32);
                    }
                    value
                }
                _ => serde_json::json!({
                    "operation_order": event.operation_order as i32,
                    "message": event.message,
                }),
            }
        })
        .collect()
}

fn classify_data_block_error(error: &anyhow::Error) -> ErrorType {
    if error.downcast_ref::<InjectionError>().is_some() {
        return ErrorType::SchemaValidationError;
    }

    if let Some(loader_error) = error.downcast_ref::<LoaderError>() {
        return match loader_error {
            LoaderError::TableNotFound { .. } | LoaderError::DataFrameBuild { .. } => {
                ErrorType::SchemaValidationError
            }
            LoaderError::CsvLoad { .. } | LoaderError::ParquetLoad { .. } => ErrorType::ExecutionError,
        };
    }

    let message = error.to_string();
    if message.contains("Data file not found or unreadable") || message.contains("No such file") {
        return ErrorType::FileNotFound;
    }
    if message.contains("Unsupported data file format")
        || message.contains("missing required temporal")
        || message.contains("Schema mismatch")
    {
        return ErrorType::SchemaValidationError;
    }

    ErrorType::ExecutionError
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
        .filter_entry(|entry| {
            let relative_path = entry
                .path()
                .strip_prefix(suite_path)
                .unwrap_or_else(|_| entry.path());
            !has_hidden_or_underscored_segment(relative_path)
        })
        .filter_map(|e| e.ok())
        .filter(|e| e.file_type().is_file())
    {
        let path = entry.path();
        if let Some(ext) = path.extension() {
            if ext == "yaml" || ext == "yml" {
                scenarios.push(path.to_path_buf());
            }
        }
    }

    scenarios.sort();
    Ok(scenarios)
}

fn has_hidden_or_underscored_segment(path: &Path) -> bool {
    path.components().any(|component| match component {
        Component::Normal(segment) => segment
            .to_str()
            .is_some_and(|value| value.starts_with('.') || value.starts_with('_')),
        _ => false,
    })
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
    fn validate_traceability_passes_when_trace_events_match() {
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
        assert_eq!(result.status, TestStatus::Error);
        assert_eq!(
            result.error.as_ref().map(|error| error.error_type),
            Some(ErrorType::ExecutionError)
        );
        assert!(result
            .error
            .as_ref()
            .map(|error| error.message.contains("did not emit trace events"))
            .unwrap_or(false));
    }

    #[test]
    fn validate_traceability_without_assertions_keeps_result_pass() {
        let dataset = sample_dataset();
        let mut scenario = sample_scenario(dataset);
        scenario.config.validate_traceability = true;
        scenario.expected_trace = vec![];

        let result = execute_scenario_with_base_dir(&scenario, None).unwrap();
        assert_eq!(result.status, TestStatus::Pass);
        assert!(result.trace_mismatches.is_empty());
        assert_eq!(result.warnings.len(), 1);
        assert!(result.warnings[0].contains("no trace events were emitted"));
    }

    #[test]
    fn missing_temporal_input_is_classified_as_schema_validation_error() {
        let mut scenario = sample_scenario(sample_dataset());
        scenario.input.data = HashMap::from([(
            "orders".to_string(),
            DataBlock {
                rows: Some(vec![HashMap::from([
                    ("id".to_string(), json!(1)),
                    ("value".to_string(), json!(10)),
                ])]),
                file: None,
            },
        )]);

        let result = execute_scenario_with_base_dir(&scenario, None).unwrap();
        assert_eq!(result.status, TestStatus::Error);
        assert_eq!(
            result.error.as_ref().map(|error| error.error_type),
            Some(ErrorType::SchemaValidationError)
        );
    }

    #[test]
    fn missing_input_file_is_classified_as_file_not_found() {
        let dataset = sample_dataset();
        let mut scenario = sample_scenario(dataset.clone());
        scenario.input.data = HashMap::from([(
            "orders".to_string(),
            DataBlock {
                rows: None,
                file: Some("missing.csv".to_string()),
            },
        )]);

        let result = execute_scenario_with_base_dir(&scenario, None).unwrap();
        assert_eq!(result.status, TestStatus::Error);
        assert_eq!(
            result.error.as_ref().map(|error| error.error_type),
            Some(ErrorType::FileNotFound)
        );
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
    fn project_ref_executes_via_base_dir_fixture_store() {
        let temp = TempDir::new().unwrap();
        let dataset = sample_dataset();
        let mut scenario = sample_scenario(dataset.clone());
        let project_id = Uuid::now_v7();
        scenario.project = ProjectDef::Ref {
            id: project_id,
            version: 1,
        };

        fs::write(
            temp.path().join(PROJECT_FIXTURE_FILE),
            format!(
                r#"
- id: "{project_id}"
  name: "fixture-project"
  owner: "owner"
  version: 3
  status: active
  visibility: private
  input_dataset_id: "{dataset_id}"
  input_dataset_version: {dataset_version}
  materialization: eager
  operations: []
  selectors: {{}}
  resolver_overrides: {{}}
"#,
                project_id = project_id,
                dataset_id = dataset.id,
                dataset_version = dataset.version
            ),
        )
        .unwrap();

        let result = execute_scenario_with_base_dir(&scenario, Some(temp.path())).unwrap();
        assert_eq!(result.status, TestStatus::Pass);
        assert_eq!(result.warnings.len(), 1);
        assert!(result.warnings[0].contains("version drift"));
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

    #[test]
    fn rejects_file_input_with_missing_temporal_values_in_csv() {
        let temp = TempDir::new().unwrap();
        let file_path = temp.path().join("input.csv");
        let mut file = fs::File::create(&file_path).unwrap();
        writeln!(file, "id,value,_period").unwrap();
        writeln!(file, "1,10,").unwrap();

        let dataset = sample_dataset();
        let block = DataBlock {
            rows: None,
            file: Some("input.csv".to_string()),
        };

        let error = match load_data_block(&block, "orders", &dataset, Some(temp.path()), true) {
            Ok(_) => panic!("expected missing temporal value error"),
            Err(error) => error.to_string(),
        };
        assert!(error.contains("missing required temporal value"));
        assert!(error.contains("_period"));
    }

    #[test]
    fn rejects_parquet_input_with_missing_temporal_values() {
        let temp = TempDir::new().unwrap();
        let file_path = temp.path().join("input.parquet");

        let mut dataframe = DataFrame::new(vec![
            Series::new("id".into(), vec![1i64, 2i64]).into(),
            Series::new("value".into(), vec![10i64, 20i64]).into(),
            Series::new("_period".into(), vec![Some("2026-01"), None]).into(),
        ])
        .unwrap();
        let mut file = fs::File::create(&file_path).unwrap();
        ParquetWriter::new(&mut file)
            .finish(&mut dataframe)
            .unwrap();

        let dataset = sample_dataset();
        let block = DataBlock {
            rows: None,
            file: Some("input.parquet".to_string()),
        };

        let error = match load_data_block(&block, "orders", &dataset, Some(temp.path()), true) {
            Ok(_) => panic!("expected missing temporal value error"),
            Err(error) => error.to_string(),
        };
        assert!(error.contains("missing required temporal value"));
        assert!(error.contains("_period"));
    }

    #[test]
    fn discover_scenarios_ignores_hidden_snapshot_directories() {
        let temp = TempDir::new().unwrap();
        let suite_dir = temp.path().join("suite");
        fs::create_dir_all(suite_dir.join(".snapshots")).unwrap();

        fs::write(suite_dir.join("valid.yaml"), "name: valid").unwrap();
        fs::write(
            suite_dir.join(".snapshots").join("fail-actual.yaml"),
            "this: should_not_be_discovered",
        )
        .unwrap();

        let scenarios = discover_scenarios(&suite_dir).unwrap();
        assert_eq!(scenarios.len(), 1);
        assert_eq!(
            scenarios[0].file_name().and_then(|name| name.to_str()),
            Some("valid.yaml")
        );
    }
}
