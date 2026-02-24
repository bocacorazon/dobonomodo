use std::collections::BTreeMap;

use polars::prelude::LazyFrame;
use uuid::Uuid;

use crate::engine::io_traits::DataLoader;
use crate::engine::join::{apply_update_operation_runtime_joins, JoinError};
use crate::model::{Dataset, OperationInstance, Period, ResolvedLocation, Run};
use crate::MetadataStore;

#[allow(clippy::too_many_arguments)]
pub fn execute_update_operation<M, R, L>(
    run: &mut Run,
    operation: &OperationInstance,
    working_lf: LazyFrame,
    working_table_name: &str,
    working_columns: &[String],
    project_overrides: &BTreeMap<Uuid, String>,
    system_default_resolver: &str,
    period: &Period,
    metadata_store: &M,
    resolve_location: R,
    loader: &L,
) -> Result<LazyFrame, JoinError>
where
    M: MetadataStore,
    R: Fn(&Dataset, &str, &Period) -> Result<ResolvedLocation, String>,
    L: DataLoader,
{
    apply_update_operation_runtime_joins(
        run,
        operation,
        working_lf,
        working_table_name,
        working_columns,
        project_overrides,
        system_default_resolver,
        period,
        metadata_store,
        resolve_location,
        loader,
    )
}
