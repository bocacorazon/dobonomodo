// Operation implementations for the engine

pub mod output;

// Re-export main types for convenience
pub use output::{
    execute_output, extract_schema, ColumnDef, ColumnType, OutputError, OutputOperation,
    OutputResult, OutputSchema, TemporalMode,
};
