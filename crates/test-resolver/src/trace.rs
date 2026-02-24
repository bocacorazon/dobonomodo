use crate::errors::TraceError;
use dobo_core::trace::trace_writer::{TraceWriteError, TraceWriter};
use dobo_core::trace::types::TraceEvent;
use std::collections::HashMap;
use std::sync::{Arc, Mutex};
use uuid::Uuid;

/// In-memory trace writer for test scenarios
#[derive(Clone)]
pub struct InMemoryTraceWriter {
    events: Arc<Mutex<HashMap<Uuid, Vec<TraceEvent>>>>,
}

impl InMemoryTraceWriter {
    /// Create a new in-memory trace writer
    pub fn new() -> Self {
        Self {
            events: Arc::new(Mutex::new(HashMap::new())),
        }
    }

    /// Get collected trace events for a specific run.
    pub fn get_events_for_run(&self, run_id: &Uuid) -> Vec<TraceEvent> {
        match self.events.lock() {
            Ok(events) => events.get(run_id).cloned().unwrap_or_default(),
            Err(poisoned) => poisoned
                .into_inner()
                .get(run_id)
                .cloned()
                .unwrap_or_default(),
        }
    }

    /// Clear all collected trace events
    pub fn clear(&self) {
        match self.events.lock() {
            Ok(mut events) => events.clear(),
            Err(poisoned) => poisoned.into_inner().clear(),
        }
    }

    fn append_events(
        &self,
        run_id: &Uuid,
        events: &[TraceEvent],
    ) -> std::result::Result<(), TraceError> {
        self.events
            .lock()
            .map_err(|error| TraceError::LockPoisoned {
                message: error.to_string(),
            })?
            .entry(*run_id)
            .or_default()
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
    fn write_events(
        &self,
        run_id: &Uuid,
        events: &[TraceEvent],
    ) -> std::result::Result<(), TraceWriteError> {
        self.append_events(run_id, events)
            .map_err(|error| TraceWriteError::WriteFailed {
                message: error.to_string(),
            })
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

        let events = writer.get_events_for_run(&run_id);
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
        assert!(writer.get_events_for_run(&run_id).is_empty());
    }

    #[test]
    fn write_events_keeps_runs_isolated() {
        let writer = InMemoryTraceWriter::new();
        let run_a = Uuid::now_v7();
        let run_b = Uuid::now_v7();

        writer.write_events(&run_a, &[event(1, "a")]).unwrap();
        writer.write_events(&run_b, &[event(1, "b")]).unwrap();

        let events_a = writer.get_events_for_run(&run_a);
        let events_b = writer.get_events_for_run(&run_b);
        assert_eq!(events_a.len(), 1);
        assert_eq!(events_b.len(), 1);
        assert_eq!(events_a[0].message, "a");
        assert_eq!(events_b[0].message, "b");
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
