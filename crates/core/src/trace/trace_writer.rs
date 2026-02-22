use anyhow::Result;
use uuid::Uuid;

use crate::trace::types::TraceEvent;

pub trait TraceWriter {
    fn write_events(&self, run_id: &Uuid, events: &[TraceEvent]) -> Result<()>;
}
