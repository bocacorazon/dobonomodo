use std::collections::{BTreeMap, BTreeSet};

use anyhow::{Context, Result};
use polars::prelude::{col, lit, DataFrame, IntoLazy, LazyFrame};

use crate::engine::io_traits::{OutputWriter, OutputWriterError};
use crate::model::{
    DeleteOperationParams, OperationInstance, OperationKind, OutputOperationParams, Project,
};
use crate::operations::{delete::execute_delete, output::execute_output};
use crate::validation::{
    validate_named_selector_reference, validate_selector_boolean_type_with_schema,
};

pub fn execute_pipeline(project: &Project, input: DataFrame) -> Result<DataFrame> {
    execute_pipeline_with_output_writer(project, input, &NoopOutputWriter)
}

pub fn execute_pipeline_with_output_writer(
    project: &Project,
    input: DataFrame,
    output_writer: &dyn OutputWriter,
) -> Result<DataFrame> {
    let mut operations = project.operations.clone();
    operations.sort_by_key(|operation| operation.order);

    let available_columns = collect_available_columns(&input);
    validate_operation_selectors(&operations, &project.selectors, &available_columns)?;

    let mut working_full = input.lazy();
    let mut last_output: Option<DataFrame> = None;
    let mut should_return_active_only = false;

    for operation in operations {
        match operation.kind {
            OperationKind::Delete => {
                let params: DeleteOperationParams = serde_json::from_value(operation.parameters)
                    .with_context(|| {
                        format!(
                            "failed to deserialize delete operation parameters at order {}",
                            operation.order
                        )
                    })?;

                working_full = execute_delete(working_full, &params, &project.selectors)
                    .with_context(|| {
                        format!(
                            "failed to execute delete operation at order {}",
                            operation.order
                        )
                    })?;
            }
            OperationKind::Output => {
                let params: OutputOperationParams = serde_json::from_value(operation.parameters)
                    .with_context(|| {
                        format!(
                            "failed to deserialize output operation parameters at order {}",
                            operation.order
                        )
                    })?;

                let output = execute_output(working_full.clone(), &params, &project.selectors)
                    .with_context(|| {
                        format!(
                            "failed to execute output operation at order {}",
                            operation.order
                        )
                    })?;
                let output_frame = output.collect().with_context(|| {
                    format!(
                        "failed to materialize output operation at order {}",
                        operation.order
                    )
                })?;
                output_writer
                    .write(&output_frame, &params.destination)
                    .with_context(|| {
                        format!(
                            "failed to write output operation at order {}",
                            operation.order
                        )
                    })?;
                last_output = Some(output_frame);
            }
            OperationKind::Update | OperationKind::Aggregate | OperationKind::Append => {
                // Pipeline math operations are not implemented yet; we only enforce visibility
                // for non-output terminal results while preserving full rows for output ops.
                should_return_active_only = true;
            }
        }
    }

    if let Some(output) = last_output {
        return Ok(output);
    }

    let final_frame = if should_return_active_only {
        filter_active_rows(working_full)
    } else {
        working_full
    };
    final_frame.collect().map_err(Into::into)
}

fn filter_active_rows(frame: LazyFrame) -> LazyFrame {
    frame.filter(col("_deleted").eq(lit(false)))
}

fn collect_available_columns(input: &DataFrame) -> BTreeSet<String> {
    input
        .get_column_names()
        .iter()
        .map(|name| name.as_str().to_string())
        .collect()
}

fn validate_operation_selectors(
    operations: &[OperationInstance],
    selectors: &BTreeMap<String, String>,
    available_columns: &BTreeSet<String>,
) -> Result<()> {
    for operation in operations {
        match operation.kind {
            OperationKind::Delete => {
                let params: DeleteOperationParams =
                    serde_json::from_value(operation.parameters.clone()).with_context(|| {
                        format!(
                            "failed to deserialize delete operation parameters at order {}",
                            operation.order
                        )
                    })?;
                if let Some(selector) = params.selector.as_deref() {
                    validate_named_selector_reference(selector, selectors).with_context(|| {
                        format!(
                            "delete selector reference validation failed at order {}",
                            operation.order
                        )
                    })?;
                    validate_selector_boolean_type_with_schema(
                        selector,
                        selectors,
                        available_columns,
                    )
                    .with_context(|| {
                        format!(
                            "delete selector validation failed at order {}",
                            operation.order
                        )
                    })?;
                }
            }
            OperationKind::Output => {
                let params: OutputOperationParams =
                    serde_json::from_value(operation.parameters.clone()).with_context(|| {
                        format!(
                            "failed to deserialize output operation parameters at order {}",
                            operation.order
                        )
                    })?;
                if let Some(selector) = params.selector.as_deref() {
                    validate_named_selector_reference(selector, selectors).with_context(|| {
                        format!(
                            "output selector reference validation failed at order {}",
                            operation.order
                        )
                    })?;
                    validate_selector_boolean_type_with_schema(
                        selector,
                        selectors,
                        available_columns,
                    )
                    .with_context(|| {
                        format!(
                            "output selector validation failed at order {}",
                            operation.order
                        )
                    })?;
                }
            }
            OperationKind::Update | OperationKind::Aggregate | OperationKind::Append => {}
        }
    }

    Ok(())
}

struct NoopOutputWriter;

impl OutputWriter for NoopOutputWriter {
    fn write(
        &self,
        _frame: &DataFrame,
        _destination: &crate::model::OutputDestination,
    ) -> std::result::Result<(), OutputWriterError> {
        Ok(())
    }
}
