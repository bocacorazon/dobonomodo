use std::collections::BTreeMap;
use std::fs;
use std::path::PathBuf;

use anyhow::Result;
use dobo_core::execute_pipeline;
use dobo_core::model::{
    DeleteOperationParams, Materialization, OperationInstance, OperationKind,
    OutputOperationParams, Project, ProjectStatus, Visibility,
};
use dobo_core::operations::{delete::execute_delete, output::execute_output};
use polars::prelude::{Column, DataFrame, IntoLazy};
use serde::Deserialize;
use serde_json::json;
use uuid::Uuid;

#[derive(Debug, Deserialize)]
struct InputRow {
    _row_id: String,
    _deleted: bool,
    _modified_at: i64,
    amount: i64,
}

#[derive(Debug, Deserialize)]
struct SelectiveScenario {
    selectors: Option<BTreeMap<String, String>>,
    input_rows: Vec<InputRow>,
    delete_selector: String,
    expected_remaining_amounts: Vec<i64>,
}

#[derive(Debug, Deserialize)]
struct DeleteAllScenario {
    input_rows: Vec<InputRow>,
    expected_remaining_count: usize,
}

#[derive(Debug, Deserialize)]
struct OutputVisibilityScenario {
    input_rows: Vec<InputRow>,
    delete_selector: String,
    expected_default_count: usize,
    expected_include_deleted_count: usize,
}

#[test]
fn delete_selective_scenario_executes() -> Result<()> {
    let scenario: SelectiveScenario = read_scenario("delete_selective.yaml")?;
    let selectors = scenario.selectors.unwrap_or_default();

    let project = make_project(
        vec![
            OperationInstance {
                order: 1,
                kind: OperationKind::Delete,
                alias: Some("delete".to_string()),
                parameters: json!({ "selector": scenario.delete_selector }),
            },
            OperationInstance {
                order: 2,
                kind: OperationKind::Update,
                alias: Some("noop".to_string()),
                parameters: json!({}),
            },
        ],
        selectors,
    );

    let result = execute_pipeline(&project, frame_from_rows(&scenario.input_rows)?)?;
    let amount_values: Vec<i64> = result
        .column("amount")?
        .i64()?
        .into_iter()
        .flatten()
        .collect();

    assert_eq!(amount_values, scenario.expected_remaining_amounts);
    Ok(())
}

#[test]
fn delete_all_scenario_executes() -> Result<()> {
    let scenario: DeleteAllScenario = read_scenario("delete_all.yaml")?;
    let project = make_project(
        vec![
            OperationInstance {
                order: 1,
                kind: OperationKind::Delete,
                alias: Some("delete-all".to_string()),
                parameters: json!({}),
            },
            OperationInstance {
                order: 2,
                kind: OperationKind::Update,
                alias: Some("noop".to_string()),
                parameters: json!({}),
            },
        ],
        BTreeMap::new(),
    );

    let result = execute_pipeline(&project, frame_from_rows(&scenario.input_rows)?)?;
    assert_eq!(result.height(), scenario.expected_remaining_count);
    Ok(())
}

#[test]
fn delete_output_visibility_scenario_executes() -> Result<()> {
    let scenario: OutputVisibilityScenario = read_scenario("delete_output_visibility.yaml")?;
    let selectors = BTreeMap::new();

    let deleted = execute_delete(
        frame_from_rows(&scenario.input_rows)?.lazy(),
        &DeleteOperationParams {
            selector: Some(scenario.delete_selector),
        },
        &selectors,
    )?;

    let default_output = execute_output(
        deleted.clone(),
        &OutputOperationParams::default(),
        &selectors,
    )?
    .collect()?;
    let include_deleted_output = execute_output(
        deleted,
        &OutputOperationParams {
            include_deleted: true,
            ..OutputOperationParams::default()
        },
        &selectors,
    )?
    .collect()?;

    assert_eq!(default_output.height(), scenario.expected_default_count);
    assert_eq!(
        include_deleted_output.height(),
        scenario.expected_include_deleted_count
    );
    Ok(())
}

fn read_scenario<T: for<'de> Deserialize<'de>>(name: &str) -> Result<T> {
    let path = PathBuf::from(env!("CARGO_MANIFEST_DIR"))
        .join("tests")
        .join("scenarios")
        .join(name);
    let content = fs::read_to_string(path)?;
    serde_yaml::from_str(&content).map_err(Into::into)
}

fn frame_from_rows(rows: &[InputRow]) -> Result<DataFrame> {
    let row_ids: Vec<&str> = rows.iter().map(|row| row._row_id.as_str()).collect();
    let deleted: Vec<bool> = rows.iter().map(|row| row._deleted).collect();
    let modified_at: Vec<i64> = rows.iter().map(|row| row._modified_at).collect();
    let amount: Vec<i64> = rows.iter().map(|row| row.amount).collect();

    DataFrame::new(vec![
        Column::new("_row_id".into(), row_ids.as_slice()),
        Column::new("_deleted".into(), deleted.as_slice()),
        Column::new("_modified_at".into(), modified_at.as_slice()),
        Column::new("amount".into(), amount.as_slice()),
    ])
    .map_err(Into::into)
}

fn make_project(
    operations: Vec<OperationInstance>,
    selectors: BTreeMap<String, String>,
) -> Project {
    Project {
        id: Uuid::nil(),
        name: "scenario".to_string(),
        description: None,
        owner: "owner".to_string(),
        version: 1,
        status: ProjectStatus::Draft,
        visibility: Visibility::Private,
        input_dataset_id: Uuid::nil(),
        input_dataset_version: 1,
        materialization: Materialization::Eager,
        operations,
        selectors,
        resolver_overrides: BTreeMap::new(),
        conflict_report: None,
        created_at: None,
        updated_at: None,
    }
}
