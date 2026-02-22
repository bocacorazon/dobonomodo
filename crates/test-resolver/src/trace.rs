use crate::errors::TraceError;
use anyhow::Result;
use dobo_core::trace::trace_writer::TraceWriter;
use dobo_core::trace::types::TraceEvent;
use std::sync::{Arc, Mutex};
use uuid::Uuid;

/// In-memory trace writer for test scenarios
#[derive(Clone)]
pub struct InMemoryTraceWriter {
    events: Arc<Mutex<Vec<TraceEvent>>>,
}

impl InMemoryTraceWriter {
    /// Create a new in-memory trace writer
    pub fn new() -> Self {
        Self {
            events: Arc::new(Mutex::new(Vec::new())),
        }
    }

    /// Get all collected trace events
    pub fn get_events(&self) -> Vec<TraceEvent> {
        match self.events.lock() {
            Ok(events) => events.clone(),
            Err(poisoned) => poisoned.into_inner().clone(),
        }
    }

    /// Clear all collected trace events
    pub fn clear(&self) {
        match self.events.lock() {
            Ok(mut events) => events.clear(),
            Err(poisoned) => poisoned.into_inner().clear(),
        }
    }

    fn append_events(&self, events: &[TraceEvent]) -> std::result::Result<(), TraceError> {
        self.events
            .lock()
            .map_err(|error| TraceError::LockPoisoned {
                message: error.to_string(),
            })?
            .extend_from_slice(events);
        Ok(())
    }
}

impl Default for InMemoryTraceWriter {
    fn default() -> Self {
        Self::new()
    }
}

impl TraceWriter for InMemoryTraceWriter {
    fn write_events(&self, _run_id: &Uuid, events: &[TraceEvent]) -> Result<()> {
        self.append_events(events).map_err(Into::into)
    }
}
