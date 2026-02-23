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

#[cfg(test)]
mod tests {
    use super::*;

    fn event(order: u32, message: &str) -> TraceEvent {
        TraceEvent {
            operation_order: order,
            message: message.to_string(),
        }
    }

    #[test]
    fn write_events_appends_in_order() {
        let writer = InMemoryTraceWriter::new();
        let run_id = Uuid::now_v7();

        writer.write_events(&run_id, &[event(1, "one")]).unwrap();
        writer
            .write_events(&run_id, &[event(2, "two"), event(3, "three")])
            .unwrap();

        let events = writer.get_events();
        assert_eq!(events.len(), 3);
        assert_eq!(events[0].operation_order, 1);
        assert_eq!(events[1].operation_order, 2);
        assert_eq!(events[2].operation_order, 3);
    }

    #[test]
    fn clear_removes_all_events() {
        let writer = InMemoryTraceWriter::new();
        let run_id = Uuid::now_v7();
        writer.write_events(&run_id, &[event(1, "one")]).unwrap();

        writer.clear();
        assert!(writer.get_events().is_empty());
    }

    #[test]
    fn write_events_reports_lock_poisoning() {
        let writer = InMemoryTraceWriter::new();
        let poisoned_writer = writer.clone();
        let _ = std::thread::spawn(move || {
            let _guard = poisoned_writer.events.lock().unwrap();
            panic!("poison trace mutex");
        })
        .join();

        let run_id = Uuid::now_v7();
        let error = writer
            .write_events(&run_id, &[event(1, "one")])
            .unwrap_err()
            .to_string();

        assert!(error.contains("Failed to lock trace events mutex"));
    }
}
