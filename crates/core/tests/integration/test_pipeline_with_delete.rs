use std::collections::BTreeMap;
use std::sync::Mutex;

use anyhow::Result;
use dobo_core::model::{
    Materialization, OperationInstance, OperationKind, OutputDestination, Project, ProjectStatus,
    Visibility,
};
use dobo_core::{execute_pipeline, execute_pipeline_with_output_writer, OutputWriter};
use polars::prelude::{Column, DataFrame};
use serde_json::json;
use uuid::Uuid;

fn sample_project() -> Project {
    Project {
        id: Uuid::nil(),
        name: "delete-pipeline".to_string(),
        description: None,
        owner: "owner".to_string(),
        version: 1,
        status: ProjectStatus::Draft,
        visibility: Visibility::Private,
        input_dataset_id: Uuid::nil(),
        input_dataset_version: 1,
        materialization: Materialization::Eager,
        operations: vec![
            OperationInstance {
                order: 1,
                kind: OperationKind::Delete,
                alias: Some("delete-zero-amount".to_string()),
                parameters: json!({ "selector": "amount = 0" }),
            },
            OperationInstance {
                order: 2,
                kind: OperationKind::Update,
                alias: Some("noop-update".to_string()),
                parameters: json!({}),
            },
        ],
        selectors: BTreeMap::new(),
        resolver_overrides: BTreeMap::new(),
        conflict_report: None,
        created_at: None,
        updated_at: None,
    }
}

fn sample_frame() -> Result<DataFrame> {
    DataFrame::new(vec![
        Column::new("_row_id".into(), ["r1", "r2", "r3"].as_ref()),
        Column::new("_deleted".into(), [false, false, false].as_ref()),
        Column::new("_modified_at".into(), [1_i64, 1, 1].as_ref()),
        Column::new("amount".into(), [0_i64, 100, 200].as_ref()),
    ])
    .map_err(Into::into)
}

#[test]
fn test_deleted_rows_excluded_from_subsequent_operations() -> Result<()> {
    let project = sample_project();
    let frame = sample_frame()?;

    let result = execute_pipeline(&project, frame)?;
    let amounts = result.column("amount")?.i64()?;
    let amount_values: Vec<Option<i64>> = amounts.into_iter().collect();

    assert_eq!(result.height(), 2);
    assert_eq!(amount_values, vec![Some(100), Some(200)]);
    Ok(())
}

#[test]
fn test_pipeline_output_include_deleted_uses_full_working_set_and_writer() -> Result<()> {
    let project = Project {
        operations: vec![
            OperationInstance {
                order: 1,
                kind: OperationKind::Delete,
                alias: Some("delete-zero-amount".to_string()),
                parameters: json!({ "selector": "amount = 0" }),
            },
            OperationInstance {
                order: 2,
                kind: OperationKind::Output,
                alias: Some("output-all".to_string()),
                parameters: json!({
                    "destination": {
                        "destination_type": "memory",
                        "target": "delete-report"
                    },
                    "include_deleted": true
                }),
            },
        ],
        ..sample_project()
    };
    let frame = sample_frame()?;
    let writer = RecordingWriter::default();

    let result = execute_pipeline_with_output_writer(&project, frame, &writer)?;

    assert_eq!(result.height(), 3);
    let deleted = result.column("_deleted")?.bool()?;
    assert!(deleted.into_iter().any(|value| value == Some(true)));

    let writes = writer
        .writes
        .lock()
        .expect("recording writer mutex poisoned");
    assert_eq!(writes.len(), 1);
    assert_eq!(writes[0].0, 3);
    assert_eq!(writes[0].1, "memory");
    assert_eq!(writes[0].2.as_deref(), Some("delete-report"));
    Ok(())
}

#[test]
fn test_pipeline_rejects_unknown_selector_column_before_execution() -> Result<()> {
    let project = Project {
        operations: vec![OperationInstance {
            order: 1,
            kind: OperationKind::Delete,
            alias: Some("delete-missing-column".to_string()),
            parameters: json!({ "selector": "missing_col = 1" }),
        }],
        ..sample_project()
    };
    let frame = sample_frame()?;

    let err = execute_pipeline(&project, frame).expect_err("selector should be rejected");
    let message = err.to_string();
    assert!(message.contains("unknown column") || message.contains("selector"));
    Ok(())
}

#[derive(Default)]
struct RecordingWriter {
    writes: Mutex<Vec<(usize, String, Option<String>)>>,
}

impl OutputWriter for RecordingWriter {
    fn write(&self, frame: &DataFrame, destination: &OutputDestination) -> Result<()> {
        let mut writes = self.writes.lock().expect("recording writer mutex poisoned");
        writes.push((
            frame.height(),
            destination.destination_type.clone(),
            destination.target.clone(),
        ));
        Ok(())
    }
}
