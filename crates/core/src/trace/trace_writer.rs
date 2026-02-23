use thiserror::Error;
use uuid::Uuid;

use crate::trace::types::TraceEvent;
use crate::Result;

#[derive(Debug, Error, Clone, PartialEq, Eq)]
pub enum TraceWriteError {
    #[error("trace write failed: {message}")]
    WriteFailed { message: String },
}

pub trait TraceWriter {
    fn write_events(&self, run_id: &Uuid, events: &[TraceEvent]) -> Result<(), TraceWriteError>;
}
