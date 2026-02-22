use serde::{Deserialize, Serialize};

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
pub struct TraceEvent {
    pub operation_order: u32,
    pub message: String,
}
