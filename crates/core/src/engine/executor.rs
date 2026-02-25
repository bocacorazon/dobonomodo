use polars::prelude::DataFrame;

use crate::engine::append::{execute_append_operation, AppendExecutionContext, AppendResult};
use crate::engine::error::AppendError;
use crate::engine::io_traits::DataLoader;
use crate::model::{OperationInstance, OperationKind, Project};
use crate::MetadataStore;

pub fn execute_operation<M: MetadataStore, D: DataLoader>(
    working_frame: &DataFrame,
    metadata_store: &M,
    data_loader: &D,
    project: &Project,
    operation_instance: &OperationInstance,
    context: &AppendExecutionContext,
) -> Result<AppendResult, AppendError> {
    match operation_instance.kind {
        OperationKind::Append => {
            let operation = operation_instance.append_parameters().map_err(|error| {
                AppendError::ExpressionParseError {
                    expression: "append.parameters".to_owned(),
                    error: error.to_string(),
                }
            })?;
            execute_append_operation(
                working_frame,
                metadata_store,
                data_loader,
                project,
                &operation,
                context,
            )
        }
        _ => Err(AppendError::DataLoadError {
            message: "executor currently supports append operations only".to_owned(),
        }),
    }
}
