use dobo_core::{DataLoader, MetadataStore, OutputWriter, TraceWriter};

#[test]
fn io_traits_are_publicly_importable() {
    fn assert_loader<T: DataLoader>() {}
    fn assert_output<T: OutputWriter>() {}
    fn assert_metadata<T: MetadataStore>() {}
    fn assert_trace<T: TraceWriter>() {}

    let _ = assert_loader::<NoopLoader>;
    let _ = assert_output::<NoopOutputWriter>;
    let _ = assert_metadata::<NoopMetadataStore>;
    let _ = assert_trace::<NoopTraceWriter>;
}

struct NoopLoader;
struct NoopOutputWriter;
struct NoopMetadataStore;
struct NoopTraceWriter;

impl DataLoader for NoopLoader {
    fn load(
        &self,
        _location: &dobo_core::model::ResolvedLocation,
        _schema: &dobo_core::model::TableRef,
    ) -> anyhow::Result<polars::prelude::LazyFrame> {
        anyhow::bail!("not implemented")
    }
}

impl OutputWriter for NoopOutputWriter {
    fn write(
        &self,
        _frame: &polars::prelude::DataFrame,
        _destination: &dobo_core::model::OutputDestination,
    ) -> anyhow::Result<()> {
        anyhow::bail!("not implemented")
    }
}

impl MetadataStore for NoopMetadataStore {
    fn get_dataset(&self, _id: &uuid::Uuid, _version: Option<i32>) -> anyhow::Result<dobo_core::model::Dataset> {
        anyhow::bail!("not implemented")
    }

    fn get_project(&self, _id: &uuid::Uuid) -> anyhow::Result<dobo_core::model::Project> {
        anyhow::bail!("not implemented")
    }

    fn get_resolver(&self, _id: &str) -> anyhow::Result<dobo_core::model::Resolver> {
        anyhow::bail!("not implemented")
    }

    fn update_run_status(&self, _id: &uuid::Uuid, _status: dobo_core::model::RunStatus) -> anyhow::Result<()> {
        anyhow::bail!("not implemented")
    }
}

impl TraceWriter for NoopTraceWriter {
    fn write_events(&self, _run_id: &uuid::Uuid, _events: &[dobo_core::trace::types::TraceEvent]) -> anyhow::Result<()> {
        anyhow::bail!("not implemented")
    }
}
