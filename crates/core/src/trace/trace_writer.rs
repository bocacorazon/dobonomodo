use uuid::Uuid;

use crate::trace::types::TraceEvent;
use crate::Result;

pub trait TraceWriter {
    fn write_events(&self, run_id: &Uuid, events: &[TraceEvent]) -> Result<()>;
}
